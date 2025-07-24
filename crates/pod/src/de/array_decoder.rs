use crate::error::ErrorKind;
use crate::{Decode, DecodeUnsized, Error, Reader, Type, TypedPod, Visitor, WORD_SIZE};

/// A decoder for an array.
pub struct ArrayDecoder<R> {
    reader: R,
    child_size: u32,
    child_type: Type,
    remaining: u32,
}

impl<'de, R> ArrayDecoder<R>
where
    R: Reader<'de>,
{
    pub(crate) fn from_reader(mut reader: R, size: u32) -> Result<Self, Error> {
        let (child_size, child_type) = reader.header()?;

        let Some(size) = size.checked_sub(WORD_SIZE) else {
            return Err(Error::new(ErrorKind::SizeOverflow));
        };

        let remaining = 'out: {
            if size == 0 {
                break 'out 0;
            }

            let padded_child_size = child_size.next_multiple_of(WORD_SIZE);

            let Some(size) = size.checked_div(padded_child_size) else {
                break 'out 0;
            };

            size
        };

        Ok(Self {
            reader,
            child_size,
            child_type,
            remaining,
        })
    }

    /// Return the type of the child element.
    pub fn child_type(&self) -> Type {
        self.child_type
    }

    /// Get the number of elements left to decode from the array.
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
    /// array.close()?;
    ///
    /// let pod = Pod::new(buf.as_slice());
    /// let mut array = pod.decode_array()?;
    ///
    /// assert_eq!(array.len(), 1);
    /// assert!(!array.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    pub fn len(&self) -> u32 {
        self.remaining
    }

    /// Check if the array is empty.
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
    /// array.close()?;
    ///
    /// let pod = Pod::new(buf.as_slice());
    /// let mut array = pod.decode_array()?;
    ///
    /// assert!(array.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    pub fn is_empty(&self) -> bool {
        self.remaining == 0
    }

    /// Get the next element in the array.
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
    ///
    /// let pod = Pod::new(buf.as_slice());
    /// let mut array = pod.decode_array()?;
    ///
    /// let mut count = 0;
    ///
    /// while !array.is_empty() {
    ///     let pod = array.item()?;
    ///     assert_eq!(pod.ty(), Type::INT);
    ///     assert_eq!(pod.size(), 4);
    ///     count += 1;
    /// }
    ///
    /// assert_eq!(count, 3);
    /// # Ok::<_, pod::Error>(())
    /// ```
    pub fn item(&mut self) -> Result<TypedPod<R::Clone<'_>>, Error> {
        if self.remaining == 0 {
            return Err(Error::new(ErrorKind::ArrayUnderflow));
        }

        let tail = self.reader.split(self.child_size)?;

        let pod = TypedPod::new(self.child_size, self.child_type, tail);
        self.remaining -= 1;
        Ok(pod)
    }

    /// Decode an element in the array.
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
    ///
    /// let pod = Pod::new(buf.as_slice());
    /// let mut array = pod.decode_array()?;
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
    ///
    /// let pod = Pod::new(buf.as_slice());
    /// let mut array = pod.decode_array()?;
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
    ///
    /// let pod = Pod::new(buf.as_slice());
    /// let mut array = pod.decode_array()?;
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
