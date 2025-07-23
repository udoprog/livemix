use core::ffi::CStr;

use crate::error::ErrorKind;
use crate::{Bitmap, Decoder, Error, Reader, Type, Visitor};

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
    const TYPE: Type;

    #[doc(hidden)]
    fn decode_unsized<R>(decoder: &mut Decoder<R>) -> Result<&'de Self, Error>
    where
        R: Reader<'de>;

    #[doc(hidden)]
    fn read_content<V>(reader: impl Reader<'de>, size: usize, visitor: V) -> Result<V::Ok, Error>
    where
        V: Visitor<'de, Self>;
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
    const TYPE: Type = Type::STRING;

    #[inline]
    fn decode_unsized<R>(decoder: &mut Decoder<R>) -> Result<&'de Self, Error>
    where
        R: Reader<'de>,
    {
        decoder.decode_borrowed_c_str()
    }

    #[inline]
    fn read_content<V>(
        mut reader: impl Reader<'de>,
        size: usize,
        visitor: V,
    ) -> Result<V::Ok, Error>
    where
        V: Visitor<'de, Self>,
    {
        struct LocalVisitor<V> {
            visitor: V,
        }

        impl<'de, V> Visitor<'de, [u8]> for LocalVisitor<V>
        where
            V: Visitor<'de, CStr>,
        {
            type Ok = V::Ok;

            #[inline]
            fn visit_borrowed(self, bytes: &'de [u8]) -> Result<Self::Ok, Error> {
                let Ok(str) = CStr::from_bytes_with_nul(bytes) else {
                    return Err(Error::new(ErrorKind::NonTerminatedString));
                };

                self.visitor.visit_borrowed(str)
            }

            #[inline]
            fn visit_ref(self, bytes: &[u8]) -> Result<Self::Ok, Error> {
                let Ok(str) = CStr::from_bytes_with_nul(bytes) else {
                    return Err(Error::new(ErrorKind::NonTerminatedString));
                };

                self.visitor.visit_ref(str)
            }
        }

        let visitor = LocalVisitor { visitor };
        reader.read_bytes(size, visitor)
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
    const TYPE: Type = Type::STRING;

    #[inline]
    fn decode_unsized<R>(decoder: &mut Decoder<R>) -> Result<&'de Self, Error>
    where
        R: Reader<'de>,
    {
        decoder.decode_borrowed_str()
    }

    #[inline]
    fn read_content<V>(
        mut reader: impl Reader<'de>,
        size: usize,
        visitor: V,
    ) -> Result<V::Ok, Error>
    where
        V: Visitor<'de, Self>,
    {
        struct LocalVisitor<V>(V);

        impl<'de, V> Visitor<'de, [u8]> for LocalVisitor<V>
        where
            V: Visitor<'de, str>,
        {
            type Ok = V::Ok;

            #[inline]
            fn visit_borrowed(self, bytes: &'de [u8]) -> Result<Self::Ok, Error> {
                self.0.visit_borrowed(decode_string(bytes)?)
            }

            #[inline]
            fn visit_ref(self, bytes: &[u8]) -> Result<Self::Ok, Error> {
                self.0.visit_ref(decode_string(bytes)?)
            }
        }

        reader.read_bytes(size, LocalVisitor(visitor))
    }
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
    const TYPE: Type = Type::BYTES;

    #[inline]
    fn decode_unsized<R>(decoder: &mut Decoder<R>) -> Result<&'de Self, Error>
    where
        R: Reader<'de>,
    {
        decoder.decode_borrowed_bytes()
    }

    #[inline]
    fn read_content<V>(
        mut reader: impl Reader<'de>,
        size: usize,
        visitor: V,
    ) -> Result<V::Ok, Error>
    where
        V: Visitor<'de, Self>,
    {
        reader.read_bytes(size, visitor)
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
    const TYPE: Type = Type::BITMAP;

    #[inline]
    fn decode_unsized<R>(decoder: &mut Decoder<R>) -> Result<&'de Self, Error>
    where
        R: Reader<'de>,
    {
        decoder.decode_borrowed_bitmap()
    }

    #[inline]
    fn read_content<V>(
        mut reader: impl Reader<'de>,
        size: usize,
        visitor: V,
    ) -> Result<V::Ok, Error>
    where
        V: Visitor<'de, Self>,
    {
        struct LocalVisitor<V>(V);

        impl<'de, V> Visitor<'de, [u8]> for LocalVisitor<V>
        where
            V: Visitor<'de, Bitmap>,
        {
            type Ok = V::Ok;

            #[inline]
            fn visit_borrowed(self, value: &'de [u8]) -> Result<Self::Ok, Error> {
                self.0.visit_borrowed(Bitmap::new(value))
            }

            #[inline]
            fn visit_ref(self, value: &[u8]) -> Result<Self::Ok, Error> {
                self.0.visit_ref(Bitmap::new(value))
            }
        }

        reader.read_bytes(size, LocalVisitor(visitor))
    }
}

fn decode_string(bytes: &[u8]) -> Result<&str, Error> {
    let bytes = match bytes {
        [head @ .., 0] => head,
        _ => return Err(Error::new(ErrorKind::NonTerminatedString)),
    };

    let Ok(str) = str::from_utf8(bytes) else {
        return Err(Error::new(ErrorKind::NotUtf8));
    };

    Ok(str)
}
