use core::ffi::CStr;

use crate::error::ErrorKind;
use crate::{Bitmap, Error, Fraction, Rectangle, Type, Writer};

use super::{Encode, EncodeArray, EncodeUnsized};

/// A POD (Plain Old Data) encoder.
pub struct Encoder<W> {
    pub(crate) w: W,
}

impl<W> Encoder<W> {
    /// Construct a new encoder.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Encoder};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let mut encoder = Encoder::new(&mut buf);
    /// ```
    #[inline]
    pub const fn new(w: W) -> Self {
        Encoder { w }
    }
}

impl<W> Encoder<W>
where
    W: Writer,
{
    /// Encode a value into the encoder.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Encoder};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let mut encoder = Encoder::new(&mut buf);
    /// encoder.encode(10i32)?;
    /// encoder.encode(&b"hello world"[..])?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    pub fn encode<T>(&mut self, value: T) -> Result<(), Error>
    where
        T: Encode,
    {
        value.encode(self)
    }

    /// Encode an unsized value into the encoder.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Encoder};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let mut encoder = Encoder::new(&mut buf);
    /// encoder.encode_unsized(&b"hello world"[..])?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    pub fn encode_unsized<T>(&mut self, value: &T) -> Result<(), Error>
    where
        T: ?Sized + EncodeUnsized,
    {
        value.encode_unsized(self)
    }

    /// Encode a `None` value.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Encoder};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let mut encoder = Encoder::new(&mut buf);
    /// encoder.encode_none()?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn encode_none(&mut self) -> Result<(), Error> {
        self.w.write_words(&[0, Type::NONE.into_u32()])?;
        Ok(())
    }

    /// Encode a `bool` value.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Encoder};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let mut encoder = Encoder::new(&mut buf);
    /// encoder.encode_bool(true)?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn encode_bool(&mut self, value: bool) -> Result<(), Error> {
        self.w
            .write_words(&[4, Type::BOOL.into_u32(), if value { 1 } else { 0 }, 0])?;
        Ok(())
    }

    /// Encode an `id` value.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Encoder};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let mut encoder = Encoder::new(&mut buf);
    /// encoder.encode_id(42)?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn encode_id(&mut self, value: u32) -> Result<(), Error> {
        self.w.write_words(&[4, Type::ID.into_u32(), value, 0])?;
        Ok(())
    }

    /// Encode a signed 32-bit integer.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Encoder};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let mut encoder = Encoder::new(&mut buf);
    /// encoder.encode_int(-42)?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn encode_int(&mut self, value: i32) -> Result<(), Error> {
        self.w
            .write_words(&[4, Type::INT.into_u32(), value.cast_unsigned(), 0])?;
        Ok(())
    }

    /// Encode a signed 64-bit integer.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Encoder};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let mut encoder = Encoder::new(&mut buf);
    /// encoder.encode_long(-42)?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn encode_long(&mut self, value: i64) -> Result<(), Error> {
        self.w.write_words(&[8, Type::LONG.into_u32()])?;
        self.w.write_u64(value.cast_unsigned())?;
        Ok(())
    }

    /// Encode a signed 32-bit float.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Encoder};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let mut encoder = Encoder::new(&mut buf);
    /// encoder.encode_float(-42.42)?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn encode_float(&mut self, value: f32) -> Result<(), Error> {
        self.w
            .write_words(&[4, Type::FLOAT.into_u32(), value.to_bits(), 0])?;
        Ok(())
    }

    /// Encode a signed 64-bit float.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Encoder};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let mut encoder = Encoder::new(&mut buf);
    /// encoder.encode_double(-42.42)?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn encode_double(&mut self, value: f64) -> Result<(), Error> {
        self.w.write_words(&[8, Type::DOUBLE.into_u32()])?;
        self.w.write_u64(value.to_bits())?;
        Ok(())
    }

    /// Encode a null-terminated C-string.
    ///
    /// # Examples
    ///
    /// ```
    /// use core::ffi::CStr;
    /// use pod::{ArrayBuf, Encoder};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let mut encoder = Encoder::new(&mut buf);
    /// encoder.encode_c_str(c"hello world")?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn encode_c_str(&mut self, value: &CStr) -> Result<(), Error> {
        let bytes = value.to_bytes_with_nul();

        let Ok(len) = u32::try_from(bytes.len()) else {
            return Err(Error::new(ErrorKind::SizeOverflow));
        };

        self.w.write_words(&[len, Type::STRING.into_u32()])?;
        self.w.write_bytes(bytes)?;
        Ok(())
    }

    /// Encode a UTF-8 string.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Encoder};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let mut encoder = Encoder::new(&mut buf);
    /// encoder.encode_str("hello world")?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn encode_str(&mut self, value: &str) -> Result<(), Error> {
        let bytes = value.as_bytes();

        let Some(len) = bytes
            .len()
            .checked_add(1)
            .and_then(|v| u32::try_from(v).ok())
        else {
            return Err(Error::new(ErrorKind::SizeOverflow));
        };

        if bytes.contains(&0) {
            return Err(Error::new(ErrorKind::NullContainingString));
        }

        self.w.write_words(&[len, Type::STRING.into_u32()])?;
        self.w.write_bytes_with_nul(bytes)?;
        Ok(())
    }

    /// Encode bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Encoder};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let mut encoder = Encoder::new(&mut buf);
    /// encoder.encode_bytes(b"hello world")?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn encode_bytes(&mut self, value: &[u8]) -> Result<(), Error> {
        let Ok(len) = u32::try_from(value.len()) else {
            return Err(Error::new(ErrorKind::SizeOverflow));
        };

        self.w.write_words(&[len, Type::BYTES.into_u32()])?;
        self.w.write_bytes(value)?;
        Ok(())
    }

    /// Encode a rectangle.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Encoder, Rectangle};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let mut encoder = Encoder::new(&mut buf);
    /// encoder.encode_rectangle(Rectangle::new(2, 3))?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn encode_rectangle(&mut self, rectangle: Rectangle) -> Result<(), Error> {
        self.w.write_words(&[
            8,
            Type::RECTANGLE.into_u32(),
            rectangle.width,
            rectangle.height,
        ])?;
        Ok(())
    }

    /// Encode a fraction.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Encoder, Fraction};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let mut encoder = Encoder::new(&mut buf);
    /// encoder.encode_fraction(Fraction::new(2, 3))?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn encode_fraction(&mut self, fraction: Fraction) -> Result<(), Error> {
        self.w
            .write_words(&[8, Type::FRACTION.into_u32(), fraction.num, fraction.denom])?;
        Ok(())
    }

    /// Encode a bitmap.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Bitmap, Encoder};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let mut encoder = Encoder::new(&mut buf);
    /// encoder.encode_bitmap(Bitmap::new(b"hello world"))?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn encode_bitmap(&mut self, value: &Bitmap) -> Result<(), Error> {
        let value = value.as_bytes();

        let Ok(len) = u32::try_from(value.len()) else {
            return Err(Error::new(ErrorKind::SizeOverflow));
        };

        self.w.write_words(&[len, Type::BITMAP.into_u32()])?;
        self.w.write_bytes(value)?;
        Ok(())
    }

    /// Encode an array with the given type.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Encoder, Type};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let mut encoder = Encoder::new(&mut buf);
    /// let mut array = encoder.encode_array(Type::INT)?;
    ///
    /// array.encode(1i32)?;
    /// array.encode(2i32)?;
    /// array.encode(3i32)?;
    ///
    /// array.close()?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn encode_array(&mut self, child_type: Type) -> Result<EncodeArray<W::Mut<'_>>, Error> {
        let Some(child_size) = child_type.size() else {
            return Err(Error::new(ErrorKind::UnsizedTypeInArray { ty: child_type }));
        };

        let mut writer = self.w.borrow_mut();
        let pos = writer.reserve_words(&[0, 0, 0, 0])?;
        Ok(EncodeArray::new(writer, child_size, child_type, pos))
    }

    /// Encode an array with elements of an unsized type.
    ///
    /// The `len` specified will be used to determine the maximum size of
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Encoder, Type};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let mut encoder = Encoder::new(&mut buf);
    /// let mut array = encoder.encode_unsized_array(Type::STRING, 4)?;
    ///
    /// array.encode_unsized("foo")?;
    /// array.encode_unsized("bar")?;
    /// array.encode_unsized("baz")?;
    ///
    /// array.close()?;
    ///
    /// assert_eq!(buf.as_reader_slice().len(), 10);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn encode_unsized_array(
        &mut self,
        child_type: Type,
        len: usize,
    ) -> Result<EncodeArray<W::Mut<'_>>, Error> {
        if let Some(child_size) = child_type.size() {
            if child_size != len {
                return Err(Error::new(ErrorKind::ArrayChildSizeMismatch {
                    actual: len,
                    expected: child_size,
                }));
            }
        };

        let mut writer = self.w.borrow_mut();
        let pos = writer.reserve_words(&[0, 0, 0, 0])?;
        Ok(EncodeArray::new(writer, len, child_type, pos))
    }
}
