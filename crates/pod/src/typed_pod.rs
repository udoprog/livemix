use core::ffi::CStr;
use core::fmt;
use core::mem;

#[cfg(feature = "alloc")]
use crate::DynamicBuf;
use crate::EncodeUnsized;
use crate::PackedPod;
use crate::bstr::BStr;
#[cfg(feature = "alloc")]
use crate::buf::AllocError;
use crate::de::{Array, Choice, Object, Sequence, Struct};
use crate::error::ErrorKind;
use crate::{
    AsReader, Bitmap, Decode, DecodeUnsized, Error, Fd, Fraction, Id, PaddedPod, Pod, Pointer,
    ReadPod, Reader, Rectangle, Type, Visitor, Writer,
};

/// A POD (Plain Old Data) handler.
///
/// This is a wrapper that can be used for encoding and decoding data.
pub struct TypedPod<B, P = PaddedPod> {
    buf: B,
    size: usize,
    ty: Type,
    kind: P,
}

impl<B> TypedPod<B, PackedPod> {
    /// Construct a new [`TypedPod`] arround the specified buffer `B` and
    /// a [`PackedPod`] kind.
    #[inline]
    pub(crate) const fn packed(buf: B, size: usize, ty: Type) -> Self {
        Self::with_kind(buf, size, ty, PackedPod)
    }
}

impl<B, P> TypedPod<B, P> {
    /// Construct a new [`TypedPod`] arround the specified buffer `B` and
    /// specified kind `P`.
    #[inline]
    pub(crate) const fn with_kind(buf: B, size: usize, ty: Type, kind: P) -> Self {
        TypedPod {
            buf,
            size,
            ty,
            kind,
        }
    }

    /// Get the type of the pod.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = pod::array();
    /// pod.as_mut().push(10i32)?;
    ///
    /// let pod = pod.as_ref().into_typed()?;
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
    /// let mut pod = pod::array();
    /// pod.as_mut().push(10i32)?;
    ///
    /// let pod = pod.as_ref().into_typed()?;
    /// assert_eq!(pod.ty(), Type::INT);
    /// assert_eq!(pod.size(), 4);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub const fn size(&self) -> usize {
        self.size
    }
}

impl<'de, B, P> TypedPod<B, P>
where
    B: Reader<'de>,
    P: ReadPod,
{
    /// Construct a new [`TypedPod`] by reading and advancing the given buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, TypedPod};
    ///
    /// let mut pod = pod::array();
    /// pod.as_mut().push(10i32)?;
    ///
    /// let pod = pod.as_ref().into_typed()?;
    /// assert_eq!(pod.as_ref().next::<i32>()?, 10i32);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn from_reader(mut buf: B, kind: P) -> Result<Self, Error> {
        let (size, ty) = buf.header()?;

        Ok(TypedPod {
            size,
            ty,
            buf,
            kind,
        })
    }

    /// Skip a value in the pod.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, TypedPod, Type};
    ///
    /// let mut pod = pod::array();
    ///
    /// pod.as_mut().push_array(Type::INT, |array| {
    ///     array.child().push(10i32)?;
    ///     array.child().push(20i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let pod = pod.as_ref().into_typed()?;
    /// let mut array = pod.next_array()?;
    /// assert!(!array.is_empty());
    /// array.next().unwrap();
    /// assert_eq!(array.next().unwrap().next::<i32>()?, 20i32);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn skip(mut self) -> Result<(), Error> {
        self.buf.skip(self.size)?;
        self.kind.unpad(self.buf)?;
        Ok(())
    }

    /// Encode a value into the pod.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, TypedPod};
    ///
    /// let mut pod = pod::array();
    /// pod.as_mut().push(10i32)?;
    ///
    /// let pod = pod.as_ref().into_typed()?;
    /// assert_eq!(pod.as_ref().next::<i32>()?, 10i32);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn next<T>(mut self) -> Result<T, Error>
    where
        T: Decode<'de>,
    {
        if T::TYPE != self.ty {
            return Err(Error::new(ErrorKind::Expected {
                expected: T::TYPE,
                actual: self.ty,
            }));
        }

        let value = T::read_content(self.buf.borrow_mut(), self.size)?;
        self.kind.unpad(self.buf)?;
        Ok(value)
    }

    /// Read the next unsized value into the pod.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, TypedPod};
    ///
    /// let mut pod = pod::array();
    /// pod.as_mut().push_unsized(&b"hello world"[..])?;
    ///
    /// let pod = pod.as_ref().into_typed()?;
    /// assert_eq!(pod.visit_unsized(<[u8]>::to_owned)?, b"hello world");
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn visit_unsized<T, V>(mut self, visitor: V) -> Result<V::Ok, Error>
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

        let value = T::read_content(self.buf.borrow_mut(), self.size, visitor)?;
        self.kind.unpad(self.buf)?;
        Ok(value)
    }

    /// Read the next unsized value into the pod.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, TypedPod};
    ///
    /// let mut pod = pod::array();
    /// pod.as_mut().push_unsized(&b"hello world"[..])?;
    ///
    /// let pod = pod.as_ref().into_typed()?;
    /// assert_eq!(pod.next_unsized::<[u8]>()?, b"hello world");
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn next_unsized<T>(mut self) -> Result<&'de T, Error>
    where
        T: ?Sized + DecodeUnsized<'de>,
    {
        if T::TYPE != self.ty {
            return Err(Error::new(ErrorKind::Expected {
                expected: T::TYPE,
                actual: self.ty,
            }));
        }

        let value = T::read_borrowed(self.buf.borrow_mut(), self.size)?;
        self.kind.unpad(self.buf)?;
        Ok(value)
    }

    /// Read the next optional value.
    ///
    /// This returns `None` if the encoded value is `None`, otherwise a pod
    /// for the value is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, TypedPod};
    ///
    /// let mut pod = pod::array();
    /// pod.as_mut().push_none()?;
    ///
    /// let pod = pod.as_ref().into_typed()?;
    /// assert!(pod.next_option()?.is_none());
    ///
    /// let mut pod = pod::array();
    /// pod.as_mut().push(true)?;
    ///
    /// let pod = pod.as_ref().into_typed()?;
    ///
    /// let Some(mut pod) = pod.next_option()? else {
    ///     panic!("expected some value");
    /// };
    ///
    /// assert!(pod.as_ref().next::<bool>()?);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn next_option(self) -> Result<Option<TypedPod<B, P>>, Error> {
        match self.ty {
            Type::NONE => Ok(None),
            _ => Ok(Some(TypedPod::with_kind(
                self.buf, self.size, self.ty, self.kind,
            ))),
        }
    }

    /// Read the next array.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, TypedPod, Type};
    ///
    /// let mut pod = pod::array();
    ///
    /// pod.as_mut().push_array(Type::INT, |array| {
    ///     array.child().push(1i32)?;
    ///     array.child().push(2i32)?;
    ///     array.child().push(3i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let pod = pod.as_ref().into_typed()?;
    /// let mut array = pod.next_array()?;
    ///
    /// assert!(!array.is_empty());
    /// assert_eq!(array.len(), 3);
    ///
    /// assert_eq!(array.next().unwrap().next::<i32>()?, 1i32);
    /// assert_eq!(array.next().unwrap().next::<i32>()?, 2i32);
    /// assert_eq!(array.next().unwrap().next::<i32>()?, 3i32);
    ///
    /// assert!(array.is_empty());
    /// assert_eq!(array.len(), 0);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn next_array(self) -> Result<Array<B::Split>, Error> {
        match self.ty {
            Type::ARRAY => Array::from_reader(self.split_buffer()?),
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
    /// let mut pod = pod::array();
    /// pod.as_mut().push_struct(|st| {
    ///     st.field().push(1i32)?;
    ///     st.field().push(2i32)?;
    ///     st.field().push(3i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let mut st = pod.as_ref().into_typed()?.next_struct()?;
    /// assert!(!st.is_empty());
    /// assert_eq!(st.field()?.next::<i32>()?, 1i32);
    /// assert_eq!(st.field()?.next::<i32>()?, 2i32);
    /// assert_eq!(st.field()?.next::<i32>()?, 3i32);
    /// assert!(st.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn next_struct(self) -> Result<Struct<B::Split>, Error> {
        match self.ty {
            Type::STRUCT => Ok(Struct::new(self.split_buffer()?)),
            _ => Err(Error::new(ErrorKind::Expected {
                expected: Type::STRUCT,
                actual: self.ty,
            })),
        }
    }

    /// Read the next object.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = pod::array();
    /// pod.as_mut().push_object(10, 20, |obj| {
    ///     obj.property(1).flags(0b001).push(1i32)?;
    ///     obj.property(2).flags(0b010).push(2i32)?;
    ///     obj.property(3).flags(0b100).push(3i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let mut obj = pod.as_ref().next_object()?;
    /// assert!(!obj.is_empty());
    ///
    /// let p = obj.property()?;
    /// assert_eq!(p.key(), 1);
    /// assert_eq!(p.flags(), 0b001);
    /// assert_eq!(p.value().next::<i32>()?, 1);
    ///
    /// let p = obj.property()?;
    /// assert_eq!(p.key(), 2);
    /// assert_eq!(p.flags(), 0b010);
    /// assert_eq!(p.value().next::<i32>()?, 2);
    ///
    /// let p = obj.property()?;
    /// assert_eq!(p.key(), 3);
    /// assert_eq!(p.flags(), 0b100);
    /// assert_eq!(p.value().next::<i32>()?, 3);
    ///
    /// assert!(obj.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn next_object(self) -> Result<Object<B::Split>, Error> {
        match self.ty {
            Type::OBJECT => Object::from_reader(self.split_buffer()?),
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
    /// let mut pod = pod::array();
    /// pod.as_mut().push_sequence(|seq| {
    ///     seq.control().offset(1).ty(10).push(1i32)?;
    ///     seq.control().offset(2).ty(20).push(2i32)?;
    ///     seq.control().offset(3).ty(30).push(3i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let mut seq = pod.as_ref().into_typed()?.next_sequence()?;
    /// assert!(!seq.is_empty());
    ///
    /// let c = seq.control()?;
    /// assert_eq!(c.offset(), 1);
    /// assert_eq!(c.ty(), 10);
    /// assert_eq!(c.value().next::<i32>()?, 1);
    ///
    /// let c = seq.control()?;
    /// assert_eq!(c.offset(), 2);
    /// assert_eq!(c.ty(), 20);
    /// assert_eq!(c.value().next::<i32>()?, 2);
    ///
    /// let c = seq.control()?;
    /// assert_eq!(c.offset(), 3);
    /// assert_eq!(c.ty(), 30);
    /// assert_eq!(c.value().next::<i32>()?, 3);
    ///
    /// assert!(seq.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn next_sequence(self) -> Result<Sequence<B::Split>, Error> {
        match self.ty {
            Type::SEQUENCE => Sequence::from_reader(self.split_buffer()?),
            _ => Err(Error::new(ErrorKind::Expected {
                expected: Type::SEQUENCE,
                actual: self.ty,
            })),
        }
    }

    /// Decode a choice.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ChoiceType, Pod, Type};
    ///
    /// let mut pod = pod::array();
    /// pod.as_mut().push_choice(ChoiceType::RANGE, Type::INT, |choice| {
    ///     choice.child().push(10i32)?;
    ///     choice.child().push(0i32)?;
    ///     choice.child().push(30i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let mut choice = pod.as_ref().next_choice()?;
    /// assert!(!choice.is_empty());
    /// assert_eq!(choice.next().unwrap().next::<i32>()?, 10);
    /// assert_eq!(choice.next().unwrap().next::<i32>()?, 0);
    /// assert_eq!(choice.next().unwrap().next::<i32>()?, 30);
    /// assert!(choice.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn next_choice(self) -> Result<Choice<B::Split>, Error> {
        match self.ty {
            Type::CHOICE => Choice::from_reader(self.split_buffer()?),
            _ => Err(Error::new(ErrorKind::Expected {
                expected: Type::CHOICE,
                actual: self.ty,
            })),
        }
    }

    /// Decode a nested pod.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, TypedPod};
    ///
    /// let mut pod = pod::array();
    /// pod.as_mut().push_pod(|pod| {
    ///     pod.as_mut().push_struct(|st| {
    ///         st.field().push(1i32)?;
    ///         st.field().push(2i32)?;
    ///         st.field().push(3i32)?;
    ///         Ok(())
    ///     })
    /// })?;
    ///
    /// let pod = pod.as_ref().into_typed()?.next_pod()?;
    /// let mut st = pod.next_struct()?;
    /// assert!(!st.is_empty());
    /// assert_eq!(st.field()?.next::<i32>()?, 1i32);
    /// assert_eq!(st.field()?.next::<i32>()?, 2i32);
    /// assert_eq!(st.field()?.next::<i32>()?, 3i32);
    /// assert!(st.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn next_pod(self) -> Result<Pod<B::Split, PackedPod>, Error> {
        match self.ty {
            Type::POD => Ok(Pod::packed(self.split_buffer()?)),
            _ => Err(Error::new(ErrorKind::Expected {
                expected: Type::POD,
                actual: self.ty,
            })),
        }
    }

    fn split_buffer(mut self) -> Result<B::Split, Error> {
        let Some(buf) = self.buf.split(self.size) else {
            return Err(Error::new(ErrorKind::BufferUnderflow));
        };

        self.kind.unpad(self.buf)?;
        Ok(buf)
    }
}

impl<B, P> TypedPod<B, P>
where
    B: AsReader,
    P: Copy,
{
    /// Coerce any pod into an owned pod.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut pod = pod::array();
    /// pod.as_mut().push(10i32)?;
    ///
    /// let pod = pod.as_ref().into_typed()?.to_owned()?;
    ///
    /// assert_eq!(pod.as_ref().next::<i32>()?, 10i32);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[cfg(feature = "alloc")]
    pub fn to_owned(&self) -> Result<TypedPod<DynamicBuf, P>, AllocError> {
        Ok(TypedPod {
            buf: DynamicBuf::from_slice(self.buf.as_reader().as_bytes())?,
            size: self.size,
            ty: self.ty,
            kind: self.kind,
        })
    }

    /// Convert the [`TypedPod`] into a one borrowing from but without modifying
    /// the current buffer.
    #[inline]
    pub fn as_ref(&self) -> TypedPod<B::AsReader<'_>, P> {
        TypedPod::with_kind(self.buf.as_reader(), self.size, self.ty, self.kind)
    }
}

impl<B, P> Clone for TypedPod<B, P>
where
    B: Clone,
    P: Copy,
{
    #[inline]
    fn clone(&self) -> Self {
        TypedPod {
            size: self.size,
            ty: self.ty,
            buf: self.buf.clone(),
            kind: self.kind,
        }
    }
}

/// [`Encode`] implementation for [`TypedPod`].
///
/// # Examples
///
/// ```
/// use pod::{Pod, Type};
///
/// let mut pod = pod::array();
/// pod.as_mut().push_object(10, 20, |obj| {
///     obj.property(1).flags(0b001).push(1i32)?;
///     obj.property(2).flags(0b010).push(2i32)?;
///     obj.property(3).flags(0b100).push(3i32)?;
///     Ok(())
/// })?;
///
/// let mut pod2 = pod::array();
/// pod2.as_mut().encode(pod.as_ref().into_typed()?)?;
///
/// let mut obj = pod2.as_ref().next_pod()?.next_object()?;
/// assert!(!obj.is_empty());
///
/// let p = obj.property()?;
/// assert_eq!(p.key(), 1);
/// assert_eq!(p.flags(), 0b001);
/// assert_eq!(p.value().next::<i32>()?, 1);
///
/// let p = obj.property()?;
/// assert_eq!(p.key(), 2);
/// assert_eq!(p.flags(), 0b010);
/// assert_eq!(p.value().next::<i32>()?, 2);
///
/// let p = obj.property()?;
/// assert_eq!(p.key(), 3);
/// assert_eq!(p.flags(), 0b100);
/// assert_eq!(p.value().next::<i32>()?, 3);
///
/// assert!(obj.is_empty());
/// # Ok::<_, pod::Error>(())
/// ```
impl<B, P> EncodeUnsized for TypedPod<B, P>
where
    B: AsReader,
    P: ReadPod,
{
    const TYPE: Type = Type::POD;

    #[inline]
    fn size(&self) -> Option<usize> {
        let len = self.buf.as_reader().len();
        len.checked_add(mem::size_of::<[u32; 2]>())
    }

    #[inline]
    fn write_content(&self, mut writer: impl Writer) -> Result<(), Error> {
        let Ok(size) = u32::try_from(self.size) else {
            return Err(Error::new(ErrorKind::SizeOverflow));
        };

        writer.write(&[size, self.ty.into_u32()])?;
        writer.write(self.buf.as_reader().as_bytes())?;
        Ok(())
    }
}

crate::macros::encode_into_unsized!(impl [B, P] TypedPod<B, P> where B: AsReader, P: ReadPod);

impl<B, P> fmt::Debug for TypedPod<B, P>
where
    B: AsReader,
    P: ReadPod,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        macro_rules! tri {
            ($expr:expr) => {
                match $expr {
                    Ok(value) => value,
                    Err(e) => return e.fmt(f),
                }
            };
        }

        macro_rules! decode {
            ($ty:ty, $pat:pat => $expr:expr) => {
                match self.as_ref().next::<$ty>() {
                    Ok($pat) => $expr,
                    Err(e) => e.fmt(f),
                }
            };
        }

        macro_rules! visit_unsized {
            ($ty:ty, $pat:pat => $expr:expr) => {{
                let mut outer = Ok(());

                let result = self.as_ref().visit_unsized(|$pat: &$ty| {
                    outer = $expr;
                });

                if let Err(e) = result {
                    return e.fmt(f);
                }

                outer
            }};
        }

        match self.ty {
            Type::NONE => f.write_str("None"),
            Type::BOOL => {
                decode!(bool, value => write!(f, "{value:?}"))
            }
            Type::ID => {
                decode!(Id<u32>, value => write!(f, "{value:?}"))
            }
            Type::INT => {
                decode!(i32, value => write!(f, "{value:?}"))
            }
            Type::LONG => {
                decode!(i64, value => write!(f, "{value:?}"))
            }
            Type::FLOAT => {
                decode!(f32, value => write!(f, "{value:?}"))
            }
            Type::DOUBLE => {
                decode!(f64, value => write!(f, "{value:?}"))
            }
            Type::STRING => {
                visit_unsized!(CStr, value => write!(f, "{value:?}"))
            }
            Type::BYTES => {
                visit_unsized!([u8], value => write!(f, "{:?}", BStr::new(value)))
            }
            Type::RECTANGLE => {
                decode!(Rectangle, value => value.fmt(f))
            }
            Type::FRACTION => {
                decode!(Fraction, value => value.fmt(f))
            }
            Type::BITMAP => {
                visit_unsized!(Bitmap, value => value.fmt(f))
            }
            Type::ARRAY => tri!(self.as_ref().next_array()).fmt(f),
            Type::STRUCT => tri!(self.as_ref().next_struct()).fmt(f),
            Type::OBJECT => tri!(self.as_ref().next_object()).fmt(f),
            Type::SEQUENCE => tri!(self.as_ref().next_sequence()).fmt(f),
            Type::POINTER => decode!(Pointer, value => value.fmt(f)),
            Type::FD => decode!(Fd, value => value.fmt(f)),
            Type::CHOICE => tri!(self.as_ref().next_choice()).fmt(f),
            Type::POD => tri!(tri!(self.as_ref().next_pod()).into_typed()).fmt(f),
            ty => write!(f, "{{{ty:?}}}"),
        }
    }
}
