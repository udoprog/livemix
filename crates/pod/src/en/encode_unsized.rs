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
/// use pod::{ArrayBuf, Encoder, Decoder};
///
/// let mut buf = ArrayBuf::new();
/// let mut encoder = Encoder::new(&mut buf);
/// encoder.encode_unsized(&b"hello world"[..])?;
///
/// let mut de = Decoder::new(buf.as_reader_slice());
/// let bytes: &[u8] = de.decode_borrowed()?;
/// assert_eq!(bytes, b"hello world");
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

        writer.write_words(&[len, Type::BYTES.into_u32()])?;
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
/// use pod::{ArrayBuf, Encoder, Decoder};
///
/// let mut buf = ArrayBuf::new();
/// let mut encoder = Encoder::new(&mut buf);
/// encoder.encode_unsized(c"hello world")?;
///
/// let mut de = Decoder::new(buf.as_reader_slice());
/// let bytes: &CStr = de.decode_borrowed()?;
/// assert_eq!(bytes, c"hello world");
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

        writer.write_words(&[len, Type::STRING.into_u32()])?;
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
/// use pod::{ArrayBuf, Encoder, Decoder};
///
/// let mut buf = ArrayBuf::new();
/// let mut encoder = Encoder::new(&mut buf);
/// encoder.encode_unsized("hello world")?;
///
/// let mut de = Decoder::new(buf.as_reader_slice());
/// let bytes: &str = de.decode_borrowed()?;
/// assert_eq!(bytes, "hello world");
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

        writer.write_words(&[len, Type::STRING.into_u32()])?;
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
/// use pod::{ArrayBuf, Bitmap, Encoder, Decoder};
///
/// let mut buf = ArrayBuf::new();
/// let mut encoder = Encoder::new(&mut buf);
/// encoder.encode_unsized(Bitmap::new(b"asdfasdf"))?;
///
/// let mut de = Decoder::new(buf.as_reader_slice());
/// let bitmap: &Bitmap = de.decode_borrowed()?;
/// assert_eq!(bitmap, b"asdfasdf");
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

        writer.write_words(&[len, Type::BITMAP.into_u32()])?;
        writer.write_bytes(value, 0)?;
        Ok(())
    }

    #[inline]
    fn write_content(&self, mut writer: impl Writer) -> Result<(), Error> {
        writer.write_bytes(self.as_bytes(), 0)?;
        Ok(())
    }
}
