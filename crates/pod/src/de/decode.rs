use crate::{Error, Fraction, Reader, Rectangle, Type};

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
    /// The type of the decoded value.
    const TYPE: Type;

    /// Decode a full typed value.
    fn decode<R>(decoder: &mut Decoder<R>) -> Result<Self, Error>
    where
        R: Reader<'de>;

    /// Read the content of a type.
    fn read_content(reader: impl Reader<'de>) -> Result<Self, Error>;
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
    const TYPE: Type = Type::INT;

    #[inline]
    #[doc(hidden)]
    fn decode<W>(decoder: &mut Decoder<W>) -> Result<Self, Error>
    where
        W: Reader<'de>,
    {
        decoder.decode_int()
    }

    #[inline]
    #[doc(hidden)]
    fn read_content(mut reader: impl Reader<'de>) -> Result<Self, Error> {
        let [value, _pad] = reader.array()?;
        Ok(value.cast_signed())
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
    const TYPE: Type = Type::LONG;

    #[inline]
    fn decode<W>(decoder: &mut Decoder<W>) -> Result<Self, Error>
    where
        W: Reader<'de>,
    {
        decoder.decode_long()
    }

    #[inline]
    fn read_content(mut reader: impl Reader<'de>) -> Result<Self, Error> {
        Ok(reader.read_u64()?.cast_signed())
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
    const TYPE: Type = Type::FLOAT;

    #[inline]
    fn decode<W>(decoder: &mut Decoder<W>) -> Result<Self, Error>
    where
        W: Reader<'de>,
    {
        decoder.decode_float()
    }

    #[inline]
    fn read_content(mut reader: impl Reader<'de>) -> Result<Self, Error> {
        let [value, _pad] = reader.array()?;
        Ok(f32::from_bits(value))
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
    const TYPE: Type = Type::DOUBLE;

    #[inline]
    fn decode<W>(decoder: &mut Decoder<W>) -> Result<Self, Error>
    where
        W: Reader<'de>,
    {
        decoder.decode_double()
    }

    #[inline]
    fn read_content(mut reader: impl Reader<'de>) -> Result<Self, Error> {
        Ok(f64::from_bits(reader.read_u64()?))
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
    const TYPE: Type = Type::RECTANGLE;

    #[inline]
    fn decode<W>(decoder: &mut Decoder<W>) -> Result<Self, Error>
    where
        W: Reader<'de>,
    {
        decoder.decode_rectangle()
    }

    #[inline]
    fn read_content(mut reader: impl Reader<'de>) -> Result<Self, Error> {
        let [width, height] = reader.array()?;
        Ok(Rectangle::new(width, height))
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
    const TYPE: Type = Type::FRACTION;

    #[inline]
    fn decode<W>(decoder: &mut Decoder<W>) -> Result<Self, Error>
    where
        W: Reader<'de>,
    {
        decoder.decode_fraction()
    }

    #[inline]
    fn read_content(mut reader: impl Reader<'de>) -> Result<Self, Error> {
        let [num, denom] = reader.array()?;
        Ok(Fraction::new(num, denom))
    }
}
