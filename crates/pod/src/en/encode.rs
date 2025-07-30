use crate::error::ErrorKind;
use crate::utils::WordBytes;
use crate::{EncodeUnsized, Error, Fd, Fraction, Id, Pointer, RawId, Rectangle, Type, Writer};

/// A trait for types that can be encoded.
pub trait Encode
where
    Self: Sized,
{
    /// The type of the encoded value.
    const TYPE: Type;
    /// The size of the encoded value in bytes.
    const SIZE: usize;

    /// Write the content of a type.
    fn write_content(&self, writer: impl Writer) -> Result<(), Error>;
}

/// [`Encode`] implementation for `i32`.
///
/// # Examples
///
/// ```
/// let mut pod = pod::array();
/// pod.as_mut().push(true)?;
/// assert_eq!(pod.as_ref().next::<bool>()?, true);
/// # Ok::<_, pod::Error>(())
/// ```
impl Encode for bool {
    const TYPE: Type = Type::BOOL;
    const SIZE: usize = 4;

    #[inline]
    fn write_content(&self, mut writer: impl Writer) -> Result<(), Error> {
        writer.write([if *self { 1u32 } else { 0u32 }, 0u32])
    }
}

crate::macros::encode_into_sized!(bool);

/// [`Encode`] implementation for any type that can be converted into an [`Id`].
///
/// # Examples
///
/// ```
/// use pod::{Pod, Id};
///
/// let mut pod = pod::array();
/// pod.as_mut().push(Id(142u32))?;
/// assert_eq!(pod.as_ref().next::<Id<u32>>()?, Id(142u32));
/// # Ok::<_, pod::Error>(())
/// ```
impl<I> Encode for Id<I>
where
    I: RawId,
{
    const TYPE: Type = Type::ID;
    const SIZE: usize = 4;

    #[inline]
    fn write_content(&self, mut writer: impl Writer) -> Result<(), Error> {
        writer.write([self.0.into_id(), 0])
    }
}

crate::macros::encode_into_sized!(impl [I] Id<I> where I: RawId);

/// [`Encode`] implementation for `i32`.
///
/// # Examples
///
/// ```
/// let mut pod = pod::array();
/// pod.as_mut().push(10i32)?;
/// assert_eq!(pod.as_ref().next::<i32>()?, 10);
/// # Ok::<_, pod::Error>(())
/// ```
impl Encode for i32 {
    const TYPE: Type = Type::INT;
    const SIZE: usize = 4;

    #[inline]
    fn write_content(&self, mut writer: impl Writer) -> Result<(), Error> {
        writer.write([self.cast_unsigned(), 0])
    }
}

crate::macros::encode_into_sized!(i32);

/// [`Encode`] implementation for `isize`.
///
/// # Examples
///
/// ```
/// let mut pod = pod::array();
/// pod.as_mut().push(10isize)?;
/// assert_eq!(pod.as_ref().next::<isize>()?, 10);
/// # Ok::<_, pod::Error>(())
/// ```
impl Encode for isize {
    const TYPE: Type = Type::INT;
    const SIZE: usize = 4;

    #[inline]
    fn write_content(&self, writer: impl Writer) -> Result<(), Error> {
        let Ok(value) = i32::try_from(*self) else {
            return Err(Error::new(ErrorKind::InvalidIsizeInt { value: *self }));
        };

        value.write_content(writer)
    }
}

crate::macros::encode_into_sized!(isize);

/// [`Encode`] implementation for `u32`.
///
/// # Examples
///
/// ```
/// let mut pod = pod::array();
/// pod.as_mut().push(10u32)?;
/// assert_eq!(pod.as_ref().next::<u32>()?, 10);
///
/// let mut pod = pod::array();
/// pod.as_mut().push(10i32)?;
/// assert_eq!(pod.as_ref().next::<u32>()?, 10);
/// # Ok::<_, pod::Error>(())
/// ```
impl Encode for u32 {
    const TYPE: Type = Type::INT;
    const SIZE: usize = 4;

    #[inline]
    fn write_content(&self, writer: impl Writer) -> Result<(), Error> {
        self.cast_signed().write_content(writer)
    }
}

crate::macros::encode_into_sized!(u32);

/// [`Encode`] implementation for `usize`.
///
/// # Examples
///
/// ```
/// let mut pod = pod::array();
/// pod.as_mut().push(10usize)?;
/// assert_eq!(pod.as_ref().next::<usize>()?, 10);
///
/// let mut pod = pod::array();
/// pod.as_mut().push(10i32)?;
/// assert_eq!(pod.as_ref().next::<usize>()?, 10);
/// # Ok::<_, pod::Error>(())
/// ```
impl Encode for usize {
    const TYPE: Type = Type::INT;
    const SIZE: usize = 4;

    #[inline]
    fn write_content(&self, writer: impl Writer) -> Result<(), Error> {
        let Ok(value) = u32::try_from(*self) else {
            return Err(Error::new(ErrorKind::InvalidUsizeInt { value: *self }));
        };

        value.cast_signed().write_content(writer)
    }
}

crate::macros::encode_into_sized!(usize);

/// [`Encode`] implementation for `i64`.
///
/// # Examples
///
/// ```
/// let mut pod = pod::array();
/// pod.as_mut().push(10i64)?;
/// assert_eq!(pod.as_ref().next::<i64>()?, 10i64);
/// # Ok::<_, pod::Error>(())
/// ```
impl Encode for i64 {
    const TYPE: Type = Type::LONG;
    const SIZE: usize = 8;

    #[inline]
    fn write_content(&self, mut writer: impl Writer) -> Result<(), Error> {
        writer.write(self.cast_unsigned())
    }
}

crate::macros::encode_into_sized!(i64);

/// [`Encode`] implementation for `u64`.
///
/// # Examples
///
/// ```
/// let mut pod = pod::array();
/// pod.as_mut().push(10u64)?;
/// assert_eq!(pod.as_ref().next::<u64>()?, 10);
///
/// let mut pod = pod::array();
/// pod.as_mut().push(10i64)?;
/// assert_eq!(pod.as_ref().next::<u64>()?, 10);
/// # Ok::<_, pod::Error>(())
/// ```
impl Encode for u64 {
    const TYPE: Type = Type::LONG;
    const SIZE: usize = 8;

    #[inline]
    fn write_content(&self, writer: impl Writer) -> Result<(), Error> {
        self.cast_signed().write_content(writer)
    }
}
crate::macros::encode_into_sized!(u64);

/// [`Encode`] implementation for `f32`.
///
/// # Examples
///
/// ```
/// let mut pod = pod::array();
/// pod.as_mut().push(42.42f32)?;
/// assert_eq!(pod.as_ref().next::<f32>()?, 42.42f32);
/// # Ok::<_, pod::Error>(())
/// ```
impl Encode for f32 {
    const TYPE: Type = Type::FLOAT;
    const SIZE: usize = 4;

    #[inline]
    fn write_content(&self, mut writer: impl Writer) -> Result<(), Error> {
        writer.write([self.to_bits(), 0])
    }
}

crate::macros::encode_into_sized!(f32);

/// Decode implementation for `f64`.
///
/// # Examples
///
/// ```
/// let mut pod = pod::array();
/// pod.as_mut().push(42.42f64)?;
/// assert_eq!(pod.as_ref().next::<f64>()?, 42.42f64);
/// # Ok::<_, pod::Error>(())
/// ```
impl Encode for f64 {
    const TYPE: Type = Type::DOUBLE;
    const SIZE: usize = 8;

    #[inline]
    fn write_content(&self, mut writer: impl Writer) -> Result<(), Error> {
        writer.write(self.to_bits())
    }
}

crate::macros::encode_into_sized!(f64);

/// [`Encode`] implementation for [`Rectangle`].
///
/// # Examples
///
/// ```
/// use pod::{Pod, Rectangle};
///
/// let mut pod = pod::array();
/// pod.as_mut().push(Rectangle::new(100, 200))?;
/// assert_eq!(pod.as_ref().next::<Rectangle>()?, Rectangle::new(100, 200));
/// # Ok::<_, pod::Error>(())
/// ```
impl Encode for Rectangle {
    const TYPE: Type = Type::RECTANGLE;
    const SIZE: usize = 8;

    #[inline]
    fn write_content(&self, mut writer: impl Writer) -> Result<(), Error> {
        writer.write([self.width, self.height])
    }
}

crate::macros::encode_into_sized!(Rectangle);

/// [`Encode`] a [`Fraction`].
///
/// # Examples
///
/// ```
/// use pod::{Pod, Fraction};
///
/// let mut pod = pod::array();
/// pod.as_mut().push(Fraction::new(800, 600))?;
/// assert_eq!(pod.as_ref().next::<Fraction>()?, Fraction::new(800, 600));
/// # Ok::<_, pod::Error>(())
/// ```
impl Encode for Fraction {
    const TYPE: Type = Type::FRACTION;
    const SIZE: usize = 8;

    #[inline]
    fn write_content(&self, mut writer: impl Writer) -> Result<(), Error> {
        writer.write([self.num, self.denom])
    }
}

crate::macros::encode_into_sized!(Fraction);

/// [`Encode`] a an array of bytes `[u8; N]`.
///
/// # Examples
///
/// ```
/// use pod::{Pod, Fraction};
///
/// let mut pod = pod::array();
/// pod.as_mut().push(*b"hello world")?;
/// assert_eq!(pod.as_ref().next_borrowed::<[u8]>()?, b"hello world");
/// # Ok::<_, pod::Error>(())
/// ```
impl<const N: usize> Encode for [u8; N] {
    const TYPE: Type = Type::BYTES;
    const SIZE: usize = N;

    #[inline]
    fn write_content(&self, writer: impl Writer) -> Result<(), Error> {
        <[u8]>::write_content(self, writer)
    }
}

crate::macros::encode_into_sized!(impl [const N: usize] [u8; N]);

/// [`Encode`] implementation for [`Pointer`].
///
/// # Examples
///
/// ```
/// use pod::{Pod, Pointer};
///
/// let value = 1u32;
///
/// let mut pod = pod::array();
/// pod.as_mut().push(Pointer::new((&value as *const u32).addr()))?;
/// assert_eq!(pod.as_ref().next::<Pointer>()?, Pointer::new((&value as *const u32).addr()));
/// # Ok::<_, pod::Error>(())
/// ```
impl Encode for Pointer {
    const TYPE: Type = Type::POINTER;
    const SIZE: usize = 16;

    #[inline]
    fn write_content(&self, mut writer: impl Writer) -> Result<(), Error> {
        let mut bytes = WordBytes::new();
        bytes.write_usize(self.pointer());

        writer.write([self.ty(), 0])?;
        writer.write_words(bytes.as_array())?;
        Ok(())
    }
}

crate::macros::encode_into_sized!(Pointer);

/// [`Encode`] implementation for [`Fd`].
///
/// # Examples
///
/// ```
/// use pod::{Pod, Fd};
///
/// let mut pod = pod::array();
/// pod.as_mut().push(Fd::new(4))?;
/// assert_eq!(pod.as_ref().next::<Fd>()?, Fd::new(4));
/// # Ok::<_, pod::Error>(())
/// ```
impl Encode for Fd {
    const TYPE: Type = Type::FD;
    const SIZE: usize = 8;

    #[inline]
    fn write_content(&self, mut writer: impl Writer) -> Result<(), Error> {
        writer.write(self.fd().cast_unsigned())?;
        Ok(())
    }
}

crate::macros::encode_into_sized!(Fd);

/// [`Encode`] an unsized type through a reference.
///
/// # Examples
///
/// ```
/// let value = 42u32;
///
/// let mut pod = pod::array();
/// pod.as_mut().push(&value)?;
/// assert_eq!(pod.as_ref().next::<u32>()?, value);
/// # Ok::<_, pod::Error>(())
/// ```
impl<T> Encode for &T
where
    T: ?Sized + Encode,
{
    const TYPE: Type = T::TYPE;
    const SIZE: usize = T::SIZE;

    #[inline]
    fn write_content(&self, writer: impl Writer) -> Result<(), Error> {
        <T as Encode>::write_content(self, writer)
    }
}
