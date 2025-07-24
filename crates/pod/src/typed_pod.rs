use core::ffi::CStr;
use core::fmt;

use crate::bstr::BStr;
use crate::de::{DecodeArray, DecodeStruct};
use crate::error::ErrorKind;
use crate::id::IntoId;
use crate::{
    Bitmap, Decode, DecodeUnsized, Error, Fraction, Id, Reader, Rectangle, Type, Visitor, WORD_SIZE,
};

/// A POD (Plain Old Data) handler.
///
/// This is a wrapper that can be used for encoding and decoding data.
pub struct TypedPod<B> {
    size: u32,
    ty: Type,
    buf: B,
}

impl<B> Clone for TypedPod<B>
where
    B: Clone,
{
    #[inline]
    fn clone(&self) -> Self {
        TypedPod {
            size: self.size,
            ty: self.ty,
            buf: self.buf.clone(),
        }
    }
}

impl<B> TypedPod<B> {
    /// Construct a new [`TypedPod`] arround the specified buffer `B`.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Type, TypedPod};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let pod = TypedPod::new(4, Type::INT, &mut buf);
    /// ```
    #[inline]
    pub const fn new(size: u32, ty: Type, buf: B) -> Self {
        TypedPod { size, ty, buf }
    }

    /// Get the type of the pod.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Pod, TypedPod, Type};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let pod = Pod::new(&mut buf);
    /// pod.encode(10i32)?;
    ///
    /// let pod = TypedPod::from_reader(buf.as_slice())?;
    /// assert_eq!(pod.ty(), Type::INT);
    /// assert_eq!(pod.size(), 4);
    /// # Ok::<_, pod::Error>(())
    /// ```
    pub const fn ty(&self) -> Type {
        self.ty
    }

    /// Get the size of the pod.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Pod, TypedPod, Type};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let pod = Pod::new(&mut buf);
    /// pod.encode(10i32)?;
    ///
    /// let pod = TypedPod::from_reader(buf.as_slice())?;
    /// assert_eq!(pod.ty(), Type::INT);
    /// assert_eq!(pod.size(), 4);
    /// # Ok::<_, pod::Error>(())
    /// ```
    pub const fn size(&self) -> u32 {
        self.size
    }

    /// Get the size of the padded pod including the header.
    pub(crate) fn size_with_header(&self) -> Option<u32> {
        self.size
            .next_multiple_of(WORD_SIZE as u32)
            .checked_add(WORD_SIZE as u32)
    }
}

impl<'de, B> TypedPod<B>
where
    B: Reader<'de>,
{
    /// Construct a new [`TypedPod`] by reading and advancing the given buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Pod, TypedPod};
    /// let mut buf = ArrayBuf::new();
    /// let pod = Pod::new(&mut buf);
    /// pod.encode(10i32)?;
    ///
    /// let pod = TypedPod::from_reader(buf.as_slice())?;
    /// assert_eq!(pod.decode::<i32>()?, 10i32);
    /// # Ok::<_, pod::Error>(())
    /// ```
    pub fn from_reader(mut buf: B) -> Result<Self, Error> {
        let (size, ty) = buf.header()?;
        Ok(TypedPod { size, ty, buf })
    }

    /// Encode a value into the pod.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Pod, TypedPod};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let pod = Pod::new(&mut buf);
    /// pod.encode(10i32)?;
    ///
    /// let pod = TypedPod::from_reader(buf.as_slice())?;
    /// assert_eq!(pod.decode::<i32>()?, 10i32);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn decode<T>(self) -> Result<T, Error>
    where
        T: Decode<'de>,
    {
        if T::TYPE != self.ty {
            return Err(Error::new(ErrorKind::Expected {
                expected: self.ty,
                actual: T::TYPE,
            }));
        }

        let Ok(size) = usize::try_from(self.size) else {
            return Err(Error::new(ErrorKind::SizeOverflow));
        };

        T::read_content(self.buf, size)
    }

    /// Decode an unsized value into the pod.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Pod, TypedPod};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let pod = Pod::new(&mut buf);
    /// pod.encode_unsized(&b"hello world"[..])?;
    ///
    /// let pod = TypedPod::from_reader(buf.as_slice())?;
    /// assert_eq!(pod.decode_borrowed::<[u8]>()?, b"hello world");
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn decode_unsized<T, V>(self, visitor: V) -> Result<V::Ok, Error>
    where
        T: ?Sized + DecodeUnsized<'de>,
        V: Visitor<'de, T>,
    {
        if T::TYPE != self.ty {
            return Err(Error::new(ErrorKind::Expected {
                expected: self.ty,
                actual: T::TYPE,
            }));
        }

        let Ok(size) = usize::try_from(self.size) else {
            return Err(Error::new(ErrorKind::SizeOverflow));
        };

        T::read_content(self.buf, visitor, size)
    }

    /// Decode an unsized value into the pod.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Pod, TypedPod};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let pod = Pod::new(&mut buf);
    ///
    /// pod.encode_unsized(&b"hello world"[..])?;
    ///
    /// let pod = TypedPod::from_reader(buf.as_slice())?;
    /// assert_eq!(pod.decode_borrowed::<[u8]>()?, b"hello world");
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn decode_borrowed<T>(self) -> Result<&'de T, Error>
    where
        T: ?Sized + DecodeUnsized<'de>,
    {
        if T::TYPE != self.ty {
            return Err(Error::new(ErrorKind::Expected {
                expected: self.ty,
                actual: T::TYPE,
            }));
        }

        let Ok(size) = usize::try_from(self.size) else {
            return Err(Error::new(ErrorKind::SizeOverflow));
        };

        T::read_borrowed(self.buf, size)
    }

    /// Decode an optional value.
    ///
    /// This returns `None` if the encoded value is `None`, otherwise a pod
    /// for the value is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Pod, TypedPod};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let pod = Pod::new(&mut buf);
    /// pod.encode_none()?;
    ///
    /// let pod = TypedPod::from_reader(buf.as_slice())?;
    /// assert!(pod.decode_option()?.is_none());
    ///
    /// buf.clear();
    ///
    /// let pod = Pod::new(&mut buf);
    /// pod.encode(true)?;
    ///
    /// let pod = TypedPod::from_reader(buf.as_slice())?;
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
        match self.ty {
            Type::NONE => Ok(None),
            _ => Ok(Some(TypedPod::new(self.size, self.ty, self.buf))),
        }
    }

    /// Decode an id value.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Pod, TypedPod};
    /// use pod::id::MediaSubType;
    ///
    /// let mut buf = ArrayBuf::new();
    /// let pod = Pod::new(&mut buf);
    ///
    /// pod.encode_id(MediaSubType::Opus)?;
    /// let pod = TypedPod::from_reader(buf.as_slice())?;
    /// assert_eq!(pod.decode_id::<MediaSubType>()?, MediaSubType::Opus);
    ///
    /// buf.clear();
    ///
    /// let pod = Pod::new(&mut buf);
    /// pod.encode_id(MediaSubType::Opus)?;
    ///
    /// let pod = TypedPod::from_reader(buf.as_slice())?;
    /// assert_eq!(pod.decode_id::<MediaSubType>()?, MediaSubType::Opus);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn decode_id<I>(self) -> Result<I, Error>
    where
        I: IntoId,
    {
        if self.ty != Type::ID {
            return Err(Error::new(ErrorKind::Expected {
                expected: Type::ID,
                actual: self.ty,
            }));
        }

        let Ok(size) = usize::try_from(self.size) else {
            return Err(Error::new(ErrorKind::SizeOverflow));
        };

        let Id(id) = Id::<I>::read_content(self.buf, size)?;
        Ok(id)
    }

    /// Decode an array.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Pod, TypedPod, Type};
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
    /// let pod = TypedPod::from_reader(buf.as_slice())?;
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
    pub fn decode_array(mut self) -> Result<DecodeArray<B>, Error> {
        match self.ty {
            Type::ARRAY if self.size >= 8 => {
                let [child_size, child_type] = self.buf.read()?;
                let child_type = Type::new(child_type);

                let size = self.size - 8;

                let remaining = if size > 0 && child_size > 0 {
                    if size % child_size != 0 {
                        return Err(Error::new(ErrorKind::InvalidArraySize { size, child_size }));
                    }

                    let padded_child_size = child_size.next_multiple_of(WORD_SIZE as u32);

                    let Ok(size) = usize::try_from(size / padded_child_size) else {
                        return Err(Error::new(ErrorKind::SizeOverflow));
                    };

                    size
                } else {
                    0
                };

                Ok(DecodeArray::new(
                    self.buf, child_size, child_type, remaining,
                ))
            }
            _ => Err(Error::new(ErrorKind::Expected {
                expected: Type::ARRAY,
                actual: self.ty,
            })),
        }
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
    /// let pod = TypedPod::from_reader(buf.as_slice())?;
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
        match self.ty {
            Type::STRUCT => Ok(DecodeStruct::new(self.buf, self.size)),
            _ => Err(Error::new(ErrorKind::Expected {
                expected: Type::STRUCT,
                actual: self.ty,
            })),
        }
    }

    fn debug_fmt_with_type(self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let primitive = !matches!(self.ty, Type::ARRAY | Type::STRUCT);

        if primitive {
            write!(f, "{}(", self.ty)?;
        }

        self.debug_fmt(f)?;

        if primitive {
            write!(f, ")")?;
        }

        Ok(())
    }

    fn debug_fmt(self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.ty {
            Type::NONE => f.write_str("None"),
            Type::BOOL => {
                let value = self.decode::<bool>().map_err(|_| fmt::Error)?;
                write!(f, "{value:?}")
            }
            Type::ID => {
                let Id(value) = self.decode::<Id<u32>>().map_err(|_| fmt::Error)?;
                write!(f, "{value:?}")
            }
            Type::INT => {
                let value = self.decode::<i32>().map_err(|_| fmt::Error)?;
                write!(f, "{value:?}")
            }
            Type::LONG => {
                let value = self.decode::<i64>().map_err(|_| fmt::Error)?;
                write!(f, "{value:?}")
            }
            Type::FLOAT => {
                let value = self.decode::<f32>().map_err(|_| fmt::Error)?;
                write!(f, "{value:?}")
            }
            Type::DOUBLE => {
                let value = self.decode::<f64>().map_err(|_| fmt::Error)?;
                write!(f, "{value:?}")
            }
            Type::STRING => {
                let value = self.decode_borrowed::<CStr>().map_err(|_| fmt::Error)?;
                write!(f, "{value:?}")
            }
            Type::BYTES => {
                let value = self.decode_borrowed::<[u8]>().map_err(|_| fmt::Error)?;
                write!(f, "{:?}", BStr::new(value))
            }
            Type::RECTANGLE => {
                let value = self.decode::<Rectangle>().map_err(|_| fmt::Error)?;
                write!(f, "{value:?}")
            }
            Type::FRACTION => {
                let value = self.decode::<Fraction>().map_err(|_| fmt::Error)?;
                write!(f, "{value:?}")
            }
            Type::BITMAP => {
                let value = self
                    .typed()
                    .decode_borrowed::<Bitmap>()
                    .map_err(|_| fmt::Error)?;
                write!(f, "{value:?}")
            }
            Type::ARRAY => {
                let mut array = self.decode_array().map_err(|_| fmt::Error)?;
                write!(f, "Array[{:?}](", array.child_type())?;

                while !array.is_empty() {
                    array.next().map_err(|_| fmt::Error)?.debug_fmt(f)?;

                    if !array.is_empty() {
                        write!(f, ", ")?;
                    }
                }

                write!(f, ")")?;
                Ok(())
            }
            Type::STRUCT => {
                let mut st = self.decode_struct().map_err(|_| fmt::Error)?;
                write!(f, "Struct(")?;

                while !st.is_empty() {
                    let pod = st.next().map_err(|_| fmt::Error)?;

                    write!(f, "{:?}: ", pod.ty())?;
                    pod.debug_fmt(f)?;

                    if !st.is_empty() {
                        write!(f, ", ")?;
                    }
                }

                write!(f, ")")?;
                Ok(())
            }
            ty => {
                write!(f, "{ty:?}")
            }
        }
    }

    #[inline]
    fn typed(&self) -> TypedPod<B::Clone<'_>> {
        TypedPod::new(self.size, self.ty, self.buf.clone_reader())
    }
}

impl<'de, B> fmt::Debug for TypedPod<B>
where
    B: Reader<'de>,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.typed().debug_fmt_with_type(f)
    }
}
