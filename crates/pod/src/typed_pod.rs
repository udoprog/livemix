use core::ffi::CStr;
use core::fmt;
use core::mem;

use crate::ChoiceType;
#[cfg(feature = "alloc")]
use crate::DynamicBuf;
use crate::PodStream;
use crate::Readable;
use crate::bstr::BStr;
#[cfg(feature = "alloc")]
use crate::buf::AllocError;
use crate::error::ErrorKind;
use crate::read::{Array, Choice, Object, Sequence, Struct};
use crate::{
    AsSlice, Bitmap, Error, Fd, Fraction, Id, PackedPod, PaddedPod, Pod, PodItem, Pointer, ReadPod,
    Reader, Rectangle, SizedReadable, Slice, Type, UnsizedReadable, UnsizedWritable, Visitor,
    Writer,
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
    /// pod.as_mut().write(10i32)?;
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
    /// pod.as_mut().write(10i32)?;
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
    P: Copy,
{
    /// Construct a new [`TypedPod`] by reading and advancing the given buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, TypedPod};
    ///
    /// let mut pod = pod::array();
    /// pod.as_mut().write(10i32)?;
    ///
    /// let pod = pod.as_ref().into_typed()?;
    /// assert_eq!(pod.as_ref().read_sized::<i32>()?, 10i32);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn from_reader(mut buf: B, kind: P) -> Result<Self, Error> {
        let (size, ty) = buf.header()?;

        Ok(TypedPod {
            buf,
            size,
            ty,
            kind,
        })
    }

    /// Convert the [`TypedPod`] into a one borrowing from mutably.
    #[inline]
    pub fn as_mut(&mut self) -> TypedPod<B::Mut<'_>, P> {
        TypedPod::with_kind(self.buf.borrow_mut(), self.size, self.ty, self.kind)
    }
}

impl<'de, B, P> TypedPod<B, P>
where
    B: Reader<'de>,
    P: ReadPod,
{
    /// Skip a value in the pod.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut pod = pod::array();
    ///
    /// pod.as_mut().write((1, 2, "hello world", 4));
    ///
    /// let mut pod = pod.as_ref();
    /// assert_eq!(pod.as_mut().into_typed()?.read_sized::<i32>()?, 1);
    /// assert_eq!(pod.as_mut().into_typed()?.read_sized::<i32>()?, 2);
    /// assert_eq!(pod.as_mut().into_typed()?.skip()?, 12);
    /// assert_eq!(pod.as_mut().into_typed()?.read_sized::<i32>()?, 4);
    /// assert!(pod.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn skip(mut self) -> Result<usize, Error> {
        self.buf.skip(self.size)?;
        self.kind.unpad(self.buf)?;
        Ok(self.size)
    }

    /// Conveniently decode a value from the pod.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut pod = pod::array();
    /// pod.as_mut().write((10i32, "hello world", [1u32, 2u32]))?;
    ///
    /// let (a, s, [c, d]) = pod.as_ref().into_typed()?.read::<(i32, String, [u32; 2])>()?;
    ///
    /// assert_eq!(a, 10i32);
    /// assert_eq!(s, "hello world");
    /// assert_eq!(c, 1u32);
    /// assert_eq!(d, 2u32);
    /// # Ok::<_, pod::Error>(())
    /// ```
    pub fn read<T>(mut self) -> Result<T, Error>
    where
        T: Readable<'de>,
    {
        T::read_from(&mut self)
    }

    /// Read a value.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, TypedPod};
    ///
    /// let mut pod = pod::array();
    /// pod.as_mut().write(10i32)?;
    ///
    /// let pod = pod.as_ref().into_typed()?;
    /// assert_eq!(pod.as_ref().read_sized::<i32>()?, 10i32);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn read_sized<T>(mut self) -> Result<T, Error>
    where
        T: SizedReadable<'de>,
    {
        let value = match self.ty {
            Type::CHOICE => {
                let pod = TypedPod::packed(self.buf.borrow_mut(), self.size, self.ty);
                let mut choice = pod.read_choice()?;

                if choice.choice_type() != ChoiceType::NONE {
                    return Err(Error::new(ErrorKind::InvalidChoiceType {
                        ty: Type::INT,
                        expected: ChoiceType::NONE,
                        actual: choice.choice_type(),
                    }));
                }

                let Some(choice) = choice.next() else {
                    return Err(Error::new(ErrorKind::BufferUnderflow));
                };

                choice.read_sized()?
            }
            _ => T::read_content(self.buf.borrow_mut(), self.ty, self.size)?,
        };

        self.kind.unpad(self.buf)?;
        Ok(value)
    }

    /// Read the next unsized value.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, TypedPod};
    ///
    /// let mut pod = pod::array();
    /// pod.as_mut().write_unsized(&b"hello world"[..])?;
    ///
    /// let pod = pod.as_ref().into_typed()?;
    /// assert_eq!(pod.visit_unsized(<[u8]>::to_owned)?, b"hello world");
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn visit_unsized<T, V>(mut self, visitor: V) -> Result<V::Ok, Error>
    where
        T: ?Sized + UnsizedReadable<'de>,
        V: Visitor<'de, T>,
    {
        if T::TYPE != self.ty {
            return Err(Error::expected(T::TYPE, self.ty, self.size));
        }

        let value = T::read_content(self.buf.borrow_mut(), self.size, visitor)?;
        self.kind.unpad(self.buf)?;
        Ok(value)
    }

    /// Read the next unsized value.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, TypedPod};
    ///
    /// let mut pod = pod::array();
    /// pod.as_mut().write_unsized(&b"hello world"[..])?;
    ///
    /// let pod = pod.as_ref().into_typed()?;
    /// assert_eq!(pod.read_unsized::<[u8]>()?, b"hello world");
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn read_unsized<T>(mut self) -> Result<&'de T, Error>
    where
        T: ?Sized + UnsizedReadable<'de>,
    {
        if T::TYPE != self.ty {
            return Err(Error::expected(T::TYPE, self.ty, self.size));
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
    /// pod.as_mut().write_none()?;
    ///
    /// let pod = pod.as_ref().into_typed()?;
    /// assert!(pod.read_option()?.is_none());
    ///
    /// let mut pod = pod::array();
    /// pod.as_mut().write(true)?;
    ///
    /// let pod = pod.as_ref().into_typed()?;
    ///
    /// let Some(mut pod) = pod.read_option()? else {
    ///     panic!("expected some value");
    /// };
    ///
    /// assert!(pod.as_ref().read_sized::<bool>()?);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn read_option(self) -> Result<Option<TypedPod<B, P>>, Error> {
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
    /// pod.as_mut().write_array(Type::INT, |array| {
    ///     array.child().write(1i32)?;
    ///     array.child().write(2i32)?;
    ///     array.child().write(3i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let pod = pod.as_ref().into_typed()?;
    /// let mut array = pod.read_array()?;
    ///
    /// assert!(!array.is_empty());
    /// assert_eq!(array.len(), 3);
    ///
    /// assert_eq!(array.next().unwrap().read_sized::<i32>()?, 1i32);
    /// assert_eq!(array.next().unwrap().read_sized::<i32>()?, 2i32);
    /// assert_eq!(array.next().unwrap().read_sized::<i32>()?, 3i32);
    ///
    /// assert!(array.is_empty());
    /// assert_eq!(array.len(), 0);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn read_array(self) -> Result<Array<Slice<'de>>, Error> {
        match self.ty {
            Type::ARRAY => Array::from_reader(self.split()?),
            _ => Err(Error::expected(Type::ARRAY, self.ty, self.size)),
        }
    }

    /// Read a struct.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, TypedPod};
    ///
    /// let mut pod = pod::array();
    /// pod.as_mut().write_struct(|st| {
    ///     st.field().write(1i32)?;
    ///     st.field().write(2i32)?;
    ///     st.field().write(3i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let mut st = pod.as_ref().into_typed()?.read_struct()?;
    /// assert!(!st.is_empty());
    /// assert_eq!(st.field()?.read_sized::<i32>()?, 1i32);
    /// assert_eq!(st.field()?.read_sized::<i32>()?, 2i32);
    /// assert_eq!(st.field()?.read_sized::<i32>()?, 3i32);
    /// assert!(st.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn read_struct(self) -> Result<Struct<Slice<'de>>, Error> {
        match self.ty {
            Type::STRUCT => Ok(Struct::new(self.split()?)),
            _ => Err(Error::expected(Type::STRUCT, self.ty, self.size)),
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
    /// pod.as_mut().write_object(10, 20, |obj| {
    ///     obj.property(1).flags(0b001).write(1i32)?;
    ///     obj.property(2).flags(0b010).write(2i32)?;
    ///     obj.property(3).flags(0b100).write(3i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let mut obj = pod.as_ref().read_object()?;
    /// assert!(!obj.is_empty());
    ///
    /// let p = obj.property()?;
    /// assert_eq!(p.key(), 1);
    /// assert_eq!(p.flags(), 0b001);
    /// assert_eq!(p.value().read_sized::<i32>()?, 1);
    ///
    /// let p = obj.property()?;
    /// assert_eq!(p.key(), 2);
    /// assert_eq!(p.flags(), 0b010);
    /// assert_eq!(p.value().read_sized::<i32>()?, 2);
    ///
    /// let p = obj.property()?;
    /// assert_eq!(p.key(), 3);
    /// assert_eq!(p.flags(), 0b100);
    /// assert_eq!(p.value().read_sized::<i32>()?, 3);
    ///
    /// assert!(obj.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn read_object(self) -> Result<Object<Slice<'de>>, Error> {
        match self.ty {
            Type::OBJECT => Object::from_reader(self.split()?),
            _ => Err(Error::expected(Type::OBJECT, self.ty, self.size)),
        }
    }

    /// Read a sequence.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, TypedPod, Type};
    ///
    /// let mut pod = pod::array();
    /// pod.as_mut().write_sequence(|seq| {
    ///     seq.control().offset(1).ty(10).write(1i32)?;
    ///     seq.control().offset(2).ty(20).write(2i32)?;
    ///     seq.control().offset(3).ty(30).write(3i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let mut seq = pod.as_ref().into_typed()?.read_sequence()?;
    /// assert!(!seq.is_empty());
    ///
    /// let c = seq.control()?;
    /// assert_eq!(c.offset(), 1);
    /// assert_eq!(c.ty(), 10);
    /// assert_eq!(c.value().read_sized::<i32>()?, 1);
    ///
    /// let c = seq.control()?;
    /// assert_eq!(c.offset(), 2);
    /// assert_eq!(c.ty(), 20);
    /// assert_eq!(c.value().read_sized::<i32>()?, 2);
    ///
    /// let c = seq.control()?;
    /// assert_eq!(c.offset(), 3);
    /// assert_eq!(c.ty(), 30);
    /// assert_eq!(c.value().read_sized::<i32>()?, 3);
    ///
    /// assert!(seq.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn read_sequence(self) -> Result<Sequence<Slice<'de>>, Error> {
        match self.ty {
            Type::SEQUENCE => Sequence::from_reader(self.split()?),
            _ => Err(Error::expected(Type::SEQUENCE, self.ty, self.size)),
        }
    }

    /// Read a choice.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ChoiceType, Pod, Type};
    ///
    /// let mut pod = pod::array();
    /// pod.as_mut().write_choice(ChoiceType::RANGE, Type::INT, |choice| {
    ///     choice.child().write(10i32)?;
    ///     choice.child().write(0i32)?;
    ///     choice.child().write(30i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let mut choice = pod.as_ref().read_choice()?;
    /// assert!(!choice.is_empty());
    /// assert_eq!(choice.next().unwrap().read_sized::<i32>()?, 10);
    /// assert_eq!(choice.next().unwrap().read_sized::<i32>()?, 0);
    /// assert_eq!(choice.next().unwrap().read_sized::<i32>()?, 30);
    /// assert!(choice.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn read_choice(self) -> Result<Choice<Slice<'de>>, Error> {
        match self.ty {
            Type::CHOICE => Choice::from_reader(self.split()?),
            _ => Err(Error::expected(Type::CHOICE, self.ty, self.size)),
        }
    }

    /// Read a nested pod.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, TypedPod};
    ///
    /// let mut pod = pod::array();
    /// pod.as_mut().write_pod(|pod| {
    ///     pod.as_mut().write_struct(|st| st.write((1, 2, 3)))
    /// })?;
    ///
    /// let pod = pod.as_ref().into_typed()?.read_pod()?;
    /// let mut st = pod.read_struct()?;
    /// assert_eq!(st.read::<(i32, i32, i32)>()?, (1, 2, 3));
    /// assert!(st.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn read_pod(self) -> Result<Pod<Slice<'de>, PackedPod>, Error> {
        match self.ty {
            Type::POD => Ok(Pod::packed(self.split()?)),
            _ => Err(Error::expected(Type::POD, self.ty, self.size)),
        }
    }

    fn split(mut self) -> Result<Slice<'de>, Error> {
        let Some(buf) = self.buf.split(self.size) else {
            return Err(Error::new(ErrorKind::BufferUnderflow));
        };

        self.kind.unpad(self.buf)?;
        Ok(buf)
    }
}

impl<B, P> TypedPod<B, P>
where
    B: AsSlice,
{
    /// Test if the typed pod is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    /// let mut pod = pod::array();
    ///
    /// pod.as_mut().write(1);
    ///
    /// let mut pod = pod.as_ref().into_typed()?;
    /// assert!(!pod.is_empty());
    /// assert_eq!(pod.as_mut().read_sized::<i32>()?, 1);
    /// assert!(pod.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    pub fn is_empty(&self) -> bool {
        self.buf.as_slice().is_empty()
    }
}

impl<B, P> TypedPod<B, P>
where
    B: AsSlice,
    P: Copy,
{
    /// Coerce any pod into an owned pod.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut pod = pod::array();
    /// pod.as_mut().write(10i32)?;
    ///
    /// let pod = pod.as_ref().into_typed()?.to_owned()?;
    ///
    /// assert_eq!(pod.as_ref().read_sized::<i32>()?, 10i32);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[cfg(feature = "alloc")]
    pub fn to_owned(&self) -> Result<TypedPod<DynamicBuf, P>, AllocError> {
        Ok(TypedPod {
            buf: DynamicBuf::from_slice(self.buf.as_slice().as_bytes())?,
            size: self.size,
            ty: self.ty,
            kind: self.kind,
        })
    }

    /// Convert the [`TypedPod`] into a one borrowing from but without modifying
    /// the current buffer.
    #[inline]
    pub fn as_ref(&self) -> TypedPod<Slice<'_>, P> {
        TypedPod::with_kind(self.buf.as_slice(), self.size, self.ty, self.kind)
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

impl<'de, B, P> PodStream<'de> for TypedPod<B, P>
where
    B: Reader<'de>,
    P: ReadPod,
{
    type Item = TypedPod<Slice<'de>, PackedPod>;

    #[inline]
    fn next(&mut self) -> Result<TypedPod<Slice<'de>, PackedPod>, Error> {
        let Some(buf) = self.buf.split(self.size) else {
            return Err(Error::new(ErrorKind::BufferUnderflow));
        };

        self.kind.unpad(self.buf.borrow_mut())?;
        Ok(TypedPod::packed(buf, self.size, self.ty))
    }
}

/// [`UnsizedWritable`] implementation for [`TypedPod`].
///
/// # Examples
///
/// ```
/// use pod::{Pod, Type};
///
/// let mut pod = pod::array();
/// pod.as_mut().write_object(10, 20, |obj| {
///     obj.property(1).flags(0b001).write(1i32)?;
///     obj.property(2).flags(0b010).write(2i32)?;
///     obj.property(3).flags(0b100).write(3i32)?;
///     Ok(())
/// })?;
///
/// let mut pod2 = pod::array();
/// pod2.as_mut().write(pod.as_ref().into_typed()?)?;
///
/// let mut obj = pod2.as_ref().read_pod()?.read_object()?;
/// assert!(!obj.is_empty());
///
/// let p = obj.property()?;
/// assert_eq!(p.key(), 1);
/// assert_eq!(p.flags(), 0b001);
/// assert_eq!(p.value().read_sized::<i32>()?, 1);
///
/// let p = obj.property()?;
/// assert_eq!(p.key(), 2);
/// assert_eq!(p.flags(), 0b010);
/// assert_eq!(p.value().read_sized::<i32>()?, 2);
///
/// let p = obj.property()?;
/// assert_eq!(p.key(), 3);
/// assert_eq!(p.flags(), 0b100);
/// assert_eq!(p.value().read_sized::<i32>()?, 3);
///
/// assert!(obj.is_empty());
/// # Ok::<_, pod::Error>(())
/// ```
impl<B, P> UnsizedWritable for TypedPod<B, P>
where
    B: AsSlice,
    P: ReadPod,
{
    const TYPE: Type = Type::POD;

    #[inline]
    fn size(&self) -> Option<usize> {
        let len = self.buf.as_slice().len();
        len.checked_add(mem::size_of::<[u32; 2]>())
    }

    #[inline]
    fn write_unsized(&self, mut writer: impl Writer) -> Result<(), Error> {
        let Ok(size) = u32::try_from(self.size) else {
            return Err(Error::new(ErrorKind::SizeOverflow));
        };

        writer.write(&[size, self.ty.into_u32()])?;
        writer.write(self.buf.as_slice().as_bytes())?;
        Ok(())
    }
}

crate::macros::encode_into_unsized!(impl [B, P] TypedPod<B, P> where B: AsSlice, P: ReadPod);

impl<'de> PodItem<'de> for TypedPod<Slice<'de>, PackedPod> {
    #[inline]
    fn read<T>(self) -> Result<T, Error>
    where
        T: Readable<'de>,
    {
        TypedPod::read(self)
    }

    #[inline]
    fn read_sized<T>(self) -> Result<T, Error>
    where
        T: SizedReadable<'de>,
    {
        TypedPod::read_sized(self)
    }

    #[inline]
    fn read_unsized<T>(self) -> Result<&'de T, Error>
    where
        T: ?Sized + UnsizedReadable<'de>,
    {
        TypedPod::read_unsized(self)
    }

    #[inline]
    fn read_struct(self) -> Result<Struct<Slice<'de>>, Error> {
        TypedPod::read_struct(self)
    }

    #[inline]
    fn read_object(self) -> Result<Object<Slice<'de>>, Error> {
        TypedPod::read_object(self)
    }

    #[inline]
    fn read_option(self) -> Result<Option<Self>, Error> {
        TypedPod::read_option(self)
    }
}

impl<B, P> fmt::Debug for TypedPod<B, P>
where
    B: AsSlice,
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
                match self.as_ref().read_sized::<$ty>() {
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
            Type::ARRAY => tri!(self.as_ref().read_array()).fmt(f),
            Type::STRUCT => tri!(self.as_ref().read_struct()).fmt(f),
            Type::OBJECT => tri!(self.as_ref().read_object()).fmt(f),
            Type::SEQUENCE => tri!(self.as_ref().read_sequence()).fmt(f),
            Type::POINTER => decode!(Pointer, value => value.fmt(f)),
            Type::FD => decode!(Fd, value => value.fmt(f)),
            Type::CHOICE => tri!(self.as_ref().read_choice()).fmt(f),
            Type::POD => tri!(tri!(self.as_ref().read_pod()).into_typed()).fmt(f),
            ty => write!(f, "{{{ty:?}}}"),
        }
    }
}
