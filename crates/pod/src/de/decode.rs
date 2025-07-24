#[cfg(feature = "alloc")]
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
use crate::{Bitmap, DecodeUnsized, OwnedBitmap, Visitor};
use crate::{Error, Fraction, Id, IntoId, Reader, Rectangle, Type};

pub(crate) mod sealed {
    #[cfg(feature = "alloc")]
    use alloc::ffi::CString;
    #[cfg(feature = "alloc")]
    use alloc::string::String;
    #[cfg(feature = "alloc")]
    use alloc::vec::Vec;

    #[cfg(feature = "alloc")]
    use crate::OwnedBitmap;
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
    #[cfg(feature = "alloc")]
    impl Sealed for CString {}
    #[cfg(feature = "alloc")]
    impl Sealed for String {}
    #[cfg(feature = "alloc")]
    impl Sealed for Vec<u8> {}
    #[cfg(feature = "alloc")]
    impl Sealed for OwnedBitmap {}
    impl<'de, E> Sealed for &E where E: ?Sized + DecodeUnsized<'de> {}
}

/// A trait for types that can be decoded.
pub trait Decode<'de>: Sized + self::sealed::Sealed {
    /// The type of the decoded value.
    #[doc(hidden)]
    const TYPE: Type;

    /// Read the content of a type.
    #[doc(hidden)]
    fn read_content(reader: impl Reader<'de>, size: u32) -> Result<Self, Error>;
}

/// [`Decode`] implementation for `i32`.
///
/// # Examples
///
/// ```
/// use pod::{ArrayBuf, Pod};
///
/// let mut buf = ArrayBuf::new();
/// let pod = Pod::new(&mut buf);
/// pod.encode(10i32)?;
///
/// let pod = Pod::new(buf.as_slice());
/// let value: i32 = pod.decode()?;
/// assert_eq!(value, 10i32);
/// # Ok::<_, pod::Error>(())
/// ```
impl<'de> Decode<'de> for bool {
    const TYPE: Type = Type::BOOL;

    #[inline]
    fn read_content(mut reader: impl Reader<'de>, _: u32) -> Result<Self, Error> {
        let [value, _pad] = reader.read::<[u32; 2]>()?;
        Ok(value != 0)
    }
}

/// [`Decode`] implementation for an [`IntoId`] type.
///
/// # Examples
///
/// ```
/// use pod::{ArrayBuf, Pod, Id};
///
/// let mut buf = ArrayBuf::new();
/// let pod = Pod::new(&mut buf);
/// pod.encode(Id(142u32))?;
///
/// let mut pod = Pod::new(buf.as_slice());
/// assert_eq!(pod.decode::<Id<u32>>()?, Id(142u32));
/// # Ok::<_, pod::Error>(())
/// ```
impl<'de, I> Decode<'de> for Id<I>
where
    I: IntoId,
{
    const TYPE: Type = Type::ID;

    #[inline]
    fn read_content(mut reader: impl Reader<'de>, _: u32) -> Result<Self, Error> {
        let [value, _pad] = reader.read()?;
        Ok(Id(I::from_id(value)))
    }
}

/// [`Decode`] implementation for `i32`.
///
/// # Examples
///
/// ```
/// use pod::{ArrayBuf, Pod};
///
/// let mut buf = ArrayBuf::new();
/// let pod = Pod::new(&mut buf);
/// pod.encode(10i32)?;
///
/// let pod = Pod::new(buf.as_slice());
/// assert_eq!(pod.decode::<i32>()?, 10i32);
/// # Ok::<_, pod::Error>(())
/// ```
impl<'de> Decode<'de> for i32 {
    const TYPE: Type = Type::INT;

    #[inline]
    fn read_content(mut reader: impl Reader<'de>, _: u32) -> Result<Self, Error> {
        let [value, _pad] = reader.read::<[u32; 2]>()?;
        Ok(value.cast_signed())
    }
}

/// [`Decode`] implementation for `i64`.
///
/// # Examples
///
/// ```
/// use pod::{ArrayBuf, Pod};
///
/// let mut buf = ArrayBuf::new();
/// let pod = Pod::new(&mut buf);
/// pod.encode(10i64)?;
///
/// let pod = Pod::new(buf.as_slice());
/// assert_eq!(pod.decode::<i64>()?, 10i64);
/// # Ok::<_, pod::Error>(())
/// ```
impl<'de> Decode<'de> for i64 {
    const TYPE: Type = Type::LONG;

    #[inline]
    fn read_content(mut reader: impl Reader<'de>, _: u32) -> Result<Self, Error> {
        Ok(reader.read::<u64>()?.cast_signed())
    }
}

/// [`Decode`] implementation for `f32`.
///
/// # Examples
///
/// ```
/// use pod::{ArrayBuf, Pod};
///
/// let mut buf = ArrayBuf::new();
/// let pod = Pod::new(&mut buf);
/// pod.encode(42.42f32)?;
///
/// let pod = Pod::new(buf.as_slice());
/// assert_eq!(pod.decode::<f32>()?, 42.42f32);
/// # Ok::<_, pod::Error>(())
/// ```
impl<'de> Decode<'de> for f32 {
    const TYPE: Type = Type::FLOAT;

    #[inline]
    fn read_content(mut reader: impl Reader<'de>, _: u32) -> Result<Self, Error> {
        let [value, _pad] = reader.read()?;
        Ok(f32::from_bits(value))
    }
}

/// [`Decode`] implementation for `f64`.
///
/// # Examples
///
/// ```
/// use pod::{ArrayBuf, Pod};
///
/// let mut buf = ArrayBuf::new();
/// let pod = Pod::new(&mut buf);
/// pod.encode(42.42f64)?;
///
/// let pod = Pod::new(buf.as_slice());
/// assert_eq!(pod.decode::<f64>()?, 42.42f64);
/// # Ok::<_, pod::Error>(())
/// ```
impl<'de> Decode<'de> for f64 {
    const TYPE: Type = Type::DOUBLE;

    #[inline]
    fn read_content(mut reader: impl Reader<'de>, _: u32) -> Result<Self, Error> {
        Ok(f64::from_bits(reader.read::<u64>()?))
    }
}

/// [`Decode`] implementation for `Rectangle`.
///
/// # Examples
///
/// ```
/// use pod::{ArrayBuf, Pod, Rectangle};
///
/// let mut buf = ArrayBuf::new();
/// let pod = Pod::new(&mut buf);
/// pod.encode(Rectangle::new(100, 200))?;
///
/// let pod = Pod::new(buf.as_slice());
/// assert_eq!(pod.decode::<Rectangle>()?, Rectangle::new(100, 200));
/// # Ok::<_, pod::Error>(())
/// ```
impl<'de> Decode<'de> for Rectangle {
    const TYPE: Type = Type::RECTANGLE;

    #[inline]
    fn read_content(mut reader: impl Reader<'de>, _: u32) -> Result<Self, Error> {
        let [width, height] = reader.read()?;
        Ok(Rectangle::new(width, height))
    }
}

/// [`Decode`] a [`Fraction`].
///
/// # Examples
///
/// ```
/// use pod::{ArrayBuf, Pod, Fraction};
///
/// let mut buf = ArrayBuf::new();
/// let pod = Pod::new(&mut buf);
/// pod.encode(Fraction::new(800, 600))?;
///
/// let pod = Pod::new(buf.as_slice());
/// assert_eq!(pod.decode::<Fraction>()?, Fraction::new(800, 600));
/// # Ok::<_, pod::Error>(())
/// ```
impl<'de> Decode<'de> for Fraction {
    const TYPE: Type = Type::FRACTION;

    #[inline]
    fn read_content(mut reader: impl Reader<'de>, _: u32) -> Result<Self, Error> {
        let [num, denom] = reader.read()?;
        Ok(Fraction::new(num, denom))
    }
}

/// Decode an owned c-string.
///
/// # Examples
///
/// ```
/// use std::ffi::CString;
/// use pod::{ArrayBuf, Pod};
///
/// let mut buf = ArrayBuf::new();
/// let pod = Pod::new(&mut buf);
/// pod.encode_unsized(c"hello world")?;
///
/// let pod = Pod::new(buf.as_slice());
/// assert_eq!(pod.decode::<CString>()?.as_c_str(), c"hello world");
/// # Ok::<_, pod::Error>(())
/// ```
#[cfg(feature = "alloc")]
impl<'de> Decode<'de> for CString {
    const TYPE: Type = Type::STRING;

    #[inline]
    fn read_content(reader: impl Reader<'de>, size: u32) -> Result<Self, Error> {
        CStr::read_content(reader, CStrVisitor, size)
    }
}

#[cfg(feature = "alloc")]
struct CStrVisitor;

#[cfg(feature = "alloc")]
impl<'de> Visitor<'de, CStr> for CStrVisitor {
    type Ok = CString;

    #[inline]
    fn visit_ref(self, value: &CStr) -> Result<Self::Ok, Error> {
        Ok(value.to_owned())
    }
}

/// Decode an owned [`String`].
///
/// # Examples
///
/// ```
/// use pod::{ArrayBuf, Pod};
///
/// let mut buf = ArrayBuf::new();
///
/// Pod::new(&mut buf).encode_unsized("hello world")?;
/// Pod::new(&mut buf).encode_unsized("this is right")?;
///
/// let mut slice = buf.as_slice();
///
/// assert_eq!(Pod::new(&mut slice).decode::<String>()?, "hello world");
/// assert_eq!(Pod::new(&mut slice).decode::<String>()?, "this is right");
/// # Ok::<_, pod::Error>(())
/// ```
#[cfg(feature = "alloc")]
impl<'de> Decode<'de> for String {
    const TYPE: Type = Type::STRING;

    #[inline]
    fn read_content(reader: impl Reader<'de>, size: u32) -> Result<Self, Error> {
        str::read_content(reader, StrVisitor, size)
    }
}

#[cfg(feature = "alloc")]
struct StrVisitor;

#[cfg(feature = "alloc")]
impl<'de> Visitor<'de, str> for StrVisitor {
    type Ok = String;

    #[inline]
    fn visit_ref(self, value: &str) -> Result<Self::Ok, Error> {
        Ok(value.to_owned())
    }
}

/// Decode an owned vector of bytes [`Vec<u8>`].
///
/// # Examples
///
/// ```
/// use pod::{ArrayBuf, Pod};
///
/// let mut buf = ArrayBuf::new();
/// Pod::new(&mut buf).encode(*b"hello world")?;
/// Pod::new(&mut buf).encode(*b"this is right")?;
///
/// let mut slice = buf.as_slice();
///
/// assert_eq!(Pod::new(&mut slice).decode::<Vec<u8>>()?, b"hello world");
/// assert_eq!(Pod::new(&mut slice).decode::<Vec<u8>>()?, b"this is right");
/// # Ok::<_, pod::Error>(())
/// ```
#[cfg(feature = "alloc")]
impl<'de> Decode<'de> for Vec<u8> {
    const TYPE: Type = Type::BYTES;

    #[inline]
    fn read_content(reader: impl Reader<'de>, size: u32) -> Result<Self, Error> {
        <[u8]>::read_content(reader, BytesVisitor, size)
    }
}

#[cfg(feature = "alloc")]
struct BytesVisitor;

#[cfg(feature = "alloc")]
impl<'de> Visitor<'de, [u8]> for BytesVisitor {
    type Ok = Vec<u8>;

    #[inline]
    fn visit_ref(self, value: &[u8]) -> Result<Self::Ok, Error> {
        Ok(value.to_owned())
    }
}

/// Decode an owned [`OwnedBitmap`].
///
/// # Examples
///
/// ```
/// use pod::{ArrayBuf, Bitmap, Pod, OwnedBitmap};
///
/// let mut buf = ArrayBuf::new();
/// let pod = Pod::new(&mut buf);
/// pod.encode_unsized(Bitmap::new(b"hello world"))?;
///
/// let pod = Pod::new(buf.as_slice());
/// assert_eq!(pod.decode::<OwnedBitmap>()?.as_bytes(), b"hello world");
/// # Ok::<_, pod::Error>(())
/// ```
#[cfg(feature = "alloc")]
impl<'de> Decode<'de> for OwnedBitmap {
    const TYPE: Type = Type::BITMAP;

    #[inline]
    fn read_content(reader: impl Reader<'de>, size: u32) -> Result<Self, Error> {
        Bitmap::read_content(reader, BitmapVisitor, size)
    }
}

#[cfg(feature = "alloc")]
struct BitmapVisitor;

#[cfg(feature = "alloc")]
impl<'de> Visitor<'de, Bitmap> for BitmapVisitor {
    type Ok = OwnedBitmap;

    #[inline]
    fn visit_ref(self, value: &Bitmap) -> Result<Self::Ok, Error> {
        Ok(value.to_owned())
    }
}
