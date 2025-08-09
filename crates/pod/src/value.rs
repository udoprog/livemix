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
use crate::read::{Array, Choice, Object, Sequence, Struct};
use crate::utils;
use crate::{
    AsSlice, Bitmap, BufferUnderflow, Error, ErrorKind, Fd, Fraction, Id, PackedPod, Pod, PodItem,
    Pointer, Reader, Rectangle, SizedReadable, Slice, Type, UnsizedReadable, UnsizedWritable,
    Visitor, Writer,
};

/// A value inside of a [`Pod`].
///
/// This is a wrapper that can be used for encoding and decoding data.
pub struct Value<B> {
    buf: B,
    size: usize,
    ty: Type,
}

impl<B> Value<B> {
    /// Construct a new [`Value`] arround the specified buffer `B` and
    /// a [`PackedPod`] kind.
    #[inline]
    pub(crate) const fn new(buf: B, size: usize, ty: Type) -> Self {
        Self { buf, size, ty }
    }
}

impl<B> Value<B> {
    /// Get the type of the pod.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::Type;
    ///
    /// let mut pod = pod::array();
    /// pod.as_mut().write(10i32)?;
    ///
    /// let pod = pod.as_ref().into_value()?;
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
    /// use pod::Type;
    ///
    /// let mut pod = pod::array();
    /// pod.as_mut().write(10i32)?;
    ///
    /// let pod = pod.as_ref().into_value()?;
    /// assert_eq!(pod.ty(), Type::INT);
    /// assert_eq!(pod.size(), 4);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub const fn size(&self) -> usize {
        self.size
    }
}

impl<'de> Value<Slice<'de>> {
    /// Construct a new [`Value`] by reading and advancing the given buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut pod = pod::array();
    /// pod.as_mut().write(10i32)?;
    ///
    /// let pod = pod.as_ref().into_value()?;
    /// assert_eq!(pod.as_ref().read_sized::<i32>()?, 10i32);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn from_reader<B>(mut buf: B) -> Result<(Self, B), Error>
    where
        B: Reader<'de>,
    {
        let (size, ty) = buf.header()?;
        let slice = buf.split(size).ok_or(BufferUnderflow)?;

        let pod = Value {
            buf: slice,
            size,
            ty,
        };

        Ok((pod, buf))
    }
}

impl<'de, B> Value<B>
where
    B: Reader<'de>,
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
    /// assert_eq!(pod.as_mut().into_value()?.read_sized::<i32>()?, 1);
    /// assert_eq!(pod.as_mut().into_value()?.read_sized::<i32>()?, 2);
    /// assert_eq!(pod.as_mut().into_value()?.skip()?, 12);
    /// assert_eq!(pod.as_mut().into_value()?.read_sized::<i32>()?, 4);
    /// assert!(pod.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn skip(mut self) -> Result<usize, Error> {
        self.buf.skip(self.size)?;
        Ok(self.size)
    }

    /// Conveniently decode a value from the pod.
    ///
    /// Note that typed pods in contrast to [`Pod`] only every contain a single
    /// value, the type and size of which is known. Attempting to convert into a
    /// typed pod and read multiple values will cause all subsequent values read
    /// to be the equivalent of [`Type::None`].
    ///
    /// ```
    /// let mut pod = pod::array();
    /// pod.as_mut().write((10i32, 20u32))?;
    ///
    /// let mut pod = pod.as_ref();
    ///
    /// assert_eq!(pod.into_value()?.read::<(i32, Option<i32>)>()?, (10, None));
    /// # Ok::<_, pod::Error>(())
    /// ```
    ///
    /// [`Type::None`]: crate::Type::NONE
    ///
    /// # Examples
    ///
    /// ```
    /// let mut pod = pod::array();
    /// pod.as_mut().write((10i32, "hello world", [1u32, 2]))?;
    ///
    /// let mut pod = pod.as_ref();
    ///
    /// let a = pod.as_mut().into_value()?.read::<i32>()?;
    /// assert_eq!(a, 10i32);
    ///
    /// let s = pod.as_mut().into_value()?.read::<&str>()?;
    /// assert_eq!(s, "hello world");
    ///
    /// let a1 = pod.as_mut().into_value()?.read::<u32>()?;
    /// assert_eq!(a1, 1);
    ///
    /// let a2 = pod.as_mut().into_value()?.read::<u32>()?;
    /// assert_eq!(a2, 2);
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
    /// let mut pod = pod::array();
    /// pod.as_mut().write(10i32)?;
    ///
    /// let pod = pod.as_ref().into_value()?;
    /// assert_eq!(pod.as_ref().read_sized::<i32>()?, 10i32);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn read_sized<T>(self) -> Result<T, Error>
    where
        T: SizedReadable<'de>,
    {
        let value = match self.ty {
            Type::CHOICE => {
                let mut choice = self.read_choice()?;

                if choice.choice_type() != ChoiceType::NONE {
                    return Err(Error::new(ErrorKind::InvalidChoiceType {
                        ty: Type::INT,
                        expected: ChoiceType::NONE,
                        actual: choice.choice_type(),
                    }));
                }

                let choice = choice.next().ok_or(BufferUnderflow)?;
                choice.read_sized()?
            }
            _ => T::read_content(self.buf, self.ty, self.size)?,
        };

        Ok(value)
    }

    /// Read the next unsized value.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut pod = pod::array();
    /// pod.as_mut().write_unsized(&b"hello world"[..])?;
    ///
    /// let pod = pod.as_ref().into_value()?;
    /// assert_eq!(pod.visit_unsized(<[u8]>::to_owned)?, b"hello world");
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn visit_unsized<T, V>(self, visitor: V) -> Result<V::Ok, Error>
    where
        T: ?Sized + UnsizedReadable<'de>,
        V: Visitor<'de, T>,
    {
        if T::TYPE != self.ty {
            return Err(Error::expected(T::TYPE, self.ty, self.size));
        }

        T::read_content(self.buf, self.size, visitor)
    }

    /// Read the next unsized value.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut pod = pod::array();
    /// pod.as_mut().write_unsized(&b"hello world"[..])?;
    ///
    /// let pod = pod.as_ref().into_value()?;
    /// assert_eq!(pod.read_unsized::<[u8]>()?, b"hello world");
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn read_unsized<T>(self) -> Result<&'de T, Error>
    where
        T: ?Sized + UnsizedReadable<'de>,
    {
        if T::TYPE != self.ty {
            return Err(Error::expected(T::TYPE, self.ty, self.size));
        }

        T::read_borrowed(self.buf, self.size)
    }

    /// Read the next optional value.
    ///
    /// This returns `None` if the encoded value is `None`, otherwise a pod
    /// for the value is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut pod = pod::array();
    /// pod.as_mut().write_none()?;
    ///
    /// let pod = pod.as_ref().into_value()?;
    /// assert!(pod.read_option()?.is_none());
    ///
    /// let mut pod = pod::array();
    /// pod.as_mut().write(true)?;
    ///
    /// let pod = pod.as_ref().into_value()?;
    ///
    /// let Some(mut pod) = pod.read_option()? else {
    ///     panic!("expected some value");
    /// };
    ///
    /// assert!(pod.as_ref().read_sized::<bool>()?);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn read_option(self) -> Result<Option<Value<Slice<'de>>>, Error> {
        match self.ty {
            Type::NONE => Ok(None),
            _ => {
                let size = self.size;
                let ty = self.ty;
                Ok(Some(Value::new(self.split()?, size, ty)))
            }
        }
    }

    /// Read the next array.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::Type;
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
    /// let pod = pod.as_ref().into_value()?;
    /// let mut array = pod.read_array()?;
    ///
    /// assert!(!array.is_empty());
    /// assert_eq!(array.len(), 3);
    ///
    /// assert_eq!(array.next()?.unwrap().read_sized::<i32>()?, 1i32);
    /// assert_eq!(array.next()?.unwrap().read_sized::<i32>()?, 2i32);
    /// assert_eq!(array.next()?.unwrap().read_sized::<i32>()?, 3i32);
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
    /// let mut pod = pod::array();
    /// pod.as_mut().write_struct(|st| {
    ///     st.field().write(1i32)?;
    ///     st.field().write(2i32)?;
    ///     st.field().write(3i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let mut st = pod.as_ref().into_value()?.read_struct()?;
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
    /// assert_eq!(p.key::<u32>(), 1);
    /// assert_eq!(p.flags(), 0b001);
    /// assert_eq!(p.value().read_sized::<i32>()?, 1);
    ///
    /// let p = obj.property()?;
    /// assert_eq!(p.key::<u32>(), 2);
    /// assert_eq!(p.flags(), 0b010);
    /// assert_eq!(p.value().read_sized::<i32>()?, 2);
    ///
    /// let p = obj.property()?;
    /// assert_eq!(p.key::<u32>(), 3);
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
    /// let mut pod = pod::array();
    /// pod.as_mut().write_sequence(|seq| {
    ///     seq.control().offset(1).ty(10).write(1i32)?;
    ///     seq.control().offset(2).ty(20).write(2i32)?;
    ///     seq.control().offset(3).ty(30).write(3i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let mut seq = pod.as_ref().into_value()?.read_sequence()?;
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
    /// let mut pod = pod::array();
    /// pod.as_mut().write_pod(|pod| {
    ///     pod.as_mut().write_struct(|st| st.write((1, 2, 3)))
    /// })?;
    ///
    /// let pod = pod.as_ref().into_value()?.read_pod()?;
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

    #[inline]
    fn split(mut self) -> Result<Slice<'de>, BufferUnderflow> {
        self.buf.split(self.size).ok_or(BufferUnderflow)
    }
}

impl<B> Value<B>
where
    B: AsSlice,
{
    /// Coerce any pod into an owned pod.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut pod = pod::array();
    /// pod.as_mut().write(10i32)?;
    ///
    /// let pod = pod.as_ref().into_value()?.to_owned()?;
    ///
    /// assert_eq!(pod.as_ref().read_sized::<i32>()?, 10i32);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[cfg(feature = "alloc")]
    #[inline]
    pub fn to_owned(&self) -> Result<Value<DynamicBuf>, AllocError> {
        Ok(Value {
            buf: DynamicBuf::from_slice(self.buf.as_slice().as_bytes())?,
            size: self.size,
            ty: self.ty,
        })
    }

    /// Convert the [`Value`] into a one borrowing from but without modifying
    /// the current buffer.
    #[inline]
    pub fn as_ref(&self) -> Value<Slice<'_>> {
        Value::new(self.buf.as_slice(), self.size, self.ty)
    }
}

impl<B> Clone for Value<B>
where
    B: Clone,
{
    #[inline]
    fn clone(&self) -> Self {
        Value {
            size: self.size,
            ty: self.ty,
            buf: self.buf.clone(),
        }
    }
}

impl<'de> PodItem<'de> for Value<Slice<'de>> {
    #[inline]
    fn read<T>(self) -> Result<T, Error>
    where
        T: Readable<'de>,
    {
        Value::read(self)
    }

    #[inline]
    fn read_sized<T>(self) -> Result<T, Error>
    where
        T: SizedReadable<'de>,
    {
        Value::read_sized(self)
    }

    #[inline]
    fn read_unsized<T>(self) -> Result<&'de T, Error>
    where
        T: ?Sized + UnsizedReadable<'de>,
    {
        Value::read_unsized(self)
    }

    #[inline]
    fn read_struct(self) -> Result<Struct<Slice<'de>>, Error> {
        Value::read_struct(self)
    }

    #[inline]
    fn read_object(self) -> Result<Object<Slice<'de>>, Error> {
        Value::read_object(self)
    }

    #[inline]
    fn read_option(self) -> Result<Option<Self>, Error> {
        Value::read_option(self)
    }
}

impl<'de, B> PodStream<'de> for Value<B>
where
    B: Reader<'de>,
{
    type Item = Value<Slice<'de>>;

    #[inline]
    fn next(&mut self) -> Result<Value<Slice<'de>>, Error> {
        let buf = self.buf.split(self.size).ok_or(BufferUnderflow)?;
        let pod = Value::new(buf, self.size, self.ty);

        // Since the typed pod is consumed now, it no longer has a size, nor
        // does it produce values. It effective contains `Type::NONE`.
        self.size = 0;
        self.ty = Type::NONE;
        Ok(pod)
    }
}

/// [`UnsizedWritable`] implementation for [`Value`].
///
/// # Examples
///
/// ```
/// let mut pod = pod::array();
/// pod.as_mut().write_object(10, 20, |obj| {
///     obj.property(1).flags(0b001).write(1i32)?;
///     obj.property(2).flags(0b010).write(2i32)?;
///     obj.property(3).flags(0b100).write(3i32)?;
///     Ok(())
/// })?;
///
/// let mut pod2 = pod::array();
/// pod2.as_mut().write(pod.as_ref().into_value()?)?;
///
/// let mut obj = pod2.as_ref().read_pod()?.read_object()?;
/// assert!(!obj.is_empty());
///
/// let p = obj.property()?;
/// assert_eq!(p.key::<u32>(), 1);
/// assert_eq!(p.flags(), 0b001);
/// assert_eq!(p.value().read_sized::<i32>()?, 1);
///
/// let p = obj.property()?;
/// assert_eq!(p.key::<u32>(), 2);
/// assert_eq!(p.flags(), 0b010);
/// assert_eq!(p.value().read_sized::<i32>()?, 2);
///
/// let p = obj.property()?;
/// assert_eq!(p.key::<u32>(), 3);
/// assert_eq!(p.flags(), 0b100);
/// assert_eq!(p.value().read_sized::<i32>()?, 3);
///
/// assert!(obj.is_empty());
/// # Ok::<_, pod::Error>(())
/// ```
impl<B> UnsizedWritable for Value<B>
where
    B: AsSlice,
{
    const TYPE: Type = Type::POD;

    #[inline]
    fn size(&self) -> Option<usize> {
        let len = self.buf.as_slice().len();
        len.checked_add(mem::size_of::<[u32; 2]>())
    }

    #[inline]
    fn write_unsized(&self, mut writer: impl Writer) -> Result<(), Error> {
        let size = utils::to_word(self.size)?;
        writer.write(&[size, self.ty.into_u32()])?;
        writer.write(self.buf.as_slice().as_bytes())?;
        Ok(())
    }
}

crate::macros::encode_into_unsized!(impl [B] Value<B> where B: AsSlice);

impl<B> fmt::Debug for Value<B>
where
    B: AsSlice,
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
            Type::POD => tri!(tri!(self.as_ref().read_pod()).into_value()).fmt(f),
            ty => write!(f, "{{{ty:?}}}"),
        }
    }
}
