use crate::error::ErrorKind;
use crate::{Error, Fraction, Id, IntoId, Reader, Rectangle, Type};

mod sealed {
    use crate::id::IntoId;
    use crate::{DecodeUnsized, Fraction, Id, Rectangle};

    pub trait Sealed {}
    impl Sealed for bool {}
    impl<I> Sealed for Id<I> where I: IntoId {}
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
    #[doc(hidden)]
    const TYPE: Type;

    /// Decode a full typed value.
    #[doc(hidden)]
    fn decode(reader: impl Reader<'de>) -> Result<Self, Error>;

    /// Read the content of a type.
    #[doc(hidden)]
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
impl<'de> Decode<'de> for bool {
    const TYPE: Type = Type::BOOL;

    #[inline]
    fn decode(mut reader: impl Reader<'de>) -> Result<Self, Error> {
        let (size, ty) = reader.header()?;

        match ty {
            Type::BOOL if size == 4 => {
                let [value, _pad] = reader.array()?;
                Ok(value != 0)
            }
            _ => Err(Error::new(ErrorKind::Expected {
                expected: Type::BOOL,
                actual: ty,
            })),
        }
    }

    #[inline]
    fn read_content(mut reader: impl Reader<'de>) -> Result<Self, Error> {
        let [value, _pad] = reader.array()?;
        Ok(value != 0)
    }
}

/// [`Decode`] implementation for an [`IntoId`] type.
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
impl<'de, I> Decode<'de> for Id<I>
where
    I: IntoId,
{
    const TYPE: Type = Type::ID;

    #[inline]
    fn decode(mut reader: impl Reader<'de>) -> Result<Self, Error> {
        let (size, ty) = reader.header()?;

        match ty {
            Type::ID if size == 4 => {
                let [value, _pad] = reader.array()?;
                Ok(Id(I::from_id(value)))
            }
            _ => Err(Error::new(ErrorKind::Expected {
                expected: Type::ID,
                actual: ty,
            })),
        }
    }

    #[inline]
    fn read_content(mut reader: impl Reader<'de>) -> Result<Self, Error> {
        let [value, _pad] = reader.array()?;
        Ok(Id(I::from_id(value)))
    }
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
    fn decode(mut reader: impl Reader<'de>) -> Result<Self, Error> {
        let (size, ty) = reader.header()?;

        match ty {
            Type::INT if size == 4 => {
                let [value, _pad] = reader.array()?;
                Ok(value.cast_signed())
            }
            _ => Err(Error::new(ErrorKind::Expected {
                expected: Type::INT,
                actual: ty,
            })),
        }
    }

    #[inline]
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
    fn decode(mut reader: impl Reader<'de>) -> Result<Self, Error> {
        let (size, ty) = reader.header()?;

        match ty {
            Type::LONG if size == 8 => {
                let value = reader.read_u64()?.cast_signed();
                Ok(value)
            }
            _ => Err(Error::new(ErrorKind::Expected {
                expected: Type::LONG,
                actual: ty,
            })),
        }
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
    fn decode(mut reader: impl Reader<'de>) -> Result<Self, Error> {
        let (size, ty) = reader.header()?;

        match ty {
            Type::FLOAT if size == 4 => {
                let [value, _pad] = reader.array()?;
                Ok(f32::from_bits(value))
            }
            _ => Err(Error::new(ErrorKind::Expected {
                expected: Type::FLOAT,
                actual: ty,
            })),
        }
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
    fn decode(mut reader: impl Reader<'de>) -> Result<Self, Error> {
        let (size, ty) = reader.header()?;

        match ty {
            Type::DOUBLE if size == 8 => {
                let value = f64::from_bits(reader.read_u64()?);
                Ok(value)
            }
            _ => Err(Error::new(ErrorKind::Expected {
                expected: Type::DOUBLE,
                actual: ty,
            })),
        }
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
    fn decode(mut reader: impl Reader<'de>) -> Result<Self, Error> {
        let (size, ty) = reader.header()?;

        match ty {
            Type::RECTANGLE if size == 8 => {
                let [width, height] = reader.array()?;
                Ok(Rectangle::new(width, height))
            }
            _ => Err(Error::new(ErrorKind::Expected {
                expected: Type::RECTANGLE,
                actual: ty,
            })),
        }
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
    fn decode(mut reader: impl Reader<'de>) -> Result<Self, Error> {
        let (size, ty) = reader.header()?;

        match ty {
            Type::FRACTION if size == 8 => {
                let [num, denom] = reader.array()?;
                Ok(Fraction::new(num, denom))
            }
            _ => Err(Error::new(ErrorKind::Expected {
                expected: Type::FRACTION,
                actual: ty,
            })),
        }
    }

    #[inline]
    fn read_content(mut reader: impl Reader<'de>) -> Result<Self, Error> {
        let [num, denom] = reader.array()?;
        Ok(Fraction::new(num, denom))
    }
}
