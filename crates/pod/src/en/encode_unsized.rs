use core::ffi::CStr;

use crate::{Bitmap, Error, Writer};

use super::Encoder;

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
    #[doc(hidden)]
    fn encode_unsized<W>(&self, encoder: &mut Encoder<W>) -> Result<(), Error>
    where
        W: Writer;
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
/// let bytes: &[u8] = de.decode_unsized()?;
/// assert_eq!(bytes, b"hello world");
/// # Ok::<_, pod::Error>(())
/// ```
impl EncodeUnsized for [u8] {
    #[inline]
    fn encode_unsized<W>(&self, encoder: &mut Encoder<W>) -> Result<(), Error>
    where
        W: Writer,
    {
        encoder.encode_bytes(self)
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
/// let bytes: &CStr = de.decode_unsized()?;
/// assert_eq!(bytes, c"hello world");
/// # Ok::<_, pod::Error>(())
/// ```
impl EncodeUnsized for CStr {
    #[inline]
    fn encode_unsized<W>(&self, encoder: &mut Encoder<W>) -> Result<(), Error>
    where
        W: Writer,
    {
        encoder.encode_c_str(self)
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
/// let bytes: &str = de.decode_unsized()?;
/// assert_eq!(bytes, "hello world");
/// # Ok::<_, pod::Error>(())
/// ```
impl EncodeUnsized for str {
    #[inline]
    fn encode_unsized<W>(&self, encoder: &mut Encoder<W>) -> Result<(), Error>
    where
        W: Writer,
    {
        encoder.encode_str(self)
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
/// let bitmap: &Bitmap = de.decode_unsized()?;
/// assert_eq!(bitmap, b"asdfasdf");
/// # Ok::<_, pod::Error>(())
/// ```
impl EncodeUnsized for Bitmap {
    #[inline]
    fn encode_unsized<W>(&self, encoder: &mut Encoder<W>) -> Result<(), Error>
    where
        W: Writer,
    {
        encoder.encode_bitmap(self)
    }
}
