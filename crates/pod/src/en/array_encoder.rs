use crate::error::ErrorKind;
use crate::pod::{ChildPod, PodKind};
use crate::{Error, Pod, Type, WORD_SIZE, Writer};

/// An encoder for an array.
///
/// # Examples
///
/// ```
/// use pod::{Pod, Type};
///
/// let mut pod = Pod::array();
/// let mut array = pod.as_mut().encode_array(Type::INT)?;
/// array.push()?.encode(1i32)?;
/// array.push()?.encode(2i32)?;
/// array.push()?.encode(3i32)?;
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
pub struct ArrayEncoder<W, K>
where
    W: Writer,
{
    writer: W,
    kind: K,
    header: W::Pos,
    child_size: u32,
    child_type: Type,
}

impl<W, K> ArrayEncoder<W, K>
where
    W: Writer,
    K: PodKind,
{
    #[inline]
    pub(crate) fn to_writer(mut writer: W, kind: K, child_type: Type) -> Result<Self, Error> {
        let Some(child_size) = child_type.size() else {
            return Err(Error::new(ErrorKind::UnsizedTypeInArray { ty: child_type }));
        };

        let header = writer.reserve([
            WORD_SIZE,
            Type::ARRAY.into_u32(),
            child_size,
            child_type.into_u32(),
        ])?;

        Ok(Self {
            writer,
            kind,
            header,
            child_size,
            child_type,
        })
    }

    #[inline]
    pub(crate) fn to_writer_unsized(
        mut writer: W,
        kind: K,
        child_size: u32,
        child_type: Type,
    ) -> Result<Self, Error> {
        if let Some(size) = child_type.size() {
            if size != child_size {
                return Err(Error::new(ErrorKind::ChildSizeMismatch {
                    actual: child_size,
                    expected: size,
                }));
            }
        };

        let pos = writer.reserve([
            WORD_SIZE,
            Type::ARRAY.into_u32(),
            child_size,
            child_type.into_u32(),
        ])?;

        Ok(Self {
            writer,
            kind,
            header: pos,
            child_size,
            child_type,
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
    /// let mut seq = pod.as_mut().encode_array(Type::INT)?;
    /// seq.push()?.encode(1i32)?;
    /// seq.close()?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn push(&mut self) -> Result<Pod<W::Mut<'_>, ChildPod>, Error> {
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
    #[inline]
    pub fn close(mut self) -> Result<(), Error> {
        let Some(size) = self
            .writer
            .distance_from(self.header)
            .and_then(|v| v.checked_sub(WORD_SIZE))
        else {
            return Err(Error::new(ErrorKind::SizeOverflow));
        };

        self.kind.check(Type::ARRAY, size)?;
        self.writer
            .write_at(self.header, [size, Type::ARRAY.into_u32()])?;
        Ok(())
    }
}
