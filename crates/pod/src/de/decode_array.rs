use crate::error::ErrorKind;
use crate::{Decode, DecodeUnsized, Error, Reader, Type, Visitor};

/// A decoder for arrays.
pub struct DecodeArray<R> {
    reader: R,
    child_type: Type,
    child_size: usize,
    remaining: usize,
}

impl<'de, R> DecodeArray<R>
where
    R: Reader<'de>,
{
    #[inline]
    pub(crate) fn new(reader: R, child_type: Type, child_size: usize, remaining: usize) -> Self {
        Self {
            reader,
            child_type,
            child_size,
            remaining,
        }
    }

    /// Get the number of elements left to decode from the array.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Decoder, Encoder, Type};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let mut encoder = Encoder::new(&mut buf);
    /// let mut array = encoder.encode_array(Type::INT)?;
    ///
    /// array.encode(1i32)?;
    /// array.close()?;
    ///
    /// let mut decoder = Decoder::new(buf.as_reader_slice());
    /// let mut array = decoder.decode_array()?;
    ///
    /// assert_eq!(array.len(), 1);
    /// assert!(!array.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    pub fn len(&self) -> usize {
        self.remaining
    }

    /// Check if the array is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Decoder, Encoder, Type};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let mut encoder = Encoder::new(&mut buf);
    /// let mut array = encoder.encode_array(Type::INT)?;
    ///
    /// array.close()?;
    ///
    /// let mut decoder = Decoder::new(buf.as_reader_slice());
    /// let mut array = decoder.decode_array()?;
    ///
    /// assert!(array.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    pub fn is_empty(&self) -> bool {
        self.remaining == 0
    }

    /// Decode an element in the array.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Decoder, Encoder, Type};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let mut encoder = Encoder::new(&mut buf);
    /// let mut array = encoder.encode_array(Type::INT)?;
    ///
    /// array.encode(1i32)?;
    /// array.encode(2i32)?;
    /// array.encode(3i32)?;
    ///
    /// array.close()?;
    ///
    /// let mut decoder = Decoder::new(buf.as_reader_slice());
    /// let mut array = decoder.decode_array()?;
    ///
    /// assert!(!array.is_empty());
    /// assert_eq!(array.len(), 3);
    ///
    /// assert_eq!(array.decode::<i32>()?, 1i32);
    /// assert_eq!(array.decode::<i32>()?, 2i32);
    /// assert_eq!(array.decode::<i32>()?, 3i32);
    ///
    /// assert!(array.is_empty());
    /// assert_eq!(array.len(), 0);
    /// # Ok::<_, pod::Error>(())
    /// ```
    pub fn decode<T>(&mut self) -> Result<T, Error>
    where
        T: Decode<'de>,
    {
        if self.remaining == 0 {
            return Err(Error::new(ErrorKind::ArrayUnderflow));
        }

        if self.child_type != T::TYPE {
            return Err(Error::new(ErrorKind::ArrayTypeMismatch {
                expected: self.child_type,
                actual: T::TYPE,
            }));
        }

        self.remaining -= 1;
        let ok = T::read_content(self.reader.borrow_mut(), self.child_size)?;
        Ok(ok)
    }

    /// Decode an unsized element from the array.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Decoder, Encoder, Type};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let mut encoder = Encoder::new(&mut buf);
    /// let mut array = encoder.encode_unsized_array(Type::STRING, 4)?;
    ///
    /// array.encode_unsized("foo")?;
    /// array.encode_unsized("bar")?;
    /// array.encode_unsized("baz")?;
    ///
    /// array.close()?;
    ///
    /// let mut decoder = Decoder::new(buf.as_reader_slice());
    /// let mut array = decoder.decode_array()?;
    ///
    /// assert!(!array.is_empty());
    /// assert_eq!(array.len(), 3);
    ///
    /// assert_eq!(array.decode_borrowed::<str>()?, "foo");
    /// assert_eq!(array.decode_borrowed::<str>()?, "bar");
    /// assert_eq!(array.decode_borrowed::<str>()?, "baz");
    ///
    /// assert!(array.is_empty());
    /// assert_eq!(array.len(), 0);
    /// # Ok::<_, pod::Error>(())
    /// ```
    pub fn decode_unsized<V, T>(&mut self, visitor: V) -> Result<V::Ok, Error>
    where
        V: Visitor<'de, T>,
        T: ?Sized + DecodeUnsized<'de>,
    {
        if self.remaining == 0 {
            return Err(Error::new(ErrorKind::ArrayUnderflow));
        }

        if self.child_type != T::TYPE {
            return Err(Error::new(ErrorKind::ArrayTypeMismatch {
                expected: self.child_type,
                actual: T::TYPE,
            }));
        }

        let ok = T::read_content(self.reader.borrow_mut(), visitor, self.child_size)?;
        self.remaining -= 1;
        Ok(ok)
    }

    /// Decode an unsized element from the array.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Decoder, Encoder, Type};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let mut encoder = Encoder::new(&mut buf);
    /// let mut array = encoder.encode_unsized_array(Type::STRING, 4)?;
    ///
    /// array.encode_unsized("foo")?;
    /// array.encode_unsized("bar")?;
    /// array.encode_unsized("baz")?;
    ///
    /// array.close()?;
    ///
    /// let mut decoder = Decoder::new(buf.as_reader_slice());
    /// let mut array = decoder.decode_array()?;
    ///
    /// assert!(!array.is_empty());
    /// assert_eq!(array.len(), 3);
    ///
    /// assert_eq!(array.decode_borrowed::<str>()?, "foo");
    /// assert_eq!(array.decode_borrowed::<str>()?, "bar");
    /// assert_eq!(array.decode_borrowed::<str>()?, "baz");
    ///
    /// assert!(array.is_empty());
    /// assert_eq!(array.len(), 0);
    /// # Ok::<_, pod::Error>(())
    /// ```
    pub fn decode_borrowed<T>(&mut self) -> Result<&'de T, Error>
    where
        T: ?Sized + DecodeUnsized<'de>,
    {
        struct LocalVisitor;

        impl<'de, T> Visitor<'de, T> for LocalVisitor
        where
            T: 'de + ?Sized,
        {
            type Ok = &'de T;

            #[inline]
            fn visit_borrowed(self, value: &'de T) -> Result<Self::Ok, Error> {
                Ok(value)
            }
        }

        self.decode_unsized(LocalVisitor)
    }
}
