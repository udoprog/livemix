use crate::{Error, Fraction, Reader, Rectangle};

use super::{DecodeUnsized, Decoder};

mod sealed {
    use super::{DecodeUnsized, Fraction, Rectangle};

    pub trait Sealed {}
    impl Sealed for i32 {}
    impl Sealed for i64 {}
    impl Sealed for f32 {}
    impl Sealed for f64 {}
    impl Sealed for Rectangle {}
    impl Sealed for Fraction {}
    impl<'de, E> Sealed for &E where E: ?Sized + DecodeUnsized<'de> {}
}

/// A trait for types that can be decoded.
pub trait Decode<'de>: Sized + self::sealed::Sealed {
    fn decode<W>(decoder: &mut Decoder<W>) -> Result<Self, Error>
    where
        W: Reader<'de>;
}

/// [`Decode`] implementation for `i32`.
///
/// # Examples
///
/// ```
/// use pod::{ArrayBuf, Encoder, Decoder};
///
/// let mut buf = ArrayBuf::new();
/// let mut encoder = Encoder::new(&mut buf);
/// encoder.encode(10i32)?;
///
/// let mut de = Decoder::new(buf.as_reader_slice());
/// let value: i32 = de.decode()?;
/// assert_eq!(value, 10i32);
/// # Ok::<_, pod::Error>(())
/// ```
impl<'de> Decode<'de> for i32 {
    #[inline]
    fn decode<W>(decoder: &mut Decoder<W>) -> Result<Self, Error>
    where
        W: Reader<'de>,
    {
        decoder.decode_int()
    }
}

/// [`Decode`] implementation for `i64`.
///
/// # Examples
///
/// ```
/// use pod::{ArrayBuf, Encoder, Decoder};
///
/// let mut buf = ArrayBuf::new();
/// let mut encoder = Encoder::new(&mut buf);
/// encoder.encode(10i64)?;
///
/// let mut de = Decoder::new(buf.as_reader_slice());
/// let value: i64 = de.decode()?;
/// assert_eq!(value, 10i64);
/// # Ok::<_, pod::Error>(())
/// ```
impl<'de> Decode<'de> for i64 {
    #[inline]
    fn decode<W>(decoder: &mut Decoder<W>) -> Result<Self, Error>
    where
        W: Reader<'de>,
    {
        decoder.decode_long()
    }
}

/// [`Decode`] implementation for `f32`.
///
/// # Examples
///
/// ```
/// use pod::{ArrayBuf, Encoder, Decoder};
///
/// let mut buf = ArrayBuf::new();
/// let mut encoder = Encoder::new(&mut buf);
/// encoder.encode(42.42f32)?;
///
/// let mut de = Decoder::new(buf.as_reader_slice());
/// let value: f32 = de.decode()?;
/// assert_eq!(value, 42.42f32);
/// # Ok::<_, pod::Error>(())
/// ```
impl<'de> Decode<'de> for f32 {
    #[inline]
    fn decode<W>(decoder: &mut Decoder<W>) -> Result<Self, Error>
    where
        W: Reader<'de>,
    {
        decoder.decode_float()
    }
}

/// [`Decode`] implementation for `f64`.
///
/// # Examples
///
/// ```
/// use pod::{ArrayBuf, Encoder, Decoder};
///
/// let mut buf = ArrayBuf::new();
/// let mut encoder = Encoder::new(&mut buf);
/// encoder.encode(42.42f64)?;
///
/// let mut de = Decoder::new(buf.as_reader_slice());
/// let value: f64 = de.decode()?;
/// assert_eq!(value, 42.42f64);
/// # Ok::<_, pod::Error>(())
/// ```
impl<'de> Decode<'de> for f64 {
    #[inline]
    fn decode<W>(decoder: &mut Decoder<W>) -> Result<Self, Error>
    where
        W: Reader<'de>,
    {
        decoder.decode_double()
    }
}

/// [`Decode`] implementation for `Rectangle`.
///
/// # Examples
///
/// ```
/// use pod::{ArrayBuf, Decoder, Encoder, Rectangle};
///
/// let mut buf = ArrayBuf::new();
/// let mut en = Encoder::new(&mut buf);
///
/// en.encode(Rectangle::new(100, 200))?;
///
/// let mut decoder = Decoder::new(buf.as_reader_slice());
/// assert_eq!(decoder.decode::<Rectangle>()?, Rectangle::new(100, 200));
/// # Ok::<_, pod::Error>(())
/// ```
impl<'de> Decode<'de> for Rectangle {
    #[inline]
    fn decode<W>(decoder: &mut Decoder<W>) -> Result<Self, Error>
    where
        W: Reader<'de>,
    {
        decoder.decode_rectangle()
    }
}

/// [`Decode`] a [`Fraction`].
///
/// # Examples
///
/// ```
/// use pod::{ArrayBuf, Decoder, Encoder, Fraction};
///
/// let mut buf = ArrayBuf::new();
/// let mut en = Encoder::new(&mut buf);
///
/// en.encode(Fraction::new(800, 600))?;
///
/// let mut decoder = Decoder::new(buf.as_reader_slice());
/// assert_eq!(decoder.decode::<Fraction>()?, Fraction::new(800, 600));
/// # Ok::<_, pod::Error>(())
/// ```
impl<'de> Decode<'de> for Fraction {
    #[inline]
    fn decode<W>(decoder: &mut Decoder<W>) -> Result<Self, Error>
    where
        W: Reader<'de>,
    {
        decoder.decode_fraction()
    }
}

/// [`Decode`] an unsized type through a reference.
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
/// let bytes: &[u8] = de.decode()?;
/// assert_eq!(bytes, b"hello world");
/// # Ok::<_, pod::Error>(())
/// ```
impl<'de, T> Decode<'de> for &'de T
where
    T: ?Sized + DecodeUnsized<'de>,
{
    #[inline]
    fn decode<W>(decoder: &mut Decoder<W>) -> Result<Self, Error>
    where
        W: Reader<'de>,
    {
        decoder.decode_unsized()
    }
}
