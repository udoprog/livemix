use core::ffi::CStr;

use crate::{Bitmap, Error, Reader};

use super::Decoder;

mod sealed {
    use core::ffi::CStr;

    use super::Bitmap;

    pub trait Sealed {}

    impl Sealed for Bitmap {}
    impl Sealed for [u8] {}
    impl Sealed for CStr {}
    impl Sealed for str {}
}

/// A trait for unsized types that can be decoded.
pub trait DecodeUnsized<'de>: self::sealed::Sealed {
    #[doc(hidden)]
    fn decode_unsized<R>(decoder: &mut Decoder<R>) -> Result<&'de Self, Error>
    where
        R: Reader<'de>;
}

/// [`DecodeUnsized`] implementation for an unsized `[u8]`.
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
/// let bytes: &[u8] = de.decode_unsized()?;
/// assert_eq!(bytes, b"hello world");
/// # Ok::<_, pod::Error>(())
/// ```
impl<'de> DecodeUnsized<'de> for [u8] {
    #[inline]
    fn decode_unsized<R>(decoder: &mut Decoder<R>) -> Result<&'de Self, Error>
    where
        R: Reader<'de>,
    {
        decoder.decode_borrowed_bytes()
    }
}

/// [`DecodeUnsized`] implementation for an unsized [`CStr`].
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
/// let bytes: &CStr = de.decode_unsized()?;
/// assert_eq!(bytes, c"hello world");
/// # Ok::<_, pod::Error>(())
/// ```
impl<'de> DecodeUnsized<'de> for CStr {
    #[inline]
    fn decode_unsized<R>(decoder: &mut Decoder<R>) -> Result<&'de Self, Error>
    where
        R: Reader<'de>,
    {
        decoder.decode_borrowed_c_str()
    }
}

/// [`DecodeUnsized`] implementation for an unsized [`str`].
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
/// let bytes: &str = de.decode_unsized()?;
/// assert_eq!(bytes, "hello world");
/// # Ok::<_, pod::Error>(())
/// ```
impl<'de> DecodeUnsized<'de> for str {
    #[inline]
    fn decode_unsized<R>(decoder: &mut Decoder<R>) -> Result<&'de Self, Error>
    where
        R: Reader<'de>,
    {
        decoder.decode_borrowed_str()
    }
}

/// [`DecodeUnsized`] implementation for an unsized [`Bitmap`].
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
/// let bitmap: &Bitmap = de.decode_unsized()?;
/// assert_eq!(bitmap, b"asdfasdf");
/// # Ok::<_, pod::Error>(())
/// ```
impl<'de> DecodeUnsized<'de> for Bitmap {
    #[inline]
    fn decode_unsized<R>(decoder: &mut Decoder<R>) -> Result<&'de Self, Error>
    where
        R: Reader<'de>,
    {
        decoder.decode_borrowed_bitmap()
    }
}
