use core::ffi::CStr;

#[cfg(feature = "alloc")]
use alloc::borrow::ToOwned;
#[cfg(feature = "alloc")]
use alloc::ffi::CString;
#[cfg(feature = "alloc")]
use alloc::string::String;
#[cfg(feature = "alloc")]
use alloc::vec::Vec;

#[cfg(feature = "alloc")]
use crate::OwnedBitmap;
use crate::error::ErrorKind;
use crate::utils::Align;
use crate::{Bitmap, Error, Fraction, IntoId, Reader, Rectangle, Type, Visitor};

use super::{Decode, DecodeArray, DecodeUnsized};

/// A POD (Plain Old Data) decoder.
pub struct Decoder<R> {
    r: R,
}

impl<'de, R> Decoder<R>
where
    R: Reader<'de>,
{
    /// Encode a value into the encoder.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Encoder, Decoder};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let mut encoder = Encoder::new(&mut buf);
    /// encoder.encode(10i32)?;
    /// encoder.encode(&b"hello world"[..])?;
    ///
    /// let mut de = Decoder::new(buf.as_reader_slice());
    /// let value: i32 = de.decode()?;
    /// assert_eq!(value, 10i32);
    /// # Ok::<_, pod::Error>(())
    /// ```
    pub fn decode<T>(&mut self) -> Result<T, Error>
    where
        T: Decode<'de>,
    {
        T::decode(self)
    }

    /// Decode an unsized value into the encoder.
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
    pub fn decode_unsized<T>(&mut self) -> Result<&'de T, Error>
    where
        T: ?Sized + DecodeUnsized<'de>,
    {
        T::decode_unsized(self)
    }

    /// Construct a new decoder.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Decoder, Encoder};
    ///
    /// let mut buf = ArrayBuf::new();
    /// assert!(Decoder::new(buf.as_reader_slice()).decode_option().is_err());
    ///
    /// let mut en = Encoder::new(&mut buf);
    /// en.encode_bool(true)?;
    /// let mut de = Decoder::new(buf.as_reader_slice());
    ///
    /// assert!(de.decode_bool()?);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn new(r: R) -> Self {
        Self { r }
    }

    /// Decode an optional value.
    ///
    /// This returns `None` if the encoded value is `None`, otherwise a decoder
    /// for the value is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Decoder, Encoder};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let mut en = Encoder::new(&mut buf);
    ///
    /// en.encode_none()?;
    /// let mut decoder = Decoder::new(buf.as_reader_slice());
    /// assert!(decoder.decode_option()?.is_none());
    ///
    /// buf.clear();
    ///
    /// let mut en = Encoder::new(&mut buf);
    /// en.encode_bool(true)?;
    /// let mut decoder = Decoder::new(buf.as_reader_slice());
    /// let mut decoder = decoder.decode_option()?.unwrap();
    /// assert!(decoder.decode_bool()?);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn decode_option(&mut self) -> Result<Option<Decoder<R::Mut<'_>>>, Error> {
        // SAFETY: The slice must have been initialized by the reader.
        let [_, ty] = self.r.peek_array::<2>()?;
        let ty = Type::new(ty);

        match ty {
            Type::NONE => {
                _ = self.r.array::<2>()?;
                Ok(None)
            }
            _ => Ok(Some(Decoder::new(self.r.borrow_mut()))),
        }
    }

    /// Decode a boolean value.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Decoder, Encoder};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let mut en = Encoder::new(&mut buf);
    ///
    /// en.encode_bool(true)?;
    /// let mut decoder = Decoder::new(buf.as_reader_slice());
    /// assert!(decoder.decode_bool()?);
    ///
    /// buf.clear();
    ///
    /// let mut en = Encoder::new(&mut buf);
    /// en.encode_bool(false)?;
    /// let mut decoder = Decoder::new(buf.as_reader_slice());
    /// assert!(!decoder.decode_bool()?);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn decode_bool(&mut self) -> Result<bool, Error> {
        let (size, ty) = self.header()?;

        match ty {
            Type::BOOL if size == 4 => {
                let [value, _pad] = self.r.array()?;
                Ok(value != 0)
            }
            _ => Err(Error::new(ErrorKind::Expected {
                expected: Type::BOOL,
                actual: ty,
            })),
        }
    }

    /// Decode an id value.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Decoder, Encoder};
    /// use pod::id::MediaSubType;
    ///
    /// let mut buf = ArrayBuf::new();
    /// let mut en = Encoder::new(&mut buf);
    ///
    /// en.encode_id(MediaSubType::Opus)?;
    /// let mut decoder = Decoder::new(buf.as_reader_slice());
    /// let sub_type: MediaSubType = decoder.decode_id()?;
    /// assert_eq!(sub_type, MediaSubType::Opus);
    ///
    /// buf.clear();
    ///
    /// let mut en = Encoder::new(&mut buf);
    /// en.encode_id(MediaSubType::Opus)?;
    /// let mut decoder = Decoder::new(buf.as_reader_slice());
    /// let sub_type: MediaSubType = decoder.decode_id()?;
    /// assert_eq!(sub_type, MediaSubType::Opus);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn decode_id<I>(&mut self) -> Result<I, Error>
    where
        I: IntoId,
    {
        let (size, ty) = self.header()?;

        match ty {
            Type::ID if size == 4 => {
                let [value, _pad] = self.r.array()?;
                Ok(I::from_id(value))
            }
            _ => Err(Error::new(ErrorKind::Expected {
                expected: Type::ID,
                actual: ty,
            })),
        }
    }

    /// Decode a signed 32-bit integer.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Decoder, Encoder};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let mut en = Encoder::new(&mut buf);
    ///
    /// en.encode_int(-42)?;
    /// let mut decoder = Decoder::new(buf.as_reader_slice());
    /// assert_eq!(decoder.decode_int()?, -42);
    ///
    /// buf.clear();
    ///
    /// let mut en = Encoder::new(&mut buf);
    /// en.encode_int(-42)?;
    /// let mut decoder = Decoder::new(buf.as_reader_slice());
    /// assert_eq!(decoder.decode_int()?, -42);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn decode_int(&mut self) -> Result<i32, Error> {
        let (size, ty) = self.header()?;

        match ty {
            Type::INT if size == 4 => {
                let [value, _pad] = self.r.array()?;
                Ok(value.cast_signed())
            }
            _ => Err(Error::new(ErrorKind::Expected {
                expected: Type::INT,
                actual: ty,
            })),
        }
    }

    /// Decode a signed 64-bit long.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Decoder, Encoder};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let mut en = Encoder::new(&mut buf);
    ///
    /// en.encode_long(-42)?;
    /// let mut decoder = Decoder::new(buf.as_reader_slice());
    /// assert_eq!(decoder.decode_long()?, -42);
    ///
    /// buf.clear();
    ///
    /// let mut en = Encoder::new(&mut buf);
    /// en.encode_long(-42)?;
    /// let mut decoder = Decoder::new(buf.as_reader_slice());
    /// assert_eq!(decoder.decode_long()?, -42);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn decode_long(&mut self) -> Result<i64, Error> {
        let (size, ty) = self.header()?;

        match ty {
            Type::LONG if size == 8 => {
                let value = self.r.read_u64()?.cast_signed();
                Ok(value)
            }
            _ => Err(Error::new(ErrorKind::Expected {
                expected: Type::LONG,
                actual: ty,
            })),
        }
    }

    /// Decode a 32-bit float.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Decoder, Encoder};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let mut en = Encoder::new(&mut buf);
    ///
    /// en.encode_float(-42.42)?;
    /// let mut decoder = Decoder::new(buf.as_reader_slice());
    /// assert_eq!(decoder.decode_float()?, -42.42);
    ///
    /// buf.clear();
    ///
    /// let mut en = Encoder::new(&mut buf);
    /// en.encode_float(-42.42)?;
    /// let mut decoder = Decoder::new(buf.as_reader_slice());
    /// assert_eq!(decoder.decode_float()?, -42.42);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn decode_float(&mut self) -> Result<f32, Error> {
        let (size, ty) = self.header()?;

        match ty {
            Type::FLOAT if size == 4 => {
                let [value, _pad] = self.r.array()?;
                Ok(f32::from_bits(value))
            }
            _ => Err(Error::new(ErrorKind::Expected {
                expected: Type::FLOAT,
                actual: ty,
            })),
        }
    }

    /// Decode a 64-bit double.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Decoder, Encoder};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let mut en = Encoder::new(&mut buf);
    ///
    /// en.encode_double(-42.42)?;
    /// let mut decoder = Decoder::new(buf.as_reader_slice());
    /// assert_eq!(decoder.decode_double()?, -42.42);
    ///
    /// buf.clear();
    ///
    /// let mut en = Encoder::new(&mut buf);
    /// en.encode_double(-42.42)?;
    /// let mut decoder = Decoder::new(buf.as_reader_slice());
    /// assert_eq!(decoder.decode_double()?, -42.42);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn decode_double(&mut self) -> Result<f64, Error> {
        let (size, ty) = self.header()?;

        match ty {
            Type::DOUBLE if size == 8 => {
                let value = f64::from_bits(self.r.read_u64()?);
                Ok(value)
            }
            _ => Err(Error::new(ErrorKind::Expected {
                expected: Type::DOUBLE,
                actual: ty,
            })),
        }
    }

    /// Decode a c-string with full control through a visitor.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Decoder, Encoder};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let mut en = Encoder::new(&mut buf);
    ///
    /// en.encode_c_str(c"hello world")?;
    /// en.encode_c_str(c"this is right")?;
    ///
    /// let mut decoder = Decoder::new(buf.as_reader_slice());
    /// assert_eq!(decoder.decode_borrowed_c_str()?, c"hello world");
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn decode_c_str<V>(&mut self, visitor: V) -> Result<V::Ok, Error>
    where
        V: Visitor<'de, CStr>,
    {
        let (size, ty) = self.header()?;

        match ty {
            Type::STRING => CStr::read_content(self.r.borrow_mut(), size as usize, visitor),
            _ => Err(Error::new(ErrorKind::Expected {
                expected: Type::STRING,
                actual: ty,
            })),
        }
    }

    /// Decode a c-string.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Decoder, Encoder};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let mut en = Encoder::new(&mut buf);
    ///
    /// en.encode_c_str(c"hello world")?;
    /// en.encode_c_str(c"this is right")?;
    ///
    /// let mut decoder = Decoder::new(buf.as_reader_slice());
    /// assert_eq!(decoder.decode_borrowed_c_str()?, c"hello world");
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn decode_borrowed_c_str(&mut self) -> Result<&'de CStr, Error> {
        struct LocalVisitor;

        impl<'de> Visitor<'de, CStr> for LocalVisitor {
            type Ok = &'de CStr;

            #[inline]
            fn visit_borrowed(self, value: &'de CStr) -> Result<Self::Ok, Error> {
                Ok(value)
            }
        }

        self.decode_c_str(LocalVisitor)
    }

    /// Decode an owned c-string.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Decoder, Encoder};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let mut en = Encoder::new(&mut buf);
    ///
    /// en.encode_c_str(c"hello world")?;
    /// en.encode_c_str(c"this is right")?;
    ///
    /// let mut decoder = Decoder::new(buf.as_reader_slice());
    /// assert_eq!(decoder.decode_owned_c_str()?.as_c_str(), c"hello world");
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    #[cfg(feature = "alloc")]
    pub fn decode_owned_c_str(&mut self) -> Result<CString, Error> {
        struct LocalVisitor;

        impl<'de> Visitor<'de, CStr> for LocalVisitor {
            type Ok = CString;

            #[inline]
            fn visit_ref(self, value: &CStr) -> Result<Self::Ok, Error> {
                Ok(value.to_owned())
            }
        }

        self.decode_c_str(LocalVisitor)
    }

    /// Decode a string with full control through a visitor.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Decoder, Encoder};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let mut en = Encoder::new(&mut buf);
    ///
    /// en.encode_str("hello world")?;
    ///
    /// let mut decoder = Decoder::new(buf.as_reader_slice());
    /// assert_eq!(decoder.decode_borrowed_str()?, "hello world");
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn decode_str<V>(&mut self, visitor: V) -> Result<V::Ok, Error>
    where
        V: Visitor<'de, str>,
    {
        let (size, ty) = self.header()?;

        match ty {
            Type::STRING => str::read_content(self.r.borrow_mut(), size as usize, visitor),
            _ => Err(Error::new(ErrorKind::Expected {
                expected: Type::STRING,
                actual: ty,
            })),
        }
    }

    /// Decode a string.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Decoder, Encoder};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let mut en = Encoder::new(&mut buf);
    ///
    /// en.encode_str("hello world")?;
    /// en.encode_str("this is right")?;
    ///
    /// let mut decoder = Decoder::new(buf.as_reader_slice());
    /// assert_eq!(decoder.decode_borrowed_str()?, "hello world");
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn decode_borrowed_str(&mut self) -> Result<&'de str, Error> {
        struct LocalVisitor;

        impl<'de> Visitor<'de, str> for LocalVisitor {
            type Ok = &'de str;

            #[inline]
            fn visit_borrowed(self, value: &'de str) -> Result<Self::Ok, Error> {
                Ok(value)
            }
        }

        self.decode_str(LocalVisitor)
    }

    /// Decode an owned string.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Decoder, Encoder};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let mut en = Encoder::new(&mut buf);
    ///
    /// en.encode_str("hello world")?;
    /// en.encode_str("this is right")?;
    ///
    /// let mut decoder = Decoder::new(buf.as_reader_slice());
    /// assert_eq!(decoder.decode_owned_string()?.as_str(), "hello world");
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    #[cfg(feature = "alloc")]
    pub fn decode_owned_string(&mut self) -> Result<String, Error> {
        struct LocalVisitor;

        impl<'de> Visitor<'de, str> for LocalVisitor {
            type Ok = String;

            #[inline]
            fn visit_ref(self, value: &str) -> Result<Self::Ok, Error> {
                Ok(value.to_owned())
            }
        }

        self.decode_str(LocalVisitor)
    }

    /// Decode bytes with full control through a visitor.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Decoder, Encoder};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let mut en = Encoder::new(&mut buf);
    ///
    /// en.encode_bytes(b"hello world")?;
    /// en.encode_bytes(b"this is right")?;
    ///
    /// let mut decoder = Decoder::new(buf.as_reader_slice());
    /// assert_eq!(decoder.decode_borrowed_bytes()?, b"hello world");
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn decode_bytes<V>(&mut self, visitor: V) -> Result<V::Ok, Error>
    where
        V: Visitor<'de, [u8]>,
    {
        let (size, ty) = self.header()?;

        match ty {
            Type::BYTES => <[u8]>::read_content(self.r.borrow_mut(), size as usize, visitor),
            _ => Err(Error::new(ErrorKind::Expected {
                expected: Type::BYTES,
                actual: ty,
            })),
        }
    }

    /// Decode borrowed bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Decoder, Encoder};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let mut en = Encoder::new(&mut buf);
    ///
    /// en.encode_bytes(b"hello world")?;
    /// en.encode_bytes(b"this is right")?;
    ///
    /// let mut decoder = Decoder::new(buf.as_reader_slice());
    /// assert_eq!(decoder.decode_borrowed_bytes()?, b"hello world");
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn decode_borrowed_bytes(&mut self) -> Result<&'de [u8], Error> {
        struct LocalVisitor;

        impl<'de> Visitor<'de, [u8]> for LocalVisitor {
            type Ok = &'de [u8];

            #[inline]
            fn visit_borrowed(self, value: &'de [u8]) -> Result<Self::Ok, Error> {
                Ok(value)
            }
        }

        self.decode_bytes(LocalVisitor)
    }

    /// Decode an owned vector of bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Decoder, Encoder};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let mut en = Encoder::new(&mut buf);
    ///
    /// en.encode_bytes(b"hello world")?;
    /// en.encode_bytes(b"this is right")?;
    ///
    /// let mut decoder = Decoder::new(buf.as_reader_slice());
    /// assert_eq!(decoder.decode_owned_bytes()?.as_slice(), b"hello world");
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    #[cfg(feature = "alloc")]
    pub fn decode_owned_bytes(&mut self) -> Result<Vec<u8>, Error> {
        struct LocalVisitor;

        impl<'de> Visitor<'de, [u8]> for LocalVisitor {
            type Ok = Vec<u8>;

            #[inline]
            fn visit_ref(self, value: &[u8]) -> Result<Self::Ok, Error> {
                Ok(value.to_vec())
            }
        }

        self.decode_bytes(LocalVisitor)
    }

    /// Decode a [`Rectangle`].
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Decoder, Encoder, Rectangle};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let mut en = Encoder::new(&mut buf);
    ///
    /// en.encode_rectangle(Rectangle::new(100, 200))?;
    ///
    /// let mut decoder = Decoder::new(buf.as_reader_slice());
    /// assert_eq!(decoder.decode_rectangle()?, Rectangle::new(100, 200));
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn decode_rectangle(&mut self) -> Result<Rectangle, Error> {
        let (size, ty) = self.header()?;

        match ty {
            Type::RECTANGLE if size == 8 => {
                let mut out = Align::<[u32; 2], [_; 2]>::uninit();
                self.r.read_words_uninit(out.as_mut_slice())?;
                // SAFETY: The slice must have been initialized by the reader.
                let [width, height] = unsafe { out.assume_init().read() };
                Ok(Rectangle::new(width, height))
            }
            _ => Err(Error::new(ErrorKind::Expected {
                expected: Type::RECTANGLE,
                actual: ty,
            })),
        }
    }

    /// Decode a [`Fraction`].
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Decoder, Encoder, Fraction};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let mut en = Encoder::new(&mut buf);
    ///
    /// en.encode_fraction(Fraction::new(100, 200))?;
    ///
    /// let mut decoder = Decoder::new(buf.as_reader_slice());
    /// assert_eq!(decoder.decode_fraction()?, Fraction::new(100, 200));
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn decode_fraction(&mut self) -> Result<Fraction, Error> {
        let (size, ty) = self.header()?;

        match ty {
            Type::FRACTION if size == 8 => {
                let mut out = Align::<[u32; 2], [_; 2]>::uninit();
                self.r.read_words_uninit(out.as_mut_slice())?;
                // SAFETY: The slice must have been initialized by the reader.
                let [num, denom] = unsafe { out.assume_init().read() };
                Ok(Fraction::new(num, denom))
            }
            _ => Err(Error::new(ErrorKind::Expected {
                expected: Type::FRACTION,
                actual: ty,
            })),
        }
    }

    /// Decode bitmap with full control through a visitor.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Bitmap, Decoder, Encoder};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let mut en = Encoder::new(&mut buf);
    ///
    /// en.encode_bitmap(Bitmap::new(b"hello world"))?;
    ///
    /// let mut decoder = Decoder::new(buf.as_reader_slice());
    /// assert_eq!(decoder.decode_borrowed_bitmap()?, b"hello world");
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn decode_bitmap<V>(&mut self, visitor: V) -> Result<V::Ok, Error>
    where
        V: Visitor<'de, Bitmap>,
    {
        let (size, ty) = self.header()?;

        match ty {
            Type::BITMAP => Bitmap::read_content(self.r.borrow_mut(), size as usize, visitor),
            _ => Err(Error::new(ErrorKind::Expected {
                expected: Type::BITMAP,
                actual: ty,
            })),
        }
    }

    /// Decode borrowed bitmap.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Bitmap, Decoder, Encoder};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let mut en = Encoder::new(&mut buf);
    ///
    /// en.encode_bitmap(Bitmap::new(b"hello world"))?;
    ///
    /// let mut decoder = Decoder::new(buf.as_reader_slice());
    /// assert_eq!(decoder.decode_borrowed_bitmap()?, b"hello world");
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn decode_borrowed_bitmap(&mut self) -> Result<&'de Bitmap, Error> {
        struct LocalVisitor;

        impl<'de> Visitor<'de, Bitmap> for LocalVisitor {
            type Ok = &'de Bitmap;

            #[inline]
            fn visit_borrowed(self, value: &'de Bitmap) -> Result<Self::Ok, Error> {
                Ok(value)
            }
        }

        self.decode_bitmap(LocalVisitor)
    }

    /// Decode an owned vector of bitmap.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Bitmap, Decoder, Encoder};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let mut en = Encoder::new(&mut buf);
    ///
    /// en.encode_bitmap(Bitmap::new(b"hello world"))?;
    ///
    /// let mut decoder = Decoder::new(buf.as_reader_slice());
    /// assert_eq!(decoder.decode_owned_bitmap()?.as_bytes(), b"hello world");
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    #[cfg(feature = "alloc")]
    pub fn decode_owned_bitmap(&mut self) -> Result<OwnedBitmap, Error> {
        struct LocalVisitor;

        impl<'de> Visitor<'de, Bitmap> for LocalVisitor {
            type Ok = OwnedBitmap;

            #[inline]
            fn visit_ref(self, value: &Bitmap) -> Result<Self::Ok, Error> {
                Ok(value.to_owned())
            }
        }

        self.decode_bitmap(LocalVisitor)
    }

    /// Decode an array.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Decoder, Encoder, Type};
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
    ///
    /// let mut decoder = Decoder::new(buf.as_reader_slice());
    /// let mut array = decoder.decode_array()?;
    ///
    /// assert!(!array.is_empty());
    /// assert_eq!(array.len(), 3);
    ///
    /// assert_eq!(array.decode::<i32>()?, 1i32);
    /// assert_eq!(array.decode::<i32>()?, 2i32);
    /// assert_eq!(array.decode::<i32>()?, 3i32);
    ///
    /// assert!(array.is_empty());
    /// assert_eq!(array.len(), 0);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn decode_array(&mut self) -> Result<DecodeArray<R::Mut<'_>>, Error> {
        let (full_size, ty) = self.header()?;

        match ty {
            Type::ARRAY if full_size >= 8 => {
                let size = full_size - 8;

                let [child_size, child_type] = self.r.array()?;
                let child_type = Type::new(child_type);

                let remaining;

                if size > 0 && child_size > 0 {
                    if size % child_size != 0 {
                        return Err(Error::new(ErrorKind::InvalidArraySize {
                            size: full_size,
                            child_size,
                        }));
                    }

                    remaining = (size / child_size) as usize;
                } else {
                    remaining = 0;
                }

                Ok(DecodeArray::new(
                    self.r.borrow_mut(),
                    child_type,
                    child_size as usize,
                    remaining,
                ))
            }
            _ => Err(Error::new(ErrorKind::Expected {
                expected: Type::ARRAY,
                actual: ty,
            })),
        }
    }

    #[inline]
    fn header(&mut self) -> Result<(u32, Type), Error> {
        let [size, ty] = self.r.array()?;
        let ty = Type::new(ty);
        Ok((size, ty))
    }
}
