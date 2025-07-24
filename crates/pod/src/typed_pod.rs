use core::ffi::CStr;
use core::fmt;

use crate::de::DecodeArray;
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

        T::read_content(self.buf, self.size as usize)
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

        T::read_content(self.buf, visitor, self.size as usize)
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

        T::read_borrowed(self.buf, self.size as usize)
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

        let Id(id) = Id::<I>::read_content(self.buf, self.size as usize)?;
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
                    (size / padded_child_size) as usize
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
        match self.ty {
            Type::NONE => f.write_str("None"),
            Type::BOOL => {
                let value = self.typed().decode::<bool>().map_err(|_| fmt::Error)?;
                write!(f, "Bool({value:?})")
            }
            Type::ID => {
                let Id(value) = self.typed().decode::<Id<u32>>().map_err(|_| fmt::Error)?;
                write!(f, "Id({value:?})")
            }
            Type::INT => {
                let value = self.typed().decode::<i32>().map_err(|_| fmt::Error)?;
                write!(f, "Int({value:?})")
            }
            Type::LONG => {
                let value = self.typed().decode::<i64>().map_err(|_| fmt::Error)?;
                write!(f, "Long({value:?})")
            }
            Type::FLOAT => {
                let value = self.typed().decode::<f32>().map_err(|_| fmt::Error)?;
                write!(f, "Float({value:?})")
            }
            Type::DOUBLE => {
                let value = self.typed().decode::<f64>().map_err(|_| fmt::Error)?;
                write!(f, "Double({value:?})")
            }
            Type::STRING => {
                let value = self
                    .typed()
                    .decode_borrowed::<CStr>()
                    .map_err(|_| fmt::Error)?;
                write!(f, "String({value:?})")
            }
            Type::BYTES => {
                let value = self
                    .typed()
                    .decode_borrowed::<[u8]>()
                    .map_err(|_| fmt::Error)?;
                write!(f, "Bytes({value:?})")
            }
            Type::RECTANGLE => {
                let value = self.typed().decode::<Rectangle>().map_err(|_| fmt::Error)?;
                write!(f, "Rectangle({value:?})")
            }
            Type::FRACTION => {
                let value = self.typed().decode::<Fraction>().map_err(|_| fmt::Error)?;
                write!(f, "Fraction({value:?})")
            }
            Type::BITMAP => {
                let value = self
                    .typed()
                    .decode_borrowed::<Bitmap>()
                    .map_err(|_| fmt::Error)?;
                write!(f, "Bitmap({value:?})")
            }
            Type::ARRAY => {
                let mut array = self.typed().decode_array().map_err(|_| fmt::Error)?;
                write!(f, "Array[{:?}](", array.child_type())?;

                while !array.is_empty() {
                    let pod = array.next().map_err(|_| fmt::Error)?;
                    write!(f, "{pod:?}")?;
                }

                write!(f, ")")?;
                Ok(())
            }
            ty => {
                write!(f, "{ty:?}")
            }
        }
    }
}
