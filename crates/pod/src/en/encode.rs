use crate::{Error, Fraction, Rectangle, Type, Writer};

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
    /// The type of the encoded value.
    const TYPE: Type;

    /// The size in bytes of the encoded value.
    fn size(&self) -> usize;

    /// Encode the value into the encoder.
    fn encode<W>(&self, encoder: &mut Encoder<W>) -> Result<(), Error>
    where
        W: Writer;

    /// Write the content of a type.
    fn write_content(&self, writer: impl Writer) -> Result<(), Error>;
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
    const TYPE: Type = Type::INT;

    #[inline]
    fn size(&self) -> usize {
        4
    }

    #[inline]
    fn encode<W>(&self, encoder: &mut Encoder<W>) -> Result<(), Error>
    where
        W: Writer,
    {
        encoder.encode_int(*self)
    }

    #[inline]
    fn write_content(&self, mut writer: impl Writer) -> Result<(), Error> {
        writer.write_words(&[self.cast_unsigned(), 0])
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
    const TYPE: Type = Type::LONG;

    #[inline]
    fn size(&self) -> usize {
        8
    }

    #[inline]
    fn encode<W>(&self, encoder: &mut Encoder<W>) -> Result<(), Error>
    where
        W: Writer,
    {
        encoder.encode_long(*self)
    }

    #[inline]
    fn write_content(&self, mut writer: impl Writer) -> Result<(), Error> {
        writer.write_u64(self.cast_unsigned())
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
    const TYPE: Type = Type::FLOAT;

    #[inline]
    fn size(&self) -> usize {
        4
    }

    #[inline]
    fn encode<W>(&self, encoder: &mut Encoder<W>) -> Result<(), Error>
    where
        W: Writer,
    {
        encoder.encode_float(*self)
    }

    #[inline]
    fn write_content(&self, mut writer: impl Writer) -> Result<(), Error> {
        writer.write_words(&[self.to_bits(), 0])
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
    const TYPE: Type = Type::DOUBLE;

    #[inline]
    fn size(&self) -> usize {
        8
    }

    #[inline]
    fn encode<W>(&self, encoder: &mut Encoder<W>) -> Result<(), Error>
    where
        W: Writer,
    {
        encoder.encode_double(*self)
    }

    #[inline]
    fn write_content(&self, mut writer: impl Writer) -> Result<(), Error> {
        writer.write_u64(self.to_bits())
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
    const TYPE: Type = Type::RECTANGLE;

    #[inline]
    fn size(&self) -> usize {
        8
    }

    #[inline]
    fn encode<W>(&self, encoder: &mut Encoder<W>) -> Result<(), Error>
    where
        W: Writer,
    {
        encoder.encode_rectangle(*self)
    }

    #[inline]
    fn write_content(&self, mut writer: impl Writer) -> Result<(), Error> {
        writer.write_words(&[self.width, self.height])
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
    const TYPE: Type = Type::FRACTION;

    #[inline]
    fn size(&self) -> usize {
        8
    }

    #[inline]
    fn encode<W>(&self, encoder: &mut Encoder<W>) -> Result<(), Error>
    where
        W: Writer,
    {
        encoder.encode_fraction(*self)
    }

    #[inline]
    fn write_content(&self, mut writer: impl Writer) -> Result<(), Error> {
        writer.write_words(&[self.num, self.denom])
    }
}

/// [`Encode`] an unsized type through a reference.
///
/// # Examples
///
/// ```
/// use pod::{ArrayBuf, Encoder, Decoder};
///
/// let mut buf = ArrayBuf::new();
/// let mut encoder = Encoder::new(&mut buf);
/// encoder.encode(&b"hello world"[..])?;
///
/// let mut de = Decoder::new(buf.as_reader_slice());
/// let bytes: &[u8] = de.decode_borrowed_bytes()?;
/// assert_eq!(bytes, b"hello world");
/// # Ok::<_, pod::Error>(())
/// ```
impl<T> Encode for &T
where
    T: ?Sized + EncodeUnsized,
{
    const TYPE: Type = T::TYPE;

    #[inline]
    fn size(&self) -> usize {
        EncodeUnsized::size(*self)
    }

    #[inline]
    fn encode<W>(&self, encoder: &mut Encoder<W>) -> Result<(), Error>
    where
        W: Writer,
    {
        self.encode_unsized(encoder)
    }

    #[inline]
    fn write_content(&self, writer: impl Writer) -> Result<(), Error> {
        T::write_content(self, writer)
    }
}
