use crate::error::ErrorKind;
use crate::{DWORD_SIZE, Error, Id, IntoId, Reader, Type, Visitor};

use super::{Decode, DecodeArray, DecodeUnsized};

/// A POD (Plain Old Data) decoder.
pub struct Decoder<R> {
    r: R,
}

impl<'de, R> Decoder<R>
where
    R: Reader<'de>,
{
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
    /// en.encode(true)?;
    /// let mut de = Decoder::new(buf.as_reader_slice());
    ///
    /// assert!(de.decode::<bool>()?);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn new(r: R) -> Self {
        Self { r }
    }

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
    #[inline]
    pub fn decode<T>(&mut self) -> Result<T, Error>
    where
        T: Decode<'de>,
    {
        T::decode(self.r.borrow_mut())
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
    /// let bytes: &[u8] = de.decode_borrowed()?;
    /// assert_eq!(bytes, b"hello world");
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn decode_unsized<T, V>(&mut self, visitor: V) -> Result<V::Ok, Error>
    where
        T: ?Sized + DecodeUnsized<'de>,
        V: Visitor<'de, T>,
    {
        T::decode_unsized(self.r.borrow_mut(), visitor)
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
    /// let bytes: &[u8] = de.decode_borrowed()?;
    /// assert_eq!(bytes, b"hello world");
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn decode_borrowed<T>(&mut self) -> Result<&'de T, Error>
    where
        T: ?Sized + DecodeUnsized<'de>,
    {
        T::decode_borrowed(self.r.borrow_mut())
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
    /// en.encode(true)?;
    /// let mut decoder = Decoder::new(buf.as_reader_slice());
    ///
    /// let Some(mut decoder) = decoder.decode_option()? else {
    ///     panic!("expected some value");
    /// };
    ///
    /// assert!(decoder.decode::<bool>()?);
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
        let Id(id) = Id::<I>::decode(self.r.borrow_mut())?;
        Ok(id)
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
        let (full_size, ty) = self.r.header()?;

        match ty {
            Type::ARRAY if full_size >= 8 => {
                let size = full_size - 8;

                let [child_size, child_type] = self.r.array()?;
                let child_type = Type::new(child_type);

                let remaining = if size > 0 && child_size > 0 {
                    if size % child_size != 0 {
                        return Err(Error::new(ErrorKind::InvalidArraySize {
                            size: full_size,
                            child_size,
                        }));
                    }

                    let padded_child_size = child_size.next_multiple_of(DWORD_SIZE as u32);
                    (size / padded_child_size) as usize
                } else {
                    0
                };

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
}
