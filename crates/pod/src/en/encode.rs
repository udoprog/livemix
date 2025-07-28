use crate::utils::WordBytes;
use crate::{EncodeUnsized, Error, Fd, Fraction, Id, Pointer, RawId, Rectangle, Type, Writer};

/// A trait for types that can be encoded.
pub trait Encode
where
    Self: Sized,
{
    /// The type of the encoded value.
    const TYPE: Type;

    /// The size in bytes of the encoded value.
    fn size(&self) -> usize;

    /// Write the content of a type.
    fn write_content(&self, writer: impl Writer<u64>) -> Result<(), Error>;
}

/// [`Encode`] implementation for `i32`.
///
/// # Examples
///
/// ```
/// use pod::Pod;
///
/// let mut pod = Pod::array();
/// pod.as_mut().push(true)?;
/// assert_eq!(pod.decode::<bool>()?, true);
/// # Ok::<_, pod::Error>(())
/// ```
impl Encode for bool {
    const TYPE: Type = Type::BOOL;

    #[inline]
    fn size(&self) -> usize {
        4
    }

    #[inline]
    fn write_content(&self, mut writer: impl Writer<u64>) -> Result<(), Error> {
        writer.write([if *self { 1u32 } else { 0u32 }, 0u32])
    }
}

/// [`Encode`] implementation for any type that can be converted into an [`Id`].
///
/// # Examples
///
/// ```
/// use pod::{Pod, Id};
///
/// let mut pod = Pod::array();
/// pod.as_mut().push(Id(142u32))?;
/// assert_eq!(pod.decode::<Id<u32>>()?, Id(142u32));
/// # Ok::<_, pod::Error>(())
/// ```
impl<I> Encode for Id<I>
where
    I: RawId,
{
    const TYPE: Type = Type::ID;

    #[inline]
    fn size(&self) -> usize {
        4
    }

    #[inline]
    fn write_content(&self, mut writer: impl Writer<u64>) -> Result<(), Error> {
        writer.write([self.0.into_id(), 0])
    }
}

/// [`Encode`] implementation for `i32`.
///
/// # Examples
///
/// ```
/// use pod::Pod;
///
/// let mut pod = Pod::array();
/// pod.as_mut().push(10i32)?;
/// assert_eq!(pod.decode::<i32>()?, 10);
/// # Ok::<_, pod::Error>(())
/// ```
impl Encode for i32 {
    const TYPE: Type = Type::INT;

    #[inline]
    fn size(&self) -> usize {
        4
    }

    #[inline]
    fn write_content(&self, mut writer: impl Writer<u64>) -> Result<(), Error> {
        writer.write([self.cast_unsigned(), 0])
    }
}

/// [`Encode`] implementation for `u32`.
///
/// # Examples
///
/// ```
/// use pod::Pod;
///
/// let mut pod = Pod::array();
/// pod.as_mut().push(10u32)?;
/// assert_eq!(pod.decode::<u32>()?, 10);
///
/// let mut pod = Pod::array();
/// pod.as_mut().push(10i32)?;
/// assert_eq!(pod.decode::<u32>()?, 10);
/// # Ok::<_, pod::Error>(())
/// ```
impl Encode for u32 {
    const TYPE: Type = Type::INT;

    #[inline]
    fn size(&self) -> usize {
        4
    }

    #[inline]
    fn write_content(&self, writer: impl Writer<u64>) -> Result<(), Error> {
        self.cast_signed().write_content(writer)
    }
}

/// [`Encode`] implementation for `i64`.
///
/// # Examples
///
/// ```
/// use pod::Pod;
///
/// let mut pod = Pod::array();
/// pod.as_mut().push(10i64)?;
/// assert_eq!(pod.decode::<i64>()?, 10i64);
/// # Ok::<_, pod::Error>(())
/// ```
impl Encode for i64 {
    const TYPE: Type = Type::LONG;

    #[inline]
    fn size(&self) -> usize {
        8
    }

    #[inline]
    fn write_content(&self, mut writer: impl Writer<u64>) -> Result<(), Error> {
        writer.write(self.cast_unsigned())
    }
}

/// [`Encode`] implementation for `u64`.
///
/// # Examples
///
/// ```
/// use pod::Pod;
///
/// let mut pod = Pod::array();
/// pod.as_mut().push(10u64)?;
/// assert_eq!(pod.decode::<u64>()?, 10);
///
/// let mut pod = Pod::array();
/// pod.as_mut().push(10i64)?;
/// assert_eq!(pod.decode::<u64>()?, 10);
/// # Ok::<_, pod::Error>(())
/// ```
impl Encode for u64 {
    const TYPE: Type = Type::LONG;

    #[inline]
    fn size(&self) -> usize {
        8
    }

    #[inline]
    fn write_content(&self, writer: impl Writer<u64>) -> Result<(), Error> {
        self.cast_signed().write_content(writer)
    }
}

/// [`Encode`] implementation for `f32`.
///
/// # Examples
///
/// ```
/// use pod::Pod;
///
/// let mut pod = Pod::array();
/// pod.as_mut().push(42.42f32)?;
/// assert_eq!(pod.decode::<f32>()?, 42.42f32);
/// # Ok::<_, pod::Error>(())
/// ```
impl Encode for f32 {
    const TYPE: Type = Type::FLOAT;

    #[inline]
    fn size(&self) -> usize {
        4
    }

    #[inline]
    fn write_content(&self, mut writer: impl Writer<u64>) -> Result<(), Error> {
        writer.write([self.to_bits(), 0])
    }
}

/// Decode implementation for `f64`.
///
/// # Examples
///
/// ```
/// use pod::Pod;
///
/// let mut pod = Pod::array();
/// pod.as_mut().push(42.42f64)?;
/// assert_eq!(pod.decode::<f64>()?, 42.42f64);
/// # Ok::<_, pod::Error>(())
/// ```
impl Encode for f64 {
    const TYPE: Type = Type::DOUBLE;

    #[inline]
    fn size(&self) -> usize {
        8
    }

    #[inline]
    fn write_content(&self, mut writer: impl Writer<u64>) -> Result<(), Error> {
        writer.write(self.to_bits())
    }
}

/// [`Encode`] implementation for [`Rectangle`].
///
/// # Examples
///
/// ```
/// use pod::{Pod, Rectangle};
///
/// let mut pod = Pod::array();
/// pod.as_mut().push(Rectangle::new(100, 200))?;
/// assert_eq!(pod.decode::<Rectangle>()?, Rectangle::new(100, 200));
/// # Ok::<_, pod::Error>(())
/// ```
impl Encode for Rectangle {
    const TYPE: Type = Type::RECTANGLE;

    #[inline]
    fn size(&self) -> usize {
        8
    }

    #[inline]
    fn write_content(&self, mut writer: impl Writer<u64>) -> Result<(), Error> {
        writer.write([self.width, self.height])
    }
}

/// [`Encode`] a [`Fraction`].
///
/// # Examples
///
/// ```
/// use pod::{Pod, Fraction};
///
/// let mut pod = Pod::array();
/// pod.as_mut().push(Fraction::new(800, 600))?;
/// assert_eq!(pod.decode::<Fraction>()?, Fraction::new(800, 600));
/// # Ok::<_, pod::Error>(())
/// ```
impl Encode for Fraction {
    const TYPE: Type = Type::FRACTION;

    #[inline]
    fn size(&self) -> usize {
        8
    }

    #[inline]
    fn write_content(&self, mut writer: impl Writer<u64>) -> Result<(), Error> {
        writer.write([self.num, self.denom])
    }
}

/// [`Encode`] a an array of bytes `[u8; N]`.
///
/// # Examples
///
/// ```
/// use pod::{Pod, Fraction};
///
/// let mut pod = Pod::array();
/// pod.as_mut().push(*b"hello world")?;
/// let pod = pod.as_ref();
/// assert_eq!(pod.decode_borrowed::<[u8]>()?, b"hello world");
/// # Ok::<_, pod::Error>(())
/// ```
impl<const N: usize> Encode for [u8; N] {
    const TYPE: Type = Type::BYTES;

    #[inline]
    fn size(&self) -> usize {
        N
    }

    #[inline]
    fn write_content(&self, writer: impl Writer<u64>) -> Result<(), Error> {
        <[u8]>::write_content(self, writer)
    }
}

/// [`Encode`] implementation for [`Pointer`].
///
/// # Examples
///
/// ```
/// use pod::{Pod, Pointer};
///
/// let value = 1u32;
///
/// let mut pod = Pod::array();
/// pod.as_mut().push(Pointer::new((&value as *const u32).addr()))?;
/// assert_eq!(pod.decode::<Pointer>()?, Pointer::new((&value as *const u32).addr()));
/// # Ok::<_, pod::Error>(())
/// ```
impl Encode for Pointer {
    const TYPE: Type = Type::POINTER;

    #[inline]
    fn size(&self) -> usize {
        16
    }

    #[inline]
    fn write_content(&self, mut writer: impl Writer<u64>) -> Result<(), Error> {
        let mut bytes = WordBytes::new();
        bytes.write_usize(self.pointer());

        writer.write([self.ty(), 0])?;
        writer.write_words(bytes.as_array())?;
        Ok(())
    }
}

/// [`Encode`] implementation for [`Fd`].
///
/// # Examples
///
/// ```
/// use pod::{Pod, Fd};
///
/// let mut pod = Pod::array();
/// pod.as_mut().push(Fd::new(4))?;
/// assert_eq!(pod.decode::<Fd>()?, Fd::new(4));
/// # Ok::<_, pod::Error>(())
/// ```
impl Encode for Fd {
    const TYPE: Type = Type::FD;

    #[inline]
    fn size(&self) -> usize {
        8
    }

    #[inline]
    fn write_content(&self, mut writer: impl Writer<u64>) -> Result<(), Error> {
        writer.write(self.fd().cast_unsigned())?;
        Ok(())
    }
}

/// [`Encode`] an unsized type through a reference.
///
/// # Examples
///
/// ```
/// use pod::Pod;
///
/// let mut pod = Pod::array();
/// pod.as_mut().push(&b"hello world"[..])?;
/// let pod = pod.as_ref();
/// assert_eq!(pod.decode_borrowed::<[u8]>()?, b"hello world");
/// # Ok::<_, pod::Error>(())
/// ```
impl<T> Encode for &T
where
    T: ?Sized + EncodeUnsized,
{
    const TYPE: Type = T::TYPE;

    #[inline]
    fn size(&self) -> usize {
        EncodeUnsized::size(*self)
    }

    #[inline]
    fn write_content(&self, writer: impl Writer<u64>) -> Result<(), Error> {
        <T as EncodeUnsized>::write_content(self, writer)
    }
}
