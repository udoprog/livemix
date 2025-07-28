use core::ffi::CStr;

use crate::error::ErrorKind;
use crate::{Bitmap, Error, Type, Writer};

/// A trait for unsized types that can be encoded.
pub trait EncodeUnsized {
    /// The type of the encoded value.
    #[doc(hidden)]
    const TYPE: Type;

    /// The size in bytes of the unsized value.
    #[doc(hidden)]
    fn size(&self) -> usize;

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
/// assert_eq!(pod.next_borrowed::<[u8]>()?, b"hello world");
/// # Ok::<_, pod::Error>(())
/// ```
impl EncodeUnsized for [u8] {
    const TYPE: Type = Type::BYTES;

    #[inline]
    fn size(&self) -> usize {
        self.len()
    }

    #[inline]
    fn write_content(&self, mut writer: impl Writer<u64>) -> Result<(), Error> {
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
/// use pod::Pod;
///
/// let mut pod = Pod::array();
/// pod.as_mut().push_unsized(c"hello world")?;
/// let pod = pod.as_ref();
/// assert_eq!(pod.next_borrowed::<CStr>()?, c"hello world");
/// # Ok::<_, pod::Error>(())
/// ```
impl EncodeUnsized for CStr {
    const TYPE: Type = Type::STRING;

    #[inline]
    fn size(&self) -> usize {
        self.to_bytes_with_nul().len()
    }

    #[inline]
    fn write_content(&self, mut writer: impl Writer<u64>) -> Result<(), Error> {
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
/// use pod::Pod;
///
/// let mut pod = Pod::array();
/// pod.as_mut().push_unsized("hello\0world")?;
/// # Ok::<_, pod::Error>(())
/// ```
///
/// # Examples
///
/// ```
/// use pod::Pod;
///
/// let mut pod = Pod::array();
/// pod.as_mut().push_unsized("hello world")?;
/// let pod = pod.as_ref();
/// assert_eq!(pod.next_borrowed::<str>()?, "hello world");
/// # Ok::<_, pod::Error>(())
/// ```
impl EncodeUnsized for str {
    const TYPE: Type = Type::STRING;

    #[inline]
    fn size(&self) -> usize {
        // A string cannot be longer than `isize::MAX`, so we can always add 1
        // to it to get a correct usize.
        str::len(self).wrapping_add(1)
    }

    #[inline]
    fn write_content(&self, mut writer: impl Writer<u64>) -> Result<(), Error> {
        let bytes = self.as_bytes();

        if bytes.contains(&0) {
            return Err(Error::new(ErrorKind::NullContainingString));
        }

        writer.write_bytes(bytes, 1)?;
        Ok(())
    }
}

crate::macros::encode_into_unsized!(str);

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
/// assert_eq!(pod.next_borrowed::<Bitmap>()?, b"asdfasdf");
/// # Ok::<_, pod::Error>(())
/// ```
impl EncodeUnsized for Bitmap {
    const TYPE: Type = Type::BITMAP;

    #[inline]
    fn size(&self) -> usize {
        Bitmap::as_bytes(self).len()
    }

    #[inline]
    fn write_content(&self, mut writer: impl Writer<u64>) -> Result<(), Error> {
        writer.write_bytes(self.as_bytes(), 0)?;
        Ok(())
    }
}

crate::macros::encode_into_unsized!(Bitmap);
