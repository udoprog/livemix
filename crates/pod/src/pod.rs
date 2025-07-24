use crate::error::ErrorKind;
use crate::id::IntoId;
use crate::{
    Decode, DecodeUnsized, Encode, EncodeUnsized, Error, Id, Reader, Type, Visitor, WORD_SIZE,
    Writer,
};

use crate::de::DecodeArray;
use crate::en::EncodeArray;

/// A POD (Plain Old Data) handler.
///
/// This is a wrapper that can be used for encoding and decoding data.
pub struct Pod<B> {
    buf: B,
}

impl<B> Pod<B> {
    /// Construct a new [`Pod`] arround the specified buffer `B`.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Pod};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let mut pod = Pod::new(&mut buf);
    /// ```
    #[inline]
    pub const fn new(buf: B) -> Self {
        Pod { buf }
    }
}

impl<B> Pod<B>
where
    B: Writer,
{
    /// Encode a value into the pod.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Pod};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let mut pod = Pod::new(&mut buf);
    /// pod.encode(10i32)?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    pub fn encode<T>(&mut self, value: T) -> Result<(), Error>
    where
        T: Encode,
    {
        value.encode(self.buf.borrow_mut())
    }

    /// Encode an unsized value into the pod.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Pod};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let mut pod = Pod::new(&mut buf);
    /// pod.encode_unsized(&b"hello world"[..])?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    pub fn encode_unsized<T>(&mut self, value: &T) -> Result<(), Error>
    where
        T: ?Sized + EncodeUnsized,
    {
        value.encode_unsized(self.buf.borrow_mut())
    }

    /// Encode a `None` value.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Pod};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let mut pod = Pod::new(&mut buf);
    /// pod.encode_none()?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn encode_none(&mut self) -> Result<(), Error> {
        self.buf.write([0, Type::NONE.into_u32()])?;
        Ok(())
    }

    /// Encode an `id` value.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Pod};
    /// use pod::id::MediaSubType;
    ///
    /// let mut buf = ArrayBuf::new();
    /// let mut pod = Pod::new(&mut buf);
    /// pod.encode_id(MediaSubType::Opus)?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn encode_id(&mut self, value: impl IntoId) -> Result<(), Error> {
        Id(value).encode(self.buf.borrow_mut())
    }

    /// Encode an array with the given type.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Pod, Type};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let mut pod = Pod::new(&mut buf);
    /// let mut array = pod.encode_array(Type::INT)?;
    ///
    /// array.encode(1i32)?;
    /// array.encode(2i32)?;
    /// array.encode(3i32)?;
    ///
    /// array.close()?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn encode_array(&mut self, child_type: Type) -> Result<EncodeArray<B::Mut<'_>>, Error> {
        let Some(child_size) = child_type.size() else {
            return Err(Error::new(ErrorKind::UnsizedTypeInArray { ty: child_type }));
        };

        let mut writer = self.buf.borrow_mut();
        let pos = writer.reserve_words(&[0, 0])?;
        Ok(EncodeArray::new(writer, child_size, child_type, pos))
    }

    /// Encode an array with elements of an unsized type.
    ///
    /// The `len` specified will be used to determine the maximum size of
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Pod, Type};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let mut pod = Pod::new(&mut buf);
    /// let mut array = pod.encode_unsized_array(Type::STRING, 4)?;
    ///
    /// array.encode_unsized("foo")?;
    /// array.encode_unsized("bar")?;
    /// array.encode_unsized("baz")?;
    ///
    /// array.close()?;
    ///
    /// assert_eq!(buf.as_slice().len(), 5);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn encode_unsized_array(
        &mut self,
        child_type: Type,
        len: usize,
    ) -> Result<EncodeArray<B::Mut<'_>>, Error> {
        if let Some(child_size) = child_type.size() {
            if child_size != len {
                return Err(Error::new(ErrorKind::ArrayChildSizeMismatch {
                    actual: len,
                    expected: child_size,
                }));
            }
        };

        let mut writer = self.buf.borrow_mut();
        let pos = writer.reserve_words(&[0, 0])?;
        Ok(EncodeArray::new(writer, len, child_type, pos))
    }
}

impl<'de, B> Pod<B>
where
    B: Reader<'de>,
{
    /// Encode a value into the pod.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Pod};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let mut pod = Pod::new(&mut buf);
    /// pod.encode(10i32)?;
    /// pod.encode(&b"hello world"[..])?;
    ///
    /// let mut pod = Pod::new(buf.as_slice());
    /// let value: i32 = pod.decode()?;
    /// assert_eq!(value, 10i32);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn decode<T>(&mut self) -> Result<T, Error>
    where
        T: Decode<'de>,
    {
        T::decode(self.buf.borrow_mut())
    }

    /// Decode an unsized value into the pod.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Pod};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let mut pod = Pod::new(&mut buf);
    /// pod.encode_unsized(&b"hello world"[..])?;
    ///
    /// let mut pod = Pod::new(buf.as_slice());
    /// assert_eq!(pod.decode_borrowed::<[u8]>()?, b"hello world");
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn decode_unsized<T, V>(&mut self, visitor: V) -> Result<V::Ok, Error>
    where
        T: ?Sized + DecodeUnsized<'de>,
        V: Visitor<'de, T>,
    {
        T::decode_unsized(self.buf.borrow_mut(), visitor)
    }

    /// Decode an unsized value into the pod.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Pod};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let mut pod = Pod::new(&mut buf);
    ///
    /// pod.encode_unsized(&b"hello world"[..])?;
    ///
    /// let mut pod = Pod::new(buf.as_slice());
    /// assert_eq!(pod.decode_borrowed::<[u8]>()?, b"hello world");
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn decode_borrowed<T>(&mut self) -> Result<&'de T, Error>
    where
        T: ?Sized + DecodeUnsized<'de>,
    {
        T::decode_borrowed(self.buf.borrow_mut())
    }

    /// Decode an optional value.
    ///
    /// This returns `None` if the encoded value is `None`, otherwise a pod
    /// for the value is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Pod};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let mut pod = Pod::new(&mut buf);
    ///
    /// pod.encode_none()?;
    /// let mut pod = Pod::new(buf.as_slice());
    /// assert!(pod.decode_option()?.is_none());
    ///
    /// buf.clear();
    ///
    /// let mut pod = Pod::new(&mut buf);
    /// pod.encode(true)?;
    /// let mut pod = Pod::new(buf.as_slice());
    ///
    /// let Some(mut pod) = pod.decode_option()? else {
    ///     panic!("expected some value");
    /// };
    ///
    /// assert!(pod.decode::<bool>()?);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn decode_option(&mut self) -> Result<Option<Pod<B::Mut<'_>>>, Error> {
        // SAFETY: The slice must have been initialized by the reader.
        let [_, ty] = self.buf.peek::<[u32; 2]>()?;
        let ty = Type::new(ty);

        match ty {
            Type::NONE => {
                _ = self.buf.read::<[u32; 2]>()?;
                Ok(None)
            }
            _ => Ok(Some(Pod::new(self.buf.borrow_mut()))),
        }
    }

    /// Decode an id value.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Pod};
    /// use pod::id::MediaSubType;
    ///
    /// let mut buf = ArrayBuf::new();
    /// let mut pod = Pod::new(&mut buf);
    ///
    /// pod.encode_id(MediaSubType::Opus)?;
    /// let mut pod = Pod::new(buf.as_slice());
    /// let sub_type: MediaSubType = pod.decode_id()?;
    /// assert_eq!(sub_type, MediaSubType::Opus);
    ///
    /// buf.clear();
    ///
    /// let mut pod = Pod::new(&mut buf);
    /// pod.encode_id(MediaSubType::Opus)?;
    /// let mut pod = Pod::new(buf.as_slice());
    /// let sub_type: MediaSubType = pod.decode_id()?;
    /// assert_eq!(sub_type, MediaSubType::Opus);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn decode_id<I>(&mut self) -> Result<I, Error>
    where
        I: IntoId,
    {
        let Id(id) = Id::<I>::decode(self.buf.borrow_mut())?;
        Ok(id)
    }

    /// Decode an array.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Pod, Type};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let mut pod = Pod::new(&mut buf);
    /// let mut array = pod.encode_array(Type::INT)?;
    ///
    /// array.encode(1i32)?;
    /// array.encode(2i32)?;
    /// array.encode(3i32)?;
    ///
    /// array.close()?;
    ///
    /// let mut pod = Pod::new(buf.as_slice());
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
    #[inline]
    pub fn decode_array(&mut self) -> Result<DecodeArray<B::Mut<'_>>, Error> {
        let (full_size, ty) = self.buf.header()?;

        match ty {
            Type::ARRAY if full_size >= 8 => {
                let size = full_size - 8;

                let [child_size, child_type] = self.buf.read()?;
                let child_type = Type::new(child_type);

                let remaining = if size > 0 && child_size > 0 {
                    if size % child_size != 0 {
                        return Err(Error::new(ErrorKind::InvalidArraySize {
                            size: full_size,
                            child_size,
                        }));
                    }

                    let padded_child_size = child_size.next_multiple_of(WORD_SIZE as u32);
                    (size / padded_child_size) as usize
                } else {
                    0
                };

                Ok(DecodeArray::new(
                    self.buf.borrow_mut(),
                    child_type,
                    child_size as usize,
                    remaining,
                ))
            }
            _ => Err(Error::new(ErrorKind::Expected {
                expected: Type::ARRAY,
                actual: ty,
            })),
        }
    }
}
