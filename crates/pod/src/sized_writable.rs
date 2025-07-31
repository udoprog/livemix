use crate::error::ErrorKind;
use crate::utils::WordBytes;
use crate::{Error, Fd, Fraction, Id, Pointer, RawId, Rectangle, Type, UnsizedWritable, Writer};

/// A trait for types that can be encoded.
pub trait SizedWritable
where
    Self: Sized,
{
    /// The type of the encoded value.
    const TYPE: Type;
    /// The size of the encoded value in bytes.
    const SIZE: usize;

    /// Write the content of a type.
    fn write_sized(&self, writer: impl Writer) -> Result<(), Error>;
}

/// [`SizedWritable`] implementation for `bool`.
///
/// # Examples
///
/// ```
/// let mut pod = pod::array();
/// pod.as_mut().write(true)?;
/// assert_eq!(pod.as_ref().read_sized::<bool>()?, true);
/// # Ok::<_, pod::Error>(())
/// ```
impl SizedWritable for bool {
    const TYPE: Type = Type::BOOL;
    const SIZE: usize = 4;

    #[inline]
    fn write_sized(&self, mut writer: impl Writer) -> Result<(), Error> {
        writer.write(&[if *self { 1u32 } else { 0u32 }])
    }
}

crate::macros::encode_into_sized!(bool);

/// [`SizedWritable`] implementation for any type that can be converted into an
/// [`Id`].
///
/// # Examples
///
/// ```
/// use pod::{Pod, Id};
///
/// let mut pod = pod::array();
/// pod.as_mut().write(Id(142u32))?;
/// assert_eq!(pod.as_ref().read_sized::<Id<u32>>()?, Id(142u32));
/// # Ok::<_, pod::Error>(())
/// ```
impl<I> SizedWritable for Id<I>
where
    I: RawId,
{
    const TYPE: Type = Type::ID;
    const SIZE: usize = 4;

    #[inline]
    fn write_sized(&self, mut writer: impl Writer) -> Result<(), Error> {
        writer.write(&[self.0.into_id()])
    }
}

crate::macros::encode_into_sized!(impl [I] Id<I> where I: RawId);

/// [`SizedWritable`] implementation for `i32`.
///
/// # Examples
///
/// ```
/// let mut pod = pod::array();
/// pod.as_mut().write(10i32)?;
/// assert_eq!(pod.as_ref().read_sized::<i32>()?, 10);
/// # Ok::<_, pod::Error>(())
/// ```
impl SizedWritable for i32 {
    const TYPE: Type = Type::INT;
    const SIZE: usize = 4;

    #[inline]
    fn write_sized(&self, mut writer: impl Writer) -> Result<(), Error> {
        writer.write(&[self.cast_unsigned()])
    }
}

crate::macros::encode_into_sized!(i32);

/// [`SizedWritable`] implementation for `isize`.
///
/// # Examples
///
/// ```
/// let mut pod = pod::array();
/// pod.as_mut().write(10isize)?;
/// assert_eq!(pod.as_ref().read_sized::<isize>()?, 10);
/// # Ok::<_, pod::Error>(())
/// ```
impl SizedWritable for isize {
    const TYPE: Type = Type::INT;
    const SIZE: usize = 4;

    #[inline]
    fn write_sized(&self, writer: impl Writer) -> Result<(), Error> {
        let Ok(value) = i32::try_from(*self) else {
            return Err(Error::new(ErrorKind::InvalidIsizeInt { value: *self }));
        };

        value.write_sized(writer)
    }
}

crate::macros::encode_into_sized!(isize);

/// [`SizedWritable`] implementation for `u32`.
///
/// # Examples
///
/// ```
/// let mut pod = pod::array();
/// pod.as_mut().write(10u32)?;
/// assert_eq!(pod.as_ref().read_sized::<u32>()?, 10);
///
/// let mut pod = pod::array();
/// pod.as_mut().write(10i32)?;
/// assert_eq!(pod.as_ref().read_sized::<u32>()?, 10);
/// # Ok::<_, pod::Error>(())
/// ```
impl SizedWritable for u32 {
    const TYPE: Type = Type::INT;
    const SIZE: usize = 4;

    #[inline]
    fn write_sized(&self, writer: impl Writer) -> Result<(), Error> {
        self.cast_signed().write_sized(writer)
    }
}

crate::macros::encode_into_sized!(u32);

/// [`SizedWritable`] implementation for `usize`.
///
/// # Examples
///
/// ```
/// let mut pod = pod::array();
/// pod.as_mut().write(10usize)?;
/// assert_eq!(pod.as_ref().read_sized::<usize>()?, 10);
///
/// let mut pod = pod::array();
/// pod.as_mut().write(10i32)?;
/// assert_eq!(pod.as_ref().read_sized::<usize>()?, 10);
/// # Ok::<_, pod::Error>(())
/// ```
impl SizedWritable for usize {
    const TYPE: Type = Type::INT;
    const SIZE: usize = 4;

    #[inline]
    fn write_sized(&self, writer: impl Writer) -> Result<(), Error> {
        let Ok(value) = u32::try_from(*self) else {
            return Err(Error::new(ErrorKind::InvalidUsizeInt { value: *self }));
        };

        value.cast_signed().write_sized(writer)
    }
}

crate::macros::encode_into_sized!(usize);

/// [`SizedWritable`] implementation for `i64`.
///
/// # Examples
///
/// ```
/// let mut pod = pod::array();
/// pod.as_mut().write(10i64)?;
/// assert_eq!(pod.as_ref().read_sized::<i64>()?, 10i64);
/// # Ok::<_, pod::Error>(())
/// ```
impl SizedWritable for i64 {
    const TYPE: Type = Type::LONG;
    const SIZE: usize = 8;

    #[inline]
    fn write_sized(&self, mut writer: impl Writer) -> Result<(), Error> {
        writer.write(&[self.cast_unsigned()])
    }
}

crate::macros::encode_into_sized!(i64);

/// [`SizedWritable`] implementation for `u64`.
///
/// # Examples
///
/// ```
/// let mut pod = pod::array();
/// pod.as_mut().write(10u64)?;
/// assert_eq!(pod.as_ref().read_sized::<u64>()?, 10);
///
/// let mut pod = pod::array();
/// pod.as_mut().write(10i64)?;
/// assert_eq!(pod.as_ref().read_sized::<u64>()?, 10);
/// # Ok::<_, pod::Error>(())
/// ```
impl SizedWritable for u64 {
    const TYPE: Type = Type::LONG;
    const SIZE: usize = 8;

    #[inline]
    fn write_sized(&self, writer: impl Writer) -> Result<(), Error> {
        self.cast_signed().write_sized(writer)
    }
}
crate::macros::encode_into_sized!(u64);

/// [`SizedWritable`] implementation for `f32`.
///
/// # Examples
///
/// ```
/// let mut pod = pod::array();
/// pod.as_mut().write(42.42f32)?;
/// assert_eq!(pod.as_ref().read_sized::<f32>()?, 42.42f32);
/// # Ok::<_, pod::Error>(())
/// ```
impl SizedWritable for f32 {
    const TYPE: Type = Type::FLOAT;
    const SIZE: usize = 4;

    #[inline]
    fn write_sized(&self, mut writer: impl Writer) -> Result<(), Error> {
        writer.write(&[self.to_bits(), 0])
    }
}

crate::macros::encode_into_sized!(f32);

/// [`SizedWritable`] implementation for `f64`.
///
/// # Examples
///
/// ```
/// let mut pod = pod::array();
/// pod.as_mut().write(42.42f64)?;
/// assert_eq!(pod.as_ref().read_sized::<f64>()?, 42.42f64);
/// # Ok::<_, pod::Error>(())
/// ```
impl SizedWritable for f64 {
    const TYPE: Type = Type::DOUBLE;
    const SIZE: usize = 8;

    #[inline]
    fn write_sized(&self, mut writer: impl Writer) -> Result<(), Error> {
        writer.write(&[self.to_bits()])
    }
}

crate::macros::encode_into_sized!(f64);

/// [`SizedWritable`] implementation for [`Rectangle`].
///
/// # Examples
///
/// ```
/// use pod::{Pod, Rectangle};
///
/// let mut pod = pod::array();
/// pod.as_mut().write(Rectangle::new(100, 200))?;
/// assert_eq!(pod.as_ref().read_sized::<Rectangle>()?, Rectangle::new(100, 200));
/// # Ok::<_, pod::Error>(())
/// ```
impl SizedWritable for Rectangle {
    const TYPE: Type = Type::RECTANGLE;
    const SIZE: usize = 8;

    #[inline]
    fn write_sized(&self, mut writer: impl Writer) -> Result<(), Error> {
        writer.write(&[self.width, self.height])
    }
}

crate::macros::encode_into_sized!(Rectangle);

/// [`SizedWritable`] a [`Fraction`].
///
/// # Examples
///
/// ```
/// use pod::{Pod, Fraction};
///
/// let mut pod = pod::array();
/// pod.as_mut().write(Fraction::new(800, 600))?;
/// assert_eq!(pod.as_ref().read_sized::<Fraction>()?, Fraction::new(800, 600));
/// # Ok::<_, pod::Error>(())
/// ```
impl SizedWritable for Fraction {
    const TYPE: Type = Type::FRACTION;
    const SIZE: usize = 8;

    #[inline]
    fn write_sized(&self, mut writer: impl Writer) -> Result<(), Error> {
        writer.write(&[self.num, self.denom])
    }
}

crate::macros::encode_into_sized!(Fraction);

/// [`SizedWritable`] a an array of bytes `[u8; N]`.
///
/// # Examples
///
/// ```
/// use pod::{Pod, Fraction};
///
/// let mut pod = pod::array();
/// pod.as_mut().write(*b"hello world")?;
/// assert_eq!(pod.as_ref().read_unsized::<[u8]>()?, b"hello world");
/// # Ok::<_, pod::Error>(())
/// ```
impl<const N: usize> SizedWritable for [u8; N] {
    const TYPE: Type = Type::BYTES;
    const SIZE: usize = N;

    #[inline]
    fn write_sized(&self, writer: impl Writer) -> Result<(), Error> {
        <[u8]>::write_unsized(self, writer)
    }
}

crate::macros::encode_into_sized!(impl [const N: usize] [u8; N]);

/// [`SizedWritable`] implementation for [`Pointer`].
///
/// # Examples
///
/// ```
/// use pod::{Pod, Pointer};
///
/// let value = 1u32;
///
/// let mut pod = pod::array();
/// pod.as_mut().write(Pointer::new((&value as *const u32).addr()))?;
/// assert_eq!(pod.as_ref().read_sized::<Pointer>()?, Pointer::new((&value as *const u32).addr()));
/// # Ok::<_, pod::Error>(())
/// ```
impl SizedWritable for Pointer {
    const TYPE: Type = Type::POINTER;
    const SIZE: usize = 16;

    #[inline]
    fn write_sized(&self, mut writer: impl Writer) -> Result<(), Error> {
        let mut bytes = WordBytes::new();
        bytes.write_usize(self.pointer());

        writer.write(&[self.ty(), 0])?;
        writer.write(bytes.as_array())?;
        Ok(())
    }
}

crate::macros::encode_into_sized!(Pointer);

/// [`SizedWritable`] implementation for [`Fd`].
///
/// # Examples
///
/// ```
/// use pod::{Pod, Fd};
///
/// let mut pod = pod::array();
/// pod.as_mut().write(Fd::new(4))?;
/// assert_eq!(pod.as_ref().read_sized::<Fd>()?, Fd::new(4));
/// # Ok::<_, pod::Error>(())
/// ```
impl SizedWritable for Fd {
    const TYPE: Type = Type::FD;
    const SIZE: usize = 8;

    #[inline]
    fn write_sized(&self, mut writer: impl Writer) -> Result<(), Error> {
        writer.write(&[self.fd().cast_unsigned()])?;
        Ok(())
    }
}

crate::macros::encode_into_sized!(Fd);

/// [`SizedWritable`] an unsized type through a reference.
///
/// # Examples
///
/// ```
/// let value = 42u32;
///
/// let mut pod = pod::array();
/// pod.as_mut().write(&value)?;
/// assert_eq!(pod.as_ref().read_sized::<u32>()?, value);
/// # Ok::<_, pod::Error>(())
/// ```
impl<T> SizedWritable for &T
where
    T: ?Sized + SizedWritable,
{
    const TYPE: Type = T::TYPE;
    const SIZE: usize = T::SIZE;

    #[inline]
    fn write_sized(&self, writer: impl Writer) -> Result<(), Error> {
        <T as SizedWritable>::write_sized(self, writer)
    }
}
