use crate::error::ErrorKind;
use crate::{Encode, EncodeUnsized, Error, Type, WORD_SIZE, Writer};

/// An encoder for an array.
///
/// # Examples
///
/// ```
/// use pod::{ArrayBuf, Pod, Type};
///
/// let mut buf = ArrayBuf::new();
/// let pod = Pod::new(&mut buf);
/// let mut array = pod.encode_array(Type::INT)?;
///
/// array.encode(1i32)?;
/// array.encode(2i32)?;
/// array.encode(3i32)?;
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
    child_size: usize,
    child_type: Type,
    pos: W::Pos,
    len: usize,
}

impl<W> ArrayEncoder<W>
where
    W: Writer,
{
    /// Create a new pod for an array with the given writer and length.
    #[inline]
    pub(crate) fn new(writer: W, child_size: usize, child_type: Type, pos: W::Pos) -> Self {
        ArrayEncoder {
            writer,
            child_size,
            child_type,
            pos,
            len: 0,
        }
    }

    /// Encode a value into the array.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Pod, Type};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let pod = Pod::new(&mut buf);
    /// let mut array = pod.encode_array(Type::INT)?;
    ///
    /// array.encode(1i32)?;
    /// array.encode(2i32)?;
    /// array.encode(3i32)?;
    ///
    /// array.close()?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    pub fn encode<T>(&mut self, value: T) -> Result<(), Error>
    where
        T: Encode,
    {
        if self.child_type != T::TYPE {
            return Err(Error::new(ErrorKind::Expected {
                expected: self.child_type,
                actual: T::TYPE,
            }));
        }

        let actual = value.size();

        if actual > self.child_size {
            return Err(Error::new(ErrorKind::ArrayChildSizeMismatch {
                expected: self.child_size,
                actual,
            }));
        }

        value.write_content(self.writer.borrow_mut())?;
        self.len += 1;
        Ok(())
    }

    /// Encode an unsized value into the array.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Pod, Type};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let pod = Pod::new(&mut buf);
    /// let mut array = pod.encode_unsized_array(Type::STRING, 4)?;
    ///
    /// array.encode_unsized("foo")?;
    /// array.encode_unsized("bar")?;
    /// array.encode_unsized("baz")?;
    ///
    /// array.close()?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    pub fn encode_unsized<T>(&mut self, value: &T) -> Result<(), Error>
    where
        T: ?Sized + EncodeUnsized,
    {
        if self.child_type != T::TYPE {
            return Err(Error::new(ErrorKind::Expected {
                expected: self.child_type,
                actual: T::TYPE,
            }));
        }

        let actual = value.size();

        if actual != self.child_size {
            return Err(Error::new(ErrorKind::ArrayChildSizeMismatch {
                expected: self.child_size,
                actual,
            }));
        }

        value.write_content(self.writer.borrow_mut())?;
        self.len += 1;
        Ok(())
    }

    /// Close the array encoder, ensuring all elements have been encoded.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Pod, Type};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let pod = Pod::new(&mut buf);
    /// let mut array = pod.encode_unsized_array(Type::STRING, 4)?;
    ///
    /// array.encode_unsized("foo")?;
    /// array.encode_unsized("bar")?;
    /// array.encode_unsized("baz")?;
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

        let Ok(child_size) = u32::try_from(self.child_size) else {
            return Err(Error::new(ErrorKind::SizeOverflow));
        };

        self.writer.write_at(
            self.pos,
            [
                len,
                Type::ARRAY.into_u32(),
                child_size,
                self.child_type.into_u32(),
            ],
        )?;

        Ok(())
    }
}
