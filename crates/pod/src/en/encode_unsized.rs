use core::ffi::CStr;

use crate::{Bitmap, Error, Type, Writer};

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
    /// The type of the encoded value.
    const TYPE: Type;

    /// The size in bytes of the unsized value.
    fn size(&self) -> usize;

    #[doc(hidden)]
    fn encode_unsized<W>(&self, encoder: &mut Encoder<W>) -> Result<(), Error>
    where
        W: Writer;

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
/// let bytes: &[u8] = de.decode_unsized()?;
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
    fn encode_unsized<W>(&self, encoder: &mut Encoder<W>) -> Result<(), Error>
    where
        W: Writer,
    {
        encoder.encode_bytes(self)
    }

    #[inline]
    fn write_content(&self, mut writer: impl Writer) -> Result<(), Error> {
        writer.write_bytes(self)?;
        Ok(())
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
    const TYPE: Type = Type::STRING;

    #[inline]
    fn size(&self) -> usize {
        self.to_bytes_with_nul().len()
    }

    #[inline]
    fn encode_unsized<W>(&self, encoder: &mut Encoder<W>) -> Result<(), Error>
    where
        W: Writer,
    {
        encoder.encode_c_str(self)
    }

    #[inline]
    fn write_content(&self, mut writer: impl Writer) -> Result<(), Error> {
        writer.write_bytes(self.to_bytes_with_nul())?;
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
/// let bytes: &str = de.decode_unsized()?;
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
    fn encode_unsized<W>(&self, encoder: &mut Encoder<W>) -> Result<(), Error>
    where
        W: Writer,
    {
        encoder.encode_str(self)
    }

    #[inline]
    fn write_content(&self, mut writer: impl Writer) -> Result<(), Error> {
        writer.write_bytes_with_nul(self.as_bytes())?;
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
/// let bitmap: &Bitmap = de.decode_unsized()?;
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
    fn encode_unsized<W>(&self, encoder: &mut Encoder<W>) -> Result<(), Error>
    where
        W: Writer,
    {
        encoder.encode_bitmap(self)
    }

    #[inline]
    fn write_content(&self, mut writer: impl Writer) -> Result<(), Error> {
        writer.write_bytes(self.as_bytes())?;
        Ok(())
    }
}
