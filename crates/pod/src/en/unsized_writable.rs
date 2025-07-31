use core::ffi::CStr;

#[cfg(feature = "alloc")]
use alloc::string::String;

use crate::error::ErrorKind;
use crate::{Bitmap, Error, Type, Writer};

/// A trait for unsized types that can be encoded.
pub trait UnsizedWritable {
    /// The type of the encoded value.
    #[doc(hidden)]
    const TYPE: Type;

    /// The size in bytes of the unsized value.
    #[doc(hidden)]
    fn size(&self) -> Option<usize>;

    #[doc(hidden)]
    fn write_content(&self, writer: impl Writer) -> Result<(), Error>;
}

/// [`EncodeUnsized`] implementation for an unsized `[u8]`.
///
/// # Examples
///
/// ```
/// let mut pod = pod::array();
/// pod.as_mut().write_unsized(&b"hello world"[..])?;
/// let pod = pod.as_ref();
/// assert_eq!(pod.read_unsized::<[u8]>()?, b"hello world");
/// # Ok::<_, pod::Error>(())
/// ```
impl UnsizedWritable for [u8] {
    const TYPE: Type = Type::BYTES;

    #[inline]
    fn size(&self) -> Option<usize> {
        Some(self.len())
    }

    #[inline]
    fn write_content(&self, mut writer: impl Writer) -> Result<(), Error> {
        writer.write_bytes(self, 0)
    }
}

crate::macros::encode_into_unsized!([u8]);

/// [`EncodeUnsized`] implementation for an unsized [`CStr`].
///
/// # Examples
///
/// ```
/// use core::ffi::CStr;
/// let mut pod = pod::array();
/// pod.as_mut().write_unsized(c"hello world")?;
/// let pod = pod.as_ref();
/// assert_eq!(pod.read_unsized::<CStr>()?, c"hello world");
/// # Ok::<_, pod::Error>(())
/// ```
impl UnsizedWritable for CStr {
    const TYPE: Type = Type::STRING;

    #[inline]
    fn size(&self) -> Option<usize> {
        Some(self.to_bytes_with_nul().len())
    }

    #[inline]
    fn write_content(&self, mut writer: impl Writer) -> Result<(), Error> {
        writer.write_bytes(self.to_bytes_with_nul(), 0)?;
        Ok(())
    }
}

crate::macros::encode_into_unsized!(CStr);

/// [`EncodeUnsized`] implementation for an unsized [`str`].
///
/// # Errors
///
/// Trying to encode a UTf-8 string containing a null byte will return an error.
/// This is due to the underlying representation being a C-style string who's
/// length must be determined by a terminating null.
///
/// ```should_panic
/// let mut pod = pod::array();
/// pod.as_mut().write_unsized("hello\0world")?;
/// # Ok::<_, pod::Error>(())
/// ```
///
/// # Examples
///
/// ```
/// let mut pod = pod::array();
/// pod.as_mut().write_unsized("hello world")?;
/// let pod = pod.as_ref();
/// assert_eq!(pod.read_unsized::<str>()?, "hello world");
/// # Ok::<_, pod::Error>(())
/// ```
impl UnsizedWritable for str {
    const TYPE: Type = Type::STRING;

    #[inline]
    fn size(&self) -> Option<usize> {
        // A string cannot be longer than `isize::MAX`, so we can always add 1
        // to it to get a correct usize.
        Some(str::len(self).wrapping_add(1))
    }

    #[inline]
    fn write_content(&self, mut writer: impl Writer) -> Result<(), Error> {
        let bytes = self.as_bytes();

        if bytes.contains(&0) {
            return Err(Error::new(ErrorKind::NullContainingString));
        }

        writer.write_bytes(bytes, 1)?;
        Ok(())
    }
}

crate::macros::encode_into_unsized!(str);

/// Encode an owned [`String`].
///
/// # Examples
///
/// ```
/// let mut pod = pod::array();
///
/// pod.as_mut().write_unsized(&String::from("hello world"))?;
/// pod.as_mut().write_unsized(&String::from("this is right"))?;
///
/// let mut pod = pod.as_ref();
/// assert_eq!(pod.as_mut().read_sized::<String>()?, "hello world");
/// assert_eq!(pod.as_mut().read_sized::<String>()?, "this is right");
/// # Ok::<_, pod::Error>(())
/// ```
#[cfg(feature = "alloc")]
impl UnsizedWritable for String {
    const TYPE: Type = Type::STRING;

    #[inline]
    fn size(&self) -> Option<usize> {
        // A string cannot be longer than `isize::MAX`, so we can always add 1
        // to it to get a correct usize.
        Some(str::len(self).wrapping_add(1))
    }

    #[inline]
    fn write_content(&self, writer: impl Writer) -> Result<(), Error> {
        str::write_content(self, writer)
    }
}

crate::macros::encode_into_unsized!(String);

/// [`EncodeUnsized`] implementation for an unsized [`Bitmap`].
///
/// # Examples
///
/// ```
/// use pod::{Bitmap, Pod};
///
/// let mut pod = pod::array();
/// pod.as_mut().write_unsized(Bitmap::new(b"asdfasdf"))?;
/// let pod = pod.as_ref();
/// assert_eq!(pod.read_unsized::<Bitmap>()?, b"asdfasdf");
/// # Ok::<_, pod::Error>(())
/// ```
impl UnsizedWritable for Bitmap {
    const TYPE: Type = Type::BITMAP;

    #[inline]
    fn size(&self) -> Option<usize> {
        Some(Bitmap::as_bytes(self).len())
    }

    #[inline]
    fn write_content(&self, mut writer: impl Writer) -> Result<(), Error> {
        writer.write_bytes(self.as_bytes(), 0)?;
        Ok(())
    }
}

crate::macros::encode_into_unsized!(Bitmap);
