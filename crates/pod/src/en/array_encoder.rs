use crate::error::ErrorKind;
use crate::pod::ChildLimit;
use crate::{Error, Pod, Type, WORD_SIZE, Writer};

/// An encoder for an array.
///
/// # Examples
///
/// ```
/// use pod::{Pod, Type};
///
/// let mut pod = Pod::array();
/// let mut array = pod.encode_array(Type::INT)?;
///
/// array.push()?.encode(1i32)?;
/// array.push()?.encode(2i32)?;
/// array.push()?.encode(3i32)?;
///
/// array.close()?;
/// # Ok::<_, pod::Error>(())
/// ```
///
/// Encoding an array of unsized values:
///
/// ```
/// use pod::{Pod, Type};
///
/// let mut pod = Pod::array();
/// let mut array = pod.encode_unsized_array(Type::STRING, 4)?;
///
/// array.push()?.encode_unsized("foo")?;
/// array.push()?.encode_unsized("bar")?;
/// array.push()?.encode_unsized("baz")?;
///
/// array.close()?;
/// # Ok::<_, pod::Error>(())
/// ```
#[must_use = "Array encoders must be closed to ensure all elements are encoded"]
pub struct ArrayEncoder<W>
where
    W: Writer,
{
    writer: W,
    child_size: u32,
    child_type: Type,
    pos: W::Pos,
}

impl<W> ArrayEncoder<W>
where
    W: Writer,
{
    #[inline]
    pub(crate) fn to_writer(mut writer: W, child_type: Type) -> Result<Self, Error> {
        let Some(child_size) = child_type.size() else {
            return Err(Error::new(ErrorKind::UnsizedTypeInArray { ty: child_type }));
        };

        let pos = writer.reserve_words(&[0, 0])?;

        Ok(Self {
            writer,
            child_size,
            child_type,
            pos,
        })
    }

    #[inline]
    pub(crate) fn to_writer_unsized(
        mut writer: W,
        len: u32,
        child_type: Type,
    ) -> Result<Self, Error> {
        if let Some(child_size) = child_type.size() {
            if child_size != len {
                return Err(Error::new(ErrorKind::ChildSizeMismatch {
                    actual: len,
                    expected: child_size,
                }));
            }
        };

        let pos = writer.reserve_words(&[0, 0])?;

        Ok(Self {
            writer,
            child_size: len,
            child_type,
            pos,
        })
    }

    /// Write control into the choice.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type, Choice};
    ///
    /// let mut pod = Pod::array();
    /// let mut seq = pod.encode_array(Type::INT)?;
    /// seq.push()?.encode(1i32)?;
    /// seq.close()?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn push(&mut self) -> Result<Pod<W::Mut<'_>, ChildLimit>, Error> {
        Ok(Pod::new_child(
            self.writer.borrow_mut(),
            self.child_size,
            self.child_type,
        ))
    }

    /// Close the array encoder, ensuring all elements have been encoded.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = Pod::array();
    /// let mut array = pod.encode_unsized_array(Type::STRING, 4)?;
    ///
    /// array.push()?.encode_unsized("foo")?;
    /// array.push()?.encode_unsized("bar")?;
    /// array.push()?.encode_unsized("baz")?;
    ///
    /// array.close()?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    pub fn close(mut self) -> Result<(), Error> {
        let Some(len) = self
            .writer
            .distance_from(self.pos)
            .and_then(|v| v.checked_sub(WORD_SIZE))
        else {
            return Err(Error::new(ErrorKind::SizeOverflow));
        };

        self.writer.write_at(
            self.pos,
            [
                len,
                Type::ARRAY.into_u32(),
                self.child_size,
                self.child_type.into_u32(),
            ],
        )?;

        Ok(())
    }
}
