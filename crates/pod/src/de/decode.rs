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
use crate::{Error, Fd, Fraction, Id, IntoId, Pointer, Reader, Rectangle, Type, utils::WordBytes};

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
    use crate::{DecodeUnsized, Fd, Fraction, Id, Pointer, Rectangle};

    pub trait Sealed {}
    impl Sealed for bool {}
    impl<I> Sealed for Id<I> where I: IntoId {}
    impl Sealed for i32 {}
    impl Sealed for u32 {}
    impl Sealed for i64 {}
    impl Sealed for u64 {}
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
    impl Sealed for Pointer {}
    impl Sealed for Fd {}
    impl<'de, E> Sealed for &E where E: ?Sized + DecodeUnsized<'de> {}
}

/// A trait for types that can be decoded.
pub trait Decode<'de>: Sized + self::sealed::Sealed {
    /// The type of the decoded value.
    #[doc(hidden)]
    const TYPE: Type;

    /// Read the content of a type.
    #[doc(hidden)]
    fn read_content(reader: impl Reader<'de, u64>, size: u32) -> Result<Self, Error>;
}

/// [`Decode`] implementation for `i32`.
///
/// # Examples
///
/// ```
/// use pod::Pod;
///
/// let mut pod = Pod::array();
/// pod.as_mut().encode(10i32)?;
/// assert_eq!(pod.decode::<i32>()?, 10i32);
/// # Ok::<_, pod::Error>(())
/// ```
impl<'de> Decode<'de> for bool {
    const TYPE: Type = Type::BOOL;

    #[inline]
    fn read_content(mut reader: impl Reader<'de, u64>, _: u32) -> Result<Self, Error> {
        let [value, _pad] = reader.read::<[u32; 2]>()?;
        Ok(value != 0)
    }
}

/// [`Decode`] implementation for an [`IntoId`] type.
///
/// # Examples
///
/// ```
/// use pod::{Pod, Id};
///
/// let mut pod = Pod::array();
/// pod.as_mut().encode(Id(142u32))?;
/// assert_eq!(pod.decode::<Id<u32>>()?, Id(142u32));
/// # Ok::<_, pod::Error>(())
/// ```
impl<'de, I> Decode<'de> for Id<I>
where
    I: IntoId,
{
    const TYPE: Type = Type::ID;

    #[inline]
    fn read_content(mut reader: impl Reader<'de, u64>, _: u32) -> Result<Self, Error> {
        let [value, _pad] = reader.read()?;
        Ok(Id(I::from_id(value)))
    }
}

/// [`Decode`] implementation for `i32`.
///
/// # Examples
///
/// ```
/// use pod::Pod;
///
/// let mut pod = Pod::array();
/// pod.as_mut().encode(10i32)?;
/// assert_eq!(pod.as_ref().decode::<i32>()?, 10i32);
/// # Ok::<_, pod::Error>(())
/// ```
impl<'de> Decode<'de> for i32 {
    const TYPE: Type = Type::INT;

    #[inline]
    fn read_content(mut reader: impl Reader<'de, u64>, _: u32) -> Result<Self, Error> {
        let [value, _pad] = reader.read::<[u32; 2]>()?;
        Ok(value.cast_signed())
    }
}

/// [`Decode`] implementation for `u32`.
///
/// # Examples
///
/// ```
/// use pod::Pod;
///
/// let mut pod = Pod::array();
/// pod.as_mut().encode(10u32)?;
/// assert_eq!(pod.as_ref().decode::<u32>()?, 10u32);
///
/// let mut pod = Pod::array();
/// pod.as_mut().encode(10i32)?;
/// assert_eq!(pod.as_ref().decode::<u32>()?, 10u32);
/// # Ok::<_, pod::Error>(())
/// ```
impl<'de> Decode<'de> for u32 {
    const TYPE: Type = Type::INT;

    #[inline]
    fn read_content(reader: impl Reader<'de, u64>, size: u32) -> Result<Self, Error> {
        Ok(i32::read_content(reader, size)?.cast_unsigned())
    }
}

/// [`Decode`] implementation for `i64`.
///
/// # Examples
///
/// ```
/// use pod::Pod;
///
/// let mut pod = Pod::array();
/// pod.as_mut().encode(10i64)?;
/// assert_eq!(pod.decode::<i64>()?, 10i64);
/// # Ok::<_, pod::Error>(())
/// ```
impl<'de> Decode<'de> for i64 {
    const TYPE: Type = Type::LONG;

    #[inline]
    fn read_content(mut reader: impl Reader<'de, u64>, _: u32) -> Result<Self, Error> {
        Ok(reader.read::<u64>()?.cast_signed())
    }
}

/// [`Decode`] implementation for `u64`.
///
/// # Examples
///
/// ```
/// use pod::Pod;
///
/// let mut pod = Pod::array();
/// pod.as_mut().encode(10u64)?;
/// assert_eq!(pod.decode::<i64>()?, 10);
///
/// let mut pod = Pod::array();
/// pod.as_mut().encode(10i64)?;
/// assert_eq!(pod.decode::<i64>()?, 10);
/// # Ok::<_, pod::Error>(())
/// ```
impl<'de> Decode<'de> for u64 {
    const TYPE: Type = Type::LONG;

    #[inline]
    fn read_content(reader: impl Reader<'de, u64>, size: u32) -> Result<Self, Error> {
        Ok(i64::read_content(reader, size)?.cast_unsigned())
    }
}

/// [`Decode`] implementation for `f32`.
///
/// # Examples
///
/// ```
/// use pod::Pod;
///
/// let mut pod = Pod::array();
/// pod.as_mut().encode(42.42f32)?;
/// assert_eq!(pod.decode::<f32>()?, 42.42f32);
/// # Ok::<_, pod::Error>(())
/// ```
impl<'de> Decode<'de> for f32 {
    const TYPE: Type = Type::FLOAT;

    #[inline]
    fn read_content(mut reader: impl Reader<'de, u64>, _: u32) -> Result<Self, Error> {
        let [value, _pad] = reader.read()?;
        Ok(f32::from_bits(value))
    }
}

/// [`Decode`] implementation for `f64`.
///
/// # Examples
///
/// ```
/// use pod::Pod;
///
/// let mut pod = Pod::array();
/// pod.as_mut().encode(42.42f64)?;
/// assert_eq!(pod.decode::<f64>()?, 42.42f64);
/// # Ok::<_, pod::Error>(())
/// ```
impl<'de> Decode<'de> for f64 {
    const TYPE: Type = Type::DOUBLE;

    #[inline]
    fn read_content(mut reader: impl Reader<'de, u64>, _: u32) -> Result<Self, Error> {
        Ok(f64::from_bits(reader.read::<u64>()?))
    }
}

/// [`Decode`] implementation for [`Rectangle`].
///
/// # Examples
///
/// ```
/// use pod::{Pod, Rectangle};
///
/// let mut pod = Pod::array();
/// pod.as_mut().encode(Rectangle::new(100, 200))?;
/// assert_eq!(pod.decode::<Rectangle>()?, Rectangle::new(100, 200));
/// # Ok::<_, pod::Error>(())
/// ```
impl<'de> Decode<'de> for Rectangle {
    const TYPE: Type = Type::RECTANGLE;

    #[inline]
    fn read_content(mut reader: impl Reader<'de, u64>, _: u32) -> Result<Self, Error> {
        let [width, height] = reader.read()?;
        Ok(Rectangle::new(width, height))
    }
}

/// [`Decode`] implementation for a [`Fraction`].
///
/// # Examples
///
/// ```
/// use pod::{Pod, Fraction};
///
/// let mut pod = Pod::array();
/// pod.as_mut().encode(Fraction::new(800, 600))?;
/// assert_eq!(pod.decode::<Fraction>()?, Fraction::new(800, 600));
/// # Ok::<_, pod::Error>(())
/// ```
impl<'de> Decode<'de> for Fraction {
    const TYPE: Type = Type::FRACTION;

    #[inline]
    fn read_content(mut reader: impl Reader<'de, u64>, _: u32) -> Result<Self, Error> {
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
/// use pod::Pod;
///
/// let mut pod = Pod::array();
/// pod.as_mut().encode_unsized(c"hello world")?;
/// assert_eq!(pod.as_ref().decode::<CString>()?.as_c_str(), c"hello world");
/// # Ok::<_, pod::Error>(())
/// ```
#[cfg(feature = "alloc")]
impl<'de> Decode<'de> for CString {
    const TYPE: Type = Type::STRING;

    #[inline]
    fn read_content(reader: impl Reader<'de, u64>, size: u32) -> Result<Self, Error> {
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
/// use pod::Pod;
///
/// let mut pod = Pod::array();
///
/// pod.as_mut().encode_unsized("hello world")?;
/// pod.as_mut().encode_unsized("this is right")?;
///
/// assert_eq!(pod.as_mut().decode::<String>()?, "hello world");
/// assert_eq!(pod.as_mut().decode::<String>()?, "this is right");
/// # Ok::<_, pod::Error>(())
/// ```
#[cfg(feature = "alloc")]
impl<'de> Decode<'de> for String {
    const TYPE: Type = Type::STRING;

    #[inline]
    fn read_content(reader: impl Reader<'de, u64>, size: u32) -> Result<Self, Error> {
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
/// use pod::Pod;
///
/// let mut pod = Pod::array();
///
/// pod.as_mut().encode(*b"hello world")?;
/// pod.as_mut().encode(*b"this is right")?;
///
/// assert_eq!(pod.as_mut().decode::<Vec<u8>>()?, b"hello world");
/// assert_eq!(pod.as_mut().decode::<Vec<u8>>()?, b"this is right");
/// # Ok::<_, pod::Error>(())
/// ```
#[cfg(feature = "alloc")]
impl<'de> Decode<'de> for Vec<u8> {
    const TYPE: Type = Type::BYTES;

    #[inline]
    fn read_content(reader: impl Reader<'de, u64>, size: u32) -> Result<Self, Error> {
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
/// use pod::{Bitmap, Pod, OwnedBitmap};
///
/// let mut pod = Pod::array();
/// pod.as_mut().encode_unsized(Bitmap::new(b"hello world"))?;
/// assert_eq!(pod.decode::<OwnedBitmap>()?.as_bytes(), b"hello world");
/// # Ok::<_, pod::Error>(())
/// ```
#[cfg(feature = "alloc")]
impl<'de> Decode<'de> for OwnedBitmap {
    const TYPE: Type = Type::BITMAP;

    #[inline]
    fn read_content(reader: impl Reader<'de, u64>, size: u32) -> Result<Self, Error> {
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

/// [`Decode`] implementation for [`Pointer`].
///
/// # Examples
///
/// ```
/// use pod::{Pod, Pointer};
///
/// let value = 1u32;
///
/// let mut pod = Pod::array();
/// pod.as_mut().encode(Pointer::new((&value as *const u32).addr()))?;
/// assert_eq!(pod.decode::<Pointer>()?, Pointer::new((&value as *const u32).addr()));
/// # Ok::<_, pod::Error>(())
/// ```
impl<'de> Decode<'de> for Pointer {
    const TYPE: Type = Type::POINTER;

    #[inline]
    fn read_content(mut reader: impl Reader<'de, u64>, _: u32) -> Result<Self, Error> {
        let [ty, _pad, p1, p2] = reader.read::<[u32; 4]>()?;

        let mut bytes = WordBytes::new();
        bytes.write_half_words([p1, p2]);
        Ok(Pointer::new_with_type(bytes.read_usize(), ty))
    }
}

/// [`Decode`] implementation for [`Fd`].
///
/// # Examples
///
/// ```
/// use pod::{Pod, Fd};
///
/// let mut pod = Pod::array();
/// pod.as_mut().encode(Fd::new(4))?;
/// assert_eq!(pod.decode::<Fd>()?, Fd::new(4));
/// # Ok::<_, pod::Error>(())
/// ```
impl<'de> Decode<'de> for Fd {
    const TYPE: Type = Type::FD;

    #[inline]
    fn read_content(mut reader: impl Reader<'de, u64>, _: u32) -> Result<Self, Error> {
        Ok(Self::new(reader.read::<u64>()?.cast_signed()))
    }
}
