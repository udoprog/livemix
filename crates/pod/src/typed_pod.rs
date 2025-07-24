use core::ffi::CStr;
use core::fmt;

use crate::bstr::BStr;
use crate::de::{ArrayDecoder, ObjectDecoder, SequenceDecoder, StructDecoder};
use crate::error::ErrorKind;
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

impl<B> TypedPod<B> {
    /// Construct a new [`TypedPod`] arround the specified buffer `B`.
    #[inline]
    pub(crate) const fn new(size: u32, ty: Type, buf: B) -> Self {
        TypedPod { size, ty, buf }
    }

    /// Get the type of the pod.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = Pod::array();
    /// pod.encode(10i32)?;
    ///
    /// let pod = pod.typed()?;
    /// assert_eq!(pod.ty(), Type::INT);
    /// assert_eq!(pod.size(), 4);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub const fn ty(&self) -> Type {
        self.ty
    }

    /// Get the size of the pod.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, TypedPod, Type};
    ///
    /// let mut pod = Pod::array();
    /// pod.encode(10i32)?;
    ///
    /// let pod = pod.typed()?;
    /// assert_eq!(pod.ty(), Type::INT);
    /// assert_eq!(pod.size(), 4);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub const fn size(&self) -> u32 {
        self.size
    }

    /// Get the size of the padded pod including the header.
    #[inline]
    pub(crate) fn size_with_header(&self) -> Option<u32> {
        self.size.next_multiple_of(WORD_SIZE).checked_add(WORD_SIZE)
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
    /// use pod::{Pod, TypedPod};
    ///
    /// let mut pod = Pod::array();
    /// pod.encode(10i32)?;
    ///
    /// let pod = pod.typed()?;
    /// assert_eq!(pod.decode::<i32>()?, 10i32);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn from_reader(mut buf: B) -> Result<Self, Error> {
        let (size, ty) = buf.header()?;
        Ok(TypedPod { size, ty, buf })
    }

    /// Skip a value in the pod.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, TypedPod, Type};
    ///
    /// let mut pod = Pod::array();
    ///
    /// let mut array = pod.encode_array(Type::INT)?;
    /// array.encode(10i32)?;
    /// array.encode(20i32)?;
    /// array.close()?;
    ///
    /// let pod = pod.typed()?;
    /// let mut array = pod.decode_array()?;
    /// assert!(!array.is_empty());
    /// array.item()?.skip()?;
    /// assert_eq!(array.item()?.decode::<i32>()?, 20i32);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn skip(mut self) -> Result<(), Error> {
        self.buf.skip(self.size)?;
        Ok(())
    }

    /// Encode a value into the pod.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, TypedPod};
    ///
    /// let mut pod = Pod::array();
    ///
    /// pod.encode(10i32)?;
    ///
    /// let pod = pod.typed()?;
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
                expected: T::TYPE,
                actual: self.ty,
            }));
        }

        T::read_content(self.buf, self.size)
    }

    /// Decode an unsized value into the pod.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, TypedPod};
    ///
    /// let mut pod = Pod::array();
    ///
    /// pod.encode_unsized(&b"hello world"[..])?;
    ///
    /// let pod = pod.typed()?;
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

        T::read_content(self.buf, visitor, self.size)
    }

    /// Decode an unsized value into the pod.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, TypedPod};
    ///
    /// let mut pod = Pod::array();
    ///
    /// pod.encode_unsized(&b"hello world"[..])?;
    ///
    /// let pod = pod.typed()?;
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
                expected: T::TYPE,
                actual: self.ty,
            }));
        }

        T::read_borrowed(self.buf, self.size)
    }

    /// Decode an optional value.
    ///
    /// This returns `None` if the encoded value is `None`, otherwise a pod
    /// for the value is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, TypedPod};
    ///
    /// let mut pod = Pod::array();
    /// pod.encode_none()?;
    ///
    /// let pod = pod.typed()?;
    /// assert!(pod.decode_option()?.is_none());
    ///
    /// let mut pod = Pod::array();
    /// pod.encode(true)?;
    ///
    /// let pod = pod.typed()?;
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

    /// Decode an array.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, TypedPod, Type};
    ///
    /// let mut pod = Pod::array();
    /// let mut array = pod.encode_array(Type::INT)?;
    ///
    /// array.encode(1i32)?;
    /// array.encode(2i32)?;
    /// array.encode(3i32)?;
    ///
    /// array.close()?;
    ///
    /// let pod = pod.typed()?;
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
    pub fn decode_array(self) -> Result<ArrayDecoder<B>, Error> {
        match self.ty {
            Type::ARRAY => ArrayDecoder::from_reader(self.buf, self.size),
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
    /// use pod::{Pod, TypedPod};
    ///
    /// let mut pod = Pod::array();
    /// let mut st = pod.encode_struct()?;
    ///
    /// st.field()?.encode(1i32)?;
    /// st.field()?.encode(2i32)?;
    /// st.field()?.encode(3i32)?;
    ///
    /// st.close()?;
    ///
    /// let pod = pod.typed()?;
    /// let mut st = pod.decode_struct()?;
    ///
    /// assert!(!st.is_empty());
    /// assert_eq!(st.field()?.decode::<i32>()?, 1i32);
    /// assert_eq!(st.field()?.decode::<i32>()?, 2i32);
    /// assert_eq!(st.field()?.decode::<i32>()?, 3i32);
    /// assert!(st.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn decode_struct(self) -> Result<StructDecoder<B>, Error> {
        match self.ty {
            Type::STRUCT => Ok(StructDecoder::new(self.buf, self.size)),
            _ => Err(Error::new(ErrorKind::Expected {
                expected: Type::STRUCT,
                actual: self.ty,
            })),
        }
    }

    /// Decode an object.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = Pod::array();
    /// let mut obj = pod.encode_object(10, 20)?;
    ///
    /// obj.property(1, 10)?.encode(1i32)?;
    /// obj.property(2, 20)?.encode(2i32)?;
    /// obj.property(3, 30)?.encode(3i32)?;
    ///
    /// obj.close()?;
    ///
    /// let mut obj = pod.decode_object()?;
    ///
    /// assert!(!obj.is_empty());
    ///
    /// let p = obj.property()?;
    /// assert_eq!(p.key(), 1);
    /// assert_eq!(p.flags(), 10);
    /// assert_eq!(p.value().decode::<i32>()?, 1);
    ///
    /// let p = obj.property()?;
    /// assert_eq!(p.key(), 2);
    /// assert_eq!(p.flags(), 20);
    /// assert_eq!(p.value().decode::<i32>()?, 2);
    ///
    /// let p = obj.property()?;
    /// assert_eq!(p.key(), 3);
    /// assert_eq!(p.flags(), 30);
    /// assert_eq!(p.value().decode::<i32>()?, 3);
    ///
    /// assert!(obj.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn decode_object(self) -> Result<ObjectDecoder<B>, Error> {
        match self.ty {
            Type::OBJECT => ObjectDecoder::from_reader(self.buf, self.size),
            _ => Err(Error::new(ErrorKind::Expected {
                expected: Type::OBJECT,
                actual: self.ty,
            })),
        }
    }

    /// Decode a sequence.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, TypedPod, Type};
    ///
    /// let mut pod = Pod::array();
    /// let mut seq = pod.encode_sequence()?;
    ///
    /// seq.control(1, 10)?.encode(1i32)?;
    /// seq.control(2, 20)?.encode(2i32)?;
    /// seq.control(3, 30)?.encode(3i32)?;
    ///
    /// seq.close()?;
    ///
    /// let mut pod = pod.typed()?;
    /// let mut seq = pod.decode_sequence()?;
    ///
    /// assert!(!seq.is_empty());
    ///
    /// let c = seq.control()?;
    /// assert_eq!(c.offset(), 1);
    /// assert_eq!(c.ty(), 10);
    /// assert_eq!(c.value().decode::<i32>()?, 1);
    ///
    /// let c = seq.control()?;
    /// assert_eq!(c.offset(), 2);
    /// assert_eq!(c.ty(), 20);
    /// assert_eq!(c.value().decode::<i32>()?, 2);
    ///
    /// let c = seq.control()?;
    /// assert_eq!(c.offset(), 3);
    /// assert_eq!(c.ty(), 30);
    /// assert_eq!(c.value().decode::<i32>()?, 3);
    ///
    /// assert!(seq.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn decode_sequence(self) -> Result<SequenceDecoder<B>, Error> {
        match self.ty {
            Type::SEQUENCE => SequenceDecoder::from_reader(self.buf, self.size),
            _ => Err(Error::new(ErrorKind::Expected {
                expected: Type::SEQUENCE,
                actual: self.ty,
            })),
        }
    }

    pub(crate) fn debug_fmt_with_type(self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let container = matches!(
            self.ty,
            Type::ARRAY | Type::STRUCT | Type::OBJECT | Type::SEQUENCE
        );
        write!(f, "{}", self.ty)?;

        if !container {
            write!(f, "(")?;
        }

        self.debug_fmt(f)?;

        if !container {
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
                write!(
                    f,
                    "{{width: {:?}, height: {:?}}}",
                    value.width, value.height
                )
            }
            Type::FRACTION => {
                let value = self.decode::<Fraction>().map_err(|_| fmt::Error)?;
                write!(f, "{{num: {:?}, denom: {:?}}}", value.num, value.denom)
            }
            Type::BITMAP => {
                let value = self
                    .typed()
                    .decode_borrowed::<Bitmap>()
                    .map_err(|_| fmt::Error)?;
                write!(f, "{:?}", BStr::new(value.as_bytes()))
            }
            Type::ARRAY => {
                let mut array = self.decode_array().map_err(|_| fmt::Error)?;
                write!(f, "[{:?}](", array.child_type())?;

                while !array.is_empty() {
                    array.item().map_err(|_| fmt::Error)?.debug_fmt(f)?;

                    if !array.is_empty() {
                        write!(f, ", ")?;
                    }
                }

                write!(f, ")")?;
                Ok(())
            }
            Type::STRUCT => {
                let mut st = self.decode_struct().map_err(|_| fmt::Error)?;
                write!(f, "{{")?;

                while !st.is_empty() {
                    {
                        let pod = st.field().map_err(|_| fmt::Error)?;
                        write!(f, "{:?}: ", pod.ty())?;
                        pod.debug_fmt(f)?;
                    }

                    if !st.is_empty() {
                        write!(f, ", ")?;
                    }
                }

                write!(f, "}}")?;
                Ok(())
            }
            Type::OBJECT => {
                let mut st = self.decode_object().map_err(|_| fmt::Error)?;
                write!(f, "[{}, {}]{{", st.object_type(), st.object_id())?;

                while !st.is_empty() {
                    let prop = st.property().map_err(|_| fmt::Error)?;

                    if prop.flags() != 0 {
                        write!(f, "{{key: {}, flags: 0b{:b}}}: ", prop.key(), prop.flags())?;
                    } else {
                        write!(f, "{:?}: ", prop.key())?;
                    }

                    prop.value().debug_fmt_with_type(f)?;

                    if !st.is_empty() {
                        write!(f, ", ")?;
                    }
                }

                write!(f, "}}")?;
                Ok(())
            }
            Type::SEQUENCE => {
                let mut st = self.decode_sequence().map_err(|_| fmt::Error)?;
                write!(f, "[{}, {}]{{", st.unit(), st.pad())?;

                while !st.is_empty() {
                    let c = st.control().map_err(|_| fmt::Error)?;
                    write!(f, "{{offset: {:?}, type: {:?}}}: ", c.offset(), c.ty())?;

                    c.value().debug_fmt_with_type(f)?;

                    if !st.is_empty() {
                        write!(f, ", ")?;
                    }
                }

                write!(f, "}}")?;
                Ok(())
            }
            ty => {
                self.skip().map_err(|_| fmt::Error)?;
                write!(f, "{ty:?}")
            }
        }
    }

    #[inline]
    fn typed(&self) -> TypedPod<B::Clone<'_>> {
        TypedPod::new(self.size, self.ty, self.buf.clone_reader())
    }
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

impl<'de, B> fmt::Debug for TypedPod<B>
where
    B: Reader<'de>,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.typed().debug_fmt_with_type(f)
    }
}
