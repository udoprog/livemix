use core::ffi::CStr;

use crate::error::ErrorKind;
use crate::{Bitmap, Error, Type, Writer};

mod sealed {
    use core::ffi::CStr;

    use super::Bitmap;

    pub trait Sealed {}

    impl Sealed for [u8] {}
    impl Sealed for CStr {}
    impl Sealed for str {}
    impl Sealed for Bitmap {}
}

/// A trait for unsized types that can be encoded.
pub trait EncodeUnsized
where
    Self: self::sealed::Sealed,
{
    /// The type of the encoded value.
    #[doc(hidden)]
    const TYPE: Type;

    /// The size in bytes of the unsized value.
    #[doc(hidden)]
    fn size(&self) -> u32;

    #[doc(hidden)]
    fn write_content(&self, writer: impl Writer<u64>) -> Result<(), Error>;
}

/// [`EncodeUnsized`] implementation for an unsized `[u8]`.
///
/// # Examples
///
/// ```
/// use pod::Pod;
///
/// let mut pod = Pod::array();
/// pod.as_mut().push_unsized(&b"hello world"[..])?;
/// let pod = pod.as_ref();
/// assert_eq!(pod.decode_borrowed::<[u8]>()?, b"hello world");
/// # Ok::<_, pod::Error>(())
/// ```
impl EncodeUnsized for [u8] {
    const TYPE: Type = Type::BYTES;

    #[inline]
    fn size(&self) -> u32 {
        self.len() as u32
    }

    #[inline]
    fn write_content(&self, mut writer: impl Writer<u64>) -> Result<(), Error> {
        writer.write_bytes(self, 0)
    }
}

/// [`EncodeUnsized`] implementation for an unsized [`CStr`].
///
/// # Examples
///
/// ```
/// use core::ffi::CStr;
/// use pod::Pod;
///
/// let mut pod = Pod::array();
/// pod.as_mut().push_unsized(c"hello world")?;
/// let pod = pod.as_ref();
/// assert_eq!(pod.decode_borrowed::<CStr>()?, c"hello world");
/// # Ok::<_, pod::Error>(())
/// ```
impl EncodeUnsized for CStr {
    const TYPE: Type = Type::STRING;

    #[inline]
    fn size(&self) -> u32 {
        self.to_bytes_with_nul().len() as u32
    }

    #[inline]
    fn write_content(&self, mut writer: impl Writer<u64>) -> Result<(), Error> {
        writer.write_bytes(self.to_bytes_with_nul(), 0)?;
        Ok(())
    }
}

/// [`EncodeUnsized`] implementation for an unsized [`str`].
///
/// # Examples
///
/// ```
/// use pod::Pod;
///
/// let mut pod = Pod::array();
/// pod.as_mut().push_unsized("hello world")?;
/// let pod = pod.as_ref();
/// assert_eq!(pod.decode_borrowed::<str>()?, "hello world");
/// # Ok::<_, pod::Error>(())
/// ```
impl EncodeUnsized for str {
    const TYPE: Type = Type::STRING;

    #[inline]
    fn size(&self) -> u32 {
        str::len(self).wrapping_add(1) as u32
    }

    #[inline]
    fn write_content(&self, mut writer: impl Writer<u64>) -> Result<(), Error> {
        if self.as_bytes().contains(&0) {
            return Err(Error::new(ErrorKind::NullContainingString));
        }

        writer.write_bytes(self.as_bytes(), 1)?;
        Ok(())
    }
}

/// [`EncodeUnsized`] implementation for an unsized [`Bitmap`].
///
/// # Examples
///
/// ```
/// use pod::{Bitmap, Pod};
///
/// let mut pod = Pod::array();
/// pod.as_mut().push_unsized(Bitmap::new(b"asdfasdf"))?;
/// let pod = pod.as_ref();
/// assert_eq!(pod.decode_borrowed::<Bitmap>()?, b"asdfasdf");
/// # Ok::<_, pod::Error>(())
/// ```
impl EncodeUnsized for Bitmap {
    const TYPE: Type = Type::BITMAP;

    #[inline]
    fn size(&self) -> u32 {
        Bitmap::as_bytes(self).len() as u32
    }

    #[inline]
    fn write_content(&self, mut writer: impl Writer<u64>) -> Result<(), Error> {
        writer.write_bytes(self.as_bytes(), 0)?;
        Ok(())
    }
}
