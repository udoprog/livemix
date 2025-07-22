use crate::{Error, Fraction, Rectangle, Writer};

use super::{EncodeUnsized, Encoder};

mod sealed {
    use super::{EncodeUnsized, Fraction, Rectangle};

    pub trait Sealed {}
    impl Sealed for i32 {}
    impl Sealed for i64 {}
    impl Sealed for f32 {}
    impl Sealed for f64 {}
    impl Sealed for Rectangle {}
    impl Sealed for Fraction {}
    impl<E> Sealed for &E where E: ?Sized + EncodeUnsized {}
}

/// A trait for types that can be encoded.
pub trait Encode: Sized + self::sealed::Sealed {
    fn encode<W>(&self, encoder: &mut Encoder<W>) -> Result<(), Error>
    where
        W: Writer;
}

/// [`Encode`] implementation for `i32`.
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
impl Encode for i32 {
    #[inline]
    fn encode<W>(&self, encoder: &mut Encoder<W>) -> Result<(), Error>
    where
        W: Writer,
    {
        encoder.encode_int(*self)
    }
}

/// [`Encode`] implementation for `i64`.
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
impl Encode for i64 {
    #[inline]
    fn encode<W>(&self, encoder: &mut Encoder<W>) -> Result<(), Error>
    where
        W: Writer,
    {
        encoder.encode_long(*self)
    }
}

/// [`Encode`] implementation for `f32`.
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
impl Encode for f32 {
    #[inline]
    fn encode<W>(&self, encoder: &mut Encoder<W>) -> Result<(), Error>
    where
        W: Writer,
    {
        encoder.encode_float(*self)
    }
}

/// Decode implementation for `f64`.
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
impl Encode for f64 {
    #[inline]
    fn encode<W>(&self, encoder: &mut Encoder<W>) -> Result<(), Error>
    where
        W: Writer,
    {
        encoder.encode_double(*self)
    }
}

/// [`Encode`] implementation for `Rectangle`.
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
impl Encode for Rectangle {
    #[inline]
    fn encode<W>(&self, encoder: &mut Encoder<W>) -> Result<(), Error>
    where
        W: Writer,
    {
        encoder.encode_rectangle(*self)
    }
}

/// [`Encode`] a [`Fraction`].
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
impl Encode for Fraction {
    #[inline]
    fn encode<W>(&self, encoder: &mut Encoder<W>) -> Result<(), Error>
    where
        W: Writer,
    {
        encoder.encode_fraction(*self)
    }
}

impl<T> Encode for &T
where
    T: ?Sized + EncodeUnsized,
{
    #[inline]
    fn encode<W>(&self, encoder: &mut Encoder<W>) -> Result<(), Error>
    where
        W: Writer,
    {
        self.encode_unsized(encoder)
    }
}
