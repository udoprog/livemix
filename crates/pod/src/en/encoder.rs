use core::ffi::CStr;

use crate::error::ErrorKind;
use crate::id::IntoId;
use crate::{Bitmap, Error, Fraction, Id, Rectangle, Type, Writer};

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
    /// # Ok::<_, pod::Error>(())
    /// ```
    pub fn encode<T>(&mut self, value: T) -> Result<(), Error>
    where
        T: Encode,
    {
        value.encode(self.w.borrow_mut())
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
        value.encode_unsized(self.w.borrow_mut())
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
        value.encode(self.w.borrow_mut())
    }

    /// Encode an `id` value.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Encoder};
    /// use pod::id::MediaSubType;
    ///
    /// let mut buf = ArrayBuf::new();
    /// let mut encoder = Encoder::new(&mut buf);
    /// encoder.encode_id(MediaSubType::Opus)?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn encode_id(&mut self, value: impl IntoId) -> Result<(), Error> {
        Id(value).encode(self.w.borrow_mut())
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
        value.encode(self.w.borrow_mut())
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
        value.encode(self.w.borrow_mut())
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
        value.encode(self.w.borrow_mut())
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
        value.encode(self.w.borrow_mut())
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
        value.encode_unsized(self.w.borrow_mut())
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
        value.encode_unsized(self.w.borrow_mut())
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
        value.encode_unsized(self.w.borrow_mut())
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
        rectangle.encode(self.w.borrow_mut())
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
        fraction.encode(self.w.borrow_mut())
    }

    #[inline]
    pub fn encode_bitmap(&mut self, value: &Bitmap) -> Result<(), Error> {
        value.encode_unsized(self.w.borrow_mut())
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
