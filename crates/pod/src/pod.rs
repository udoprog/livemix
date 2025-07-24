use core::fmt;

use crate::de::{DecodeArray, DecodeStruct};
use crate::en::{EncodeArray, EncodeStruct};
use crate::error::ErrorKind;
use crate::id::IntoId;
use crate::{
    Decode, DecodeUnsized, Encode, EncodeUnsized, Error, Id, Reader, Type, TypedPod, Visitor,
    Writer,
};

/// A POD (Plain Old Data) handler.
///
/// This is a wrapper that can be used for encoding and decoding data.
pub struct Pod<B> {
    buf: B,
}

impl<B> Clone for Pod<B>
where
    B: Clone,
{
    #[inline]
    fn clone(&self) -> Self {
        Pod {
            buf: self.buf.clone(),
        }
    }
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
    /// let pod = Pod::new(&mut buf);
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
    /// let pod = Pod::new(&mut buf);
    /// pod.encode(10i32)?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    pub fn encode<T>(self, value: T) -> Result<(), Error>
    where
        T: Encode,
    {
        value.encode(self.buf)
    }

    /// Encode an unsized value into the pod.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Pod};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let pod = Pod::new(&mut buf);
    /// pod.encode_unsized(&b"hello world"[..])?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    pub fn encode_unsized<T>(self, value: &T) -> Result<(), Error>
    where
        T: ?Sized + EncodeUnsized,
    {
        value.encode_unsized(self.buf)
    }

    /// Encode a `None` value.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Pod};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let pod = Pod::new(&mut buf);
    /// pod.encode_none()?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn encode_none(mut self) -> Result<(), Error> {
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
    /// let pod = Pod::new(&mut buf);
    /// pod.encode_id(MediaSubType::Opus)?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn encode_id(self, value: impl IntoId) -> Result<(), Error> {
        Id(value).encode(self.buf)
    }

    /// Encode an array with the given type.
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
    #[inline]
    pub fn encode_array(mut self, child_type: Type) -> Result<EncodeArray<B>, Error> {
        let Some(child_size) = child_type.size() else {
            return Err(Error::new(ErrorKind::UnsizedTypeInArray { ty: child_type }));
        };

        let pos = self.buf.reserve_words(&[0, 0])?;
        Ok(EncodeArray::new(self.buf, child_size, child_type, pos))
    }

    /// Encode a struct.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Pod, Type};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let pod = Pod::new(&mut buf);
    /// let mut st = pod.encode_struct()?;
    ///
    /// st.add()?.encode(1i32)?;
    /// st.add()?.encode(2i32)?;
    /// st.add()?.encode(3i32)?;
    ///
    /// st.close()?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn encode_struct(mut self) -> Result<EncodeStruct<B>, Error> {
        // Reserve space for the header of the struct which includes its size that will be determined later.
        let header = self.buf.reserve_words(&[0])?;
        Ok(EncodeStruct::new(self.buf, header))
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
    /// let pod = Pod::new(&mut buf);
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
        mut self,
        child_type: Type,
        len: usize,
    ) -> Result<EncodeArray<B>, Error> {
        if let Some(child_size) = child_type.size() {
            if child_size != len {
                return Err(Error::new(ErrorKind::ArrayChildSizeMismatch {
                    actual: len,
                    expected: child_size,
                }));
            }
        };

        let pos = self.buf.reserve_words(&[0, 0])?;
        Ok(EncodeArray::new(self.buf, len, child_type, pos))
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
    /// let pod = Pod::new(&mut buf);
    /// pod.encode(10i32)?;
    ///
    /// let pod = Pod::new(buf.as_slice());
    /// assert_eq!(pod.decode::<i32>()?, 10i32);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn decode<T>(self) -> Result<T, Error>
    where
        T: Decode<'de>,
    {
        T::decode(self.buf)
    }

    /// Decode an unsized value into the pod.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Pod};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let pod = Pod::new(&mut buf);
    /// pod.encode_unsized(&b"hello world"[..])?;
    ///
    /// let pod = Pod::new(buf.as_slice());
    /// assert_eq!(pod.decode_borrowed::<[u8]>()?, b"hello world");
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn decode_unsized<T, V>(self, visitor: V) -> Result<V::Ok, Error>
    where
        T: ?Sized + DecodeUnsized<'de>,
        V: Visitor<'de, T>,
    {
        T::decode_unsized(self.buf, visitor)
    }

    /// Decode an unsized value into the pod.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Pod};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let pod = Pod::new(&mut buf);
    ///
    /// pod.encode_unsized(&b"hello world"[..])?;
    ///
    /// let pod = Pod::new(buf.as_slice());
    /// assert_eq!(pod.decode_borrowed::<[u8]>()?, b"hello world");
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn decode_borrowed<T>(self) -> Result<&'de T, Error>
    where
        T: ?Sized + DecodeUnsized<'de>,
    {
        T::decode_borrowed(self.buf)
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
    /// let pod = Pod::new(&mut buf);
    /// pod.encode_none()?;
    ///
    /// let pod = Pod::new(buf.as_slice());
    /// assert!(pod.decode_option()?.is_none());
    ///
    /// buf.clear();
    ///
    /// let pod = Pod::new(&mut buf);
    /// pod.encode(true)?;
    ///
    /// let pod = Pod::new(buf.as_slice());
    ///
    /// let Some(mut pod) = pod.decode_option()? else {
    ///     panic!("expected some value");
    /// };
    ///
    /// assert!(pod.decode::<bool>()?);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn decode_option(self) -> Result<Option<TypedPod<B>>, Error> {
        self.typed()?.decode_option()
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
    /// let pod = Pod::new(&mut buf);
    ///
    /// pod.encode_id(MediaSubType::Opus)?;
    /// let pod = Pod::new(buf.as_slice());
    /// let sub_type: MediaSubType = pod.decode_id()?;
    /// assert_eq!(sub_type, MediaSubType::Opus);
    ///
    /// buf.clear();
    ///
    /// let pod = Pod::new(&mut buf);
    /// pod.encode_id(MediaSubType::Opus)?;
    /// let pod = Pod::new(buf.as_slice());
    /// let sub_type: MediaSubType = pod.decode_id()?;
    /// assert_eq!(sub_type, MediaSubType::Opus);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn decode_id<I>(self) -> Result<I, Error>
    where
        I: IntoId,
    {
        let Id(id) = Id::<I>::decode(self.buf)?;
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
    #[inline]
    pub fn decode_array(self) -> Result<DecodeArray<B>, Error> {
        self.typed()?.decode_array()
    }

    /// Decode a struct.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Pod, TypedPod};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let pod = Pod::new(&mut buf);
    /// let mut st = pod.encode_struct()?;
    ///
    /// st.add()?.encode(1i32)?;
    /// st.add()?.encode(2i32)?;
    /// st.add()?.encode(3i32)?;
    ///
    /// st.close()?;
    ///
    /// let pod = Pod::new(buf.as_slice());
    /// let mut st = pod.decode_struct()?;
    ///
    /// assert!(!st.is_empty());
    /// assert_eq!(st.next()?.decode::<i32>()?, 1i32);
    /// assert_eq!(st.next()?.decode::<i32>()?, 2i32);
    /// assert_eq!(st.next()?.decode::<i32>()?, 3i32);
    /// assert!(st.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn decode_struct(self) -> Result<DecodeStruct<B>, Error> {
        self.typed()?.decode_struct()
    }

    #[inline]
    fn typed(self) -> Result<TypedPod<B>, Error> {
        TypedPod::from_reader(self.buf)
    }
}

impl<'de, B> fmt::Debug for Pod<B>
where
    B: Reader<'de>,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut buf = self.buf.clone_reader();
        let (size, ty) = buf.header().map_err(|_| fmt::Error)?;
        let pod = TypedPod::new(size, ty, buf);
        write!(f, "{pod:?}")
    }
}
