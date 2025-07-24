use crate::{EncodeUnsized, Error, Fraction, Id, IntoId, Rectangle, Type, Writer};

mod sealed {
    use crate::id::IntoId;
    use crate::{EncodeUnsized, Fraction, Id, Rectangle};

    pub trait Sealed {}
    impl Sealed for bool {}
    impl<I> Sealed for Id<I> where I: IntoId {}
    impl Sealed for i32 {}
    impl Sealed for i64 {}
    impl Sealed for f32 {}
    impl Sealed for f64 {}
    impl Sealed for Rectangle {}
    impl Sealed for Fraction {}
    impl<const N: usize> Sealed for [u8; N] {}
    impl<E> Sealed for &E where E: ?Sized + EncodeUnsized {}
}

/// A trait for types that can be encoded.
pub trait Encode: Sized + self::sealed::Sealed {
    /// The type of the encoded value.
    const TYPE: Type;

    /// The size in bytes of the encoded value.
    fn size(&self) -> usize;

    /// Encode the value into the writer.
    fn encode(&self, writer: impl Writer) -> Result<(), Error>;

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
/// encoder.encode(true)?;
///
/// let mut de = Decoder::new(buf.as_reader_slice());
/// let value: bool = de.decode()?;
/// assert_eq!(value, true);
/// # Ok::<_, pod::Error>(())
/// ```
impl Encode for bool {
    const TYPE: Type = Type::BOOL;

    #[inline]
    fn size(&self) -> usize {
        4
    }

    #[inline]
    fn encode(&self, mut writer: impl Writer) -> Result<(), Error> {
        writer.write_words(&[4, Type::BOOL.into_u32(), if *self { 1 } else { 0 }, 0])
    }

    #[inline]
    fn write_content(&self, mut writer: impl Writer) -> Result<(), Error> {
        writer.write_words(&[if *self { 1 } else { 0 }, 0])
    }
}

/// [`Encode`] implementation for any type that can be converted into an `Id`.
///
/// # Examples
///
/// ```
/// use pod::{ArrayBuf, Encoder, Decoder};
/// use pod::id::{Id, MediaSubType};
///
/// let mut buf = ArrayBuf::new();
/// let mut encoder = Encoder::new(&mut buf);
/// encoder.encode(Id(MediaSubType::Opus))?;
///
/// let mut de = Decoder::new(buf.as_reader_slice());
/// let Id(value): Id<MediaSubType> = de.decode()?;
/// assert_eq!(value, MediaSubType::Opus);
/// # Ok::<_, pod::Error>(())
/// ```
impl<I> Encode for Id<I>
where
    I: IntoId,
{
    const TYPE: Type = Type::ID;

    #[inline]
    fn size(&self) -> usize {
        4
    }

    #[inline]
    fn encode(&self, mut writer: impl Writer) -> Result<(), Error> {
        writer.write_words(&[4, Type::ID.into_u32(), self.0.into_id(), 0])
    }

    #[inline]
    fn write_content(&self, mut writer: impl Writer) -> Result<(), Error> {
        writer.write_words(&[self.0.into_id(), 0])
    }
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
    fn encode(&self, mut writer: impl Writer) -> Result<(), Error> {
        writer.write_words(&[4, Type::INT.into_u32(), self.cast_unsigned(), 0])
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
    fn encode(&self, mut writer: impl Writer) -> Result<(), Error> {
        writer.write_words(&[8, Type::LONG.into_u32()])?;
        writer.write(&self.cast_unsigned())?;
        Ok(())
    }

    #[inline]
    fn write_content(&self, mut writer: impl Writer) -> Result<(), Error> {
        writer.write(&self.cast_unsigned())
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
    fn encode(&self, mut writer: impl Writer) -> Result<(), Error> {
        writer.write_words(&[4, Type::FLOAT.into_u32(), self.to_bits(), 0])
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
    fn encode(&self, mut writer: impl Writer) -> Result<(), Error> {
        writer.write_words(&[8, Type::DOUBLE.into_u32()])?;
        writer.write(&self.to_bits())?;
        Ok(())
    }

    #[inline]
    fn write_content(&self, mut writer: impl Writer) -> Result<(), Error> {
        writer.write(&self.to_bits())
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
    fn encode(&self, mut writer: impl Writer) -> Result<(), Error> {
        writer.write_words(&[8, Type::RECTANGLE.into_u32(), self.width, self.height])
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
    fn encode(&self, mut writer: impl Writer) -> Result<(), Error> {
        writer.write_words(&[8, Type::FRACTION.into_u32(), self.num, self.denom])
    }

    #[inline]
    fn write_content(&self, mut writer: impl Writer) -> Result<(), Error> {
        writer.write_words(&[self.num, self.denom])
    }
}

/// [`Encode`] a an array of bytes `[u8; N]`.
///
/// # Examples
///
/// ```
/// use pod::{ArrayBuf, Decoder, Encoder, Fraction};
///
/// let mut buf = ArrayBuf::new();
/// let mut en = Encoder::new(&mut buf);
///
/// en.encode(*b"hello world")?;
///
/// let mut decoder = Decoder::new(buf.as_reader_slice());
/// assert_eq!(decoder.decode_borrowed::<[u8]>()?, b"hello world");
/// # Ok::<_, pod::Error>(())
/// ```
impl<const N: usize> Encode for [u8; N] {
    const TYPE: Type = Type::BYTES;

    #[inline]
    fn size(&self) -> usize {
        N
    }

    #[inline]
    fn encode(&self, writer: impl Writer) -> Result<(), Error> {
        <[u8]>::encode_unsized(self, writer)
    }

    #[inline]
    fn write_content(&self, writer: impl Writer) -> Result<(), Error> {
        <[u8]>::write_content(self, writer)
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
/// assert_eq!(de.decode_borrowed::<[u8]>()?, b"hello world");
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
    fn encode(&self, writer: impl Writer) -> Result<(), Error> {
        self.encode_unsized(writer)
    }

    #[inline]
    fn write_content(&self, writer: impl Writer) -> Result<(), Error> {
        T::write_content(self, writer)
    }
}
