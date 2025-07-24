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
pub trait EncodeUnsized: self::sealed::Sealed {
    /// The type of the encoded value.
    #[doc(hidden)]
    const TYPE: Type;

    /// The size in bytes of the unsized value.
    #[doc(hidden)]
    fn size(&self) -> usize;

    #[doc(hidden)]
    fn encode_unsized(&self, writer: impl Writer) -> Result<(), Error>;

    #[doc(hidden)]
    fn write_content(&self, writer: impl Writer) -> Result<(), Error>;
}

/// [`EncodeUnsized`] implementation for an unsized `[u8]`.
///
/// # Examples
///
/// ```
/// use pod::Pod;
///
/// let mut pod = Pod::array();
/// pod.encode_unsized(&b"hello world"[..])?;
///
/// let pod = pod.typed()?;
/// assert_eq!(pod.decode_borrowed::<[u8]>()?, b"hello world");
/// # Ok::<_, pod::Error>(())
/// ```
impl EncodeUnsized for [u8] {
    const TYPE: Type = Type::BYTES;

    #[inline]
    fn size(&self) -> usize {
        self.len()
    }

    #[inline]
    fn encode_unsized(&self, mut writer: impl Writer) -> Result<(), Error> {
        let Ok(len) = u32::try_from(self.len()) else {
            return Err(Error::new(ErrorKind::SizeOverflow));
        };

        writer.write([len, Type::BYTES.into_u32()])?;
        writer.write_bytes(self, 0)?;
        Ok(())
    }

    #[inline]
    fn write_content(&self, mut writer: impl Writer) -> Result<(), Error> {
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
/// pod.encode_unsized(c"hello world")?;
///
/// let pod = pod.typed()?;
/// assert_eq!(pod.decode_borrowed::<CStr>()?, c"hello world");
/// # Ok::<_, pod::Error>(())
/// ```
impl EncodeUnsized for CStr {
    const TYPE: Type = Type::STRING;

    #[inline]
    fn size(&self) -> usize {
        self.to_bytes_with_nul().len()
    }

    #[inline]
    fn encode_unsized(&self, mut writer: impl Writer) -> Result<(), Error> {
        let bytes = self.to_bytes_with_nul();

        let Ok(len) = u32::try_from(bytes.len()) else {
            return Err(Error::new(ErrorKind::SizeOverflow));
        };

        writer.write([len, Type::STRING.into_u32()])?;
        writer.write_bytes(bytes, 0)?;
        Ok(())
    }

    #[inline]
    fn write_content(&self, mut writer: impl Writer) -> Result<(), Error> {
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
/// pod.encode_unsized("hello world")?;
///
/// let pod = pod.typed()?;
/// assert_eq!(pod.decode_borrowed::<str>()?, "hello world");
/// # Ok::<_, pod::Error>(())
/// ```
impl EncodeUnsized for str {
    const TYPE: Type = Type::STRING;

    #[inline]
    fn size(&self) -> usize {
        str::len(self).wrapping_add(1)
    }

    #[inline]
    fn encode_unsized(&self, mut writer: impl Writer) -> Result<(), Error> {
        let bytes = self.as_bytes();

        let Some(len) = bytes
            .len()
            .checked_add(1)
            .and_then(|v| u32::try_from(v).ok())
        else {
            return Err(Error::new(ErrorKind::SizeOverflow));
        };

        if bytes.contains(&0) {
            return Err(Error::new(ErrorKind::NullContainingString));
        }

        writer.write([len, Type::STRING.into_u32()])?;
        writer.write_bytes(bytes, 1)?;
        Ok(())
    }

    #[inline]
    fn write_content(&self, mut writer: impl Writer) -> Result<(), Error> {
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
/// pod.encode_unsized(Bitmap::new(b"asdfasdf"))?;
///
/// let pod = pod.typed()?;
/// assert_eq!(pod.decode_borrowed::<Bitmap>()?, b"asdfasdf");
/// # Ok::<_, pod::Error>(())
/// ```
impl EncodeUnsized for Bitmap {
    const TYPE: Type = Type::BITMAP;

    #[inline]
    fn size(&self) -> usize {
        Bitmap::as_bytes(self).len()
    }

    #[inline]
    fn encode_unsized(&self, mut writer: impl Writer) -> Result<(), Error> {
        let value = self.as_bytes();

        let Ok(len) = u32::try_from(value.len()) else {
            return Err(Error::new(ErrorKind::SizeOverflow));
        };

        writer.write([len, Type::BITMAP.into_u32()])?;
        writer.write_bytes(value, 0)?;
        Ok(())
    }

    #[inline]
    fn write_content(&self, mut writer: impl Writer) -> Result<(), Error> {
        writer.write_bytes(self.as_bytes(), 0)?;
        Ok(())
    }
}
