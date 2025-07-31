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

use crate::buf::ArrayVec;
use crate::error::ErrorKind;
use crate::utils::WordBytes;
#[cfg(feature = "alloc")]
use crate::{Bitmap, OwnedBitmap, UnsizedReadable};
use crate::{Error, Fd, Fraction, Id, Pointer, RawId, Reader, Rectangle, Type};

/// A trait for types that can be decoded.
pub trait SizedReadable<'de>
where
    Self: Sized,
{
    /// The type of the decoded value.
    #[doc(hidden)]
    const TYPE: Type;

    /// Read the content of a type.
    #[doc(hidden)]
    fn read_content(reader: impl Reader<'de>, size: usize) -> Result<Self, Error>;
}

/// [`Decode`] implementation for `i32`.
///
/// # Examples
///
/// ```
/// let mut pod = pod::array();
/// pod.as_mut().write(10i32)?;
/// assert_eq!(pod.as_ref().read_sized::<i32>()?, 10i32);
/// # Ok::<_, pod::Error>(())
/// ```
impl<'de> SizedReadable<'de> for bool {
    const TYPE: Type = Type::BOOL;

    #[inline]
    fn read_content(mut reader: impl Reader<'de>, _: usize) -> Result<Self, Error> {
        Ok(reader.read::<u32>()? != 0)
    }
}

crate::macros::decode_from_sized!(bool);

/// [`Decode`] implementation for an [`RawId`] type.
///
/// # Examples
///
/// ```
/// use pod::{Pod, Id};
///
/// let mut pod = pod::array();
/// pod.as_mut().write(Id(142u32))?;
/// assert_eq!(pod.as_ref().read_sized::<Id<u32>>()?, Id(142u32));
/// # Ok::<_, pod::Error>(())
/// ```
impl<'de, I> SizedReadable<'de> for Id<I>
where
    I: RawId,
{
    const TYPE: Type = Type::ID;

    #[inline]
    fn read_content(mut reader: impl Reader<'de>, _: usize) -> Result<Self, Error> {
        Ok(Id(I::from_id(reader.read()?)))
    }
}

crate::macros::decode_from_sized!(impl [I] Id<I> where I: RawId);

/// [`Decode`] implementation for `i32`.
///
/// # Examples
///
/// ```
/// let mut pod = pod::array();
/// pod.as_mut().write(10i32)?;
/// assert_eq!(pod.as_ref().read_sized::<i32>()?, 10i32);
/// # Ok::<_, pod::Error>(())
/// ```
impl<'de> SizedReadable<'de> for i32 {
    const TYPE: Type = Type::INT;

    #[inline]
    fn read_content(mut reader: impl Reader<'de>, _: usize) -> Result<Self, Error> {
        Ok(reader.read::<u32>()?.cast_signed())
    }
}

crate::macros::decode_from_sized!(i32);

/// [`Decode`] implementation for `u32`.
///
/// # Examples
///
/// ```
/// let mut pod = pod::array();
/// pod.as_mut().write(10u32)?;
/// assert_eq!(pod.as_ref().read_sized::<u32>()?, 10u32);
///
/// let mut pod = pod::array();
/// pod.as_mut().write(10i32)?;
/// assert_eq!(pod.as_ref().read_sized::<u32>()?, 10u32);
/// # Ok::<_, pod::Error>(())
/// ```
impl<'de> SizedReadable<'de> for u32 {
    const TYPE: Type = Type::INT;

    #[inline]
    fn read_content(reader: impl Reader<'de>, size: usize) -> Result<Self, Error> {
        Ok(i32::read_content(reader, size)?.cast_unsigned())
    }
}

crate::macros::decode_from_sized!(u32);

/// [`Decode`] implementation for `usize`.
///
/// This is decoded as an `u32`, or `Int` and will be checked that it's in
/// bounds.
///
/// # Examples
///
/// ```
/// let mut pod = pod::array();
/// pod.as_mut().write(10u32)?;
/// assert_eq!(pod.as_ref().read_sized::<usize>()?, 10);
///
/// let mut pod = pod::array();
/// pod.as_mut().write(10i32)?;
/// assert_eq!(pod.as_ref().read_sized::<usize>()?, 10);
/// # Ok::<_, pod::Error>(())
/// ```
impl<'de> SizedReadable<'de> for usize {
    const TYPE: Type = Type::INT;

    #[inline]
    fn read_content(reader: impl Reader<'de>, size: usize) -> Result<Self, Error> {
        let value = i32::read_content(reader, size)?;

        let Ok(value) = usize::try_from(value) else {
            return Err(Error::new(ErrorKind::InvalidUsize { value }));
        };

        Ok(value)
    }
}

crate::macros::decode_from_sized!(usize);

/// [`Decode`] implementation for `isize`.
///
/// This is decoded as an `i32`, or `Int` and will be checked that it's in
/// bounds.
///
/// # Examples
///
/// ```
/// let mut pod = pod::array();
/// pod.as_mut().write(-10)?;
/// assert_eq!(pod.as_ref().read_sized::<isize>()?, -10);
///
/// let mut pod = pod::array();
/// pod.as_mut().write(-10)?;
/// assert_eq!(pod.as_ref().read_sized::<isize>()?, -10);
/// # Ok::<_, pod::Error>(())
/// ```
impl<'de> SizedReadable<'de> for isize {
    const TYPE: Type = Type::INT;

    #[inline]
    fn read_content(reader: impl Reader<'de>, size: usize) -> Result<Self, Error> {
        let value = i32::read_content(reader, size)?;

        let Ok(value) = isize::try_from(value) else {
            return Err(Error::new(ErrorKind::InvalidIsize { value }));
        };

        Ok(value)
    }
}

crate::macros::decode_from_sized!(isize);

/// [`Decode`] implementation for `i64`.
///
/// # Examples
///
/// ```
/// let mut pod = pod::array();
/// pod.as_mut().write(10i64)?;
/// assert_eq!(pod.as_ref().read_sized::<i64>()?, 10i64);
/// # Ok::<_, pod::Error>(())
/// ```
impl<'de> SizedReadable<'de> for i64 {
    const TYPE: Type = Type::LONG;

    #[inline]
    fn read_content(mut reader: impl Reader<'de>, _: usize) -> Result<Self, Error> {
        Ok(reader.read::<u64>()?.cast_signed())
    }
}

crate::macros::decode_from_sized!(i64);

/// [`Decode`] implementation for `u64`.
///
/// # Examples
///
/// ```
/// let mut pod = pod::array();
/// pod.as_mut().write(10u64)?;
/// assert_eq!(pod.as_ref().read_sized::<i64>()?, 10);
///
/// let mut pod = pod::array();
/// pod.as_mut().write(10i64)?;
/// assert_eq!(pod.as_ref().read_sized::<i64>()?, 10);
/// # Ok::<_, pod::Error>(())
/// ```
impl<'de> SizedReadable<'de> for u64 {
    const TYPE: Type = Type::LONG;

    #[inline]
    fn read_content(reader: impl Reader<'de>, size: usize) -> Result<Self, Error> {
        Ok(i64::read_content(reader, size)?.cast_unsigned())
    }
}

crate::macros::decode_from_sized!(u64);

/// [`Decode`] implementation for `f32`.
///
/// # Examples
///
/// ```
/// let mut pod = pod::array();
/// pod.as_mut().write(42.42f32)?;
/// assert_eq!(pod.as_ref().read_sized::<f32>()?, 42.42f32);
/// # Ok::<_, pod::Error>(())
/// ```
impl<'de> SizedReadable<'de> for f32 {
    const TYPE: Type = Type::FLOAT;

    #[inline]
    fn read_content(mut reader: impl Reader<'de>, _: usize) -> Result<Self, Error> {
        Ok(f32::from_bits(reader.read()?))
    }
}

crate::macros::decode_from_sized!(f32);

/// [`Decode`] implementation for `f64`.
///
/// # Examples
///
/// ```
/// let mut pod = pod::array();
/// pod.as_mut().write(42.42f64)?;
/// assert_eq!(pod.as_ref().read_sized::<f64>()?, 42.42f64);
/// # Ok::<_, pod::Error>(())
/// ```
impl<'de> SizedReadable<'de> for f64 {
    const TYPE: Type = Type::DOUBLE;

    #[inline]
    fn read_content(mut reader: impl Reader<'de>, _: usize) -> Result<Self, Error> {
        Ok(f64::from_bits(reader.read::<u64>()?))
    }
}

crate::macros::decode_from_sized!(f64);

/// [`Decode`] implementation for [`Rectangle`].
///
/// # Examples
///
/// ```
/// use pod::{Pod, Rectangle};
///
/// let mut pod = pod::array();
/// pod.as_mut().write(Rectangle::new(100, 200))?;
/// assert_eq!(pod.as_ref().read_sized::<Rectangle>()?, Rectangle::new(100, 200));
/// # Ok::<_, pod::Error>(())
/// ```
impl<'de> SizedReadable<'de> for Rectangle {
    const TYPE: Type = Type::RECTANGLE;

    #[inline]
    fn read_content(mut reader: impl Reader<'de>, _: usize) -> Result<Self, Error> {
        let [width, height] = reader.read()?;
        Ok(Rectangle::new(width, height))
    }
}

crate::macros::decode_from_sized!(Rectangle);

/// [`Decode`] implementation for a [`Fraction`].
///
/// # Examples
///
/// ```
/// use pod::{Pod, Fraction};
///
/// let mut pod = pod::array();
/// pod.as_mut().write(Fraction::new(800, 600))?;
/// assert_eq!(pod.as_ref().read_sized::<Fraction>()?, Fraction::new(800, 600));
/// # Ok::<_, pod::Error>(())
/// ```
impl<'de> SizedReadable<'de> for Fraction {
    const TYPE: Type = Type::FRACTION;

    #[inline]
    fn read_content(mut reader: impl Reader<'de>, _: usize) -> Result<Self, Error> {
        let [num, denom] = reader.read()?;
        Ok(Fraction::new(num, denom))
    }
}

crate::macros::decode_from_sized!(Fraction);

/// [`Encode`] a an array of bytes `[u8; N]`.
///
/// # Examples
///
/// ```
/// use pod::{Pod, Fraction};
///
/// let mut pod = pod::array();
/// pod.as_mut().write(*b"hello world")?;
/// assert_eq!(pod.as_ref().read_unsized::<[u8]>()?, b"hello world");
/// # Ok::<_, pod::Error>(())
/// ```
impl<'de, const N: usize> SizedReadable<'de> for [u8; N] {
    const TYPE: Type = Type::BYTES;

    #[inline]
    fn read_content(reader: impl Reader<'de>, _: usize) -> Result<Self, Error> {
        let mut buf = ArrayVec::<u8, N>::new();

        <[u8]>::read_content(reader, N, |data: &[u8]| buf.extend_from_slice(data))??;

        let Some(array) = buf.into_inner() else {
            return Err(Error::new(ErrorKind::InvalidArrayLength));
        };

        Ok(array)
    }
}

crate::macros::decode_from_sized!(impl [const N: usize] [u8; N]);

/// Decode an owned c-string.
///
/// # Examples
///
/// ```
/// use std::ffi::CString;
/// let mut pod = pod::array();
/// pod.as_mut().write_unsized(c"hello world")?;
/// assert_eq!(pod.as_ref().read_sized::<CString>()?.as_c_str(), c"hello world");
/// # Ok::<_, pod::Error>(())
/// ```
#[cfg(feature = "alloc")]
impl<'de> SizedReadable<'de> for CString {
    const TYPE: Type = Type::STRING;

    #[inline]
    fn read_content(reader: impl Reader<'de>, size: usize) -> Result<Self, Error> {
        CStr::read_content(reader, size, CStr::to_owned)
    }
}

#[cfg(feature = "alloc")]
crate::macros::decode_from_sized!(CString);
crate::macros::decode_from_borrowed!(CStr);

/// Decode an owned [`String`].
///
/// # Examples
///
/// ```
/// let mut pod = pod::array();
///
/// pod.as_mut().write_unsized("hello world")?;
/// pod.as_mut().write_unsized("this is right")?;
///
/// let mut pod = pod.as_ref();
/// assert_eq!(pod.as_mut().read_sized::<String>()?, "hello world");
/// assert_eq!(pod.as_mut().read_sized::<String>()?, "this is right");
/// # Ok::<_, pod::Error>(())
/// ```
#[cfg(feature = "alloc")]
impl<'de> SizedReadable<'de> for String {
    const TYPE: Type = Type::STRING;

    #[inline]
    fn read_content(reader: impl Reader<'de>, size: usize) -> Result<Self, Error> {
        str::read_content(reader, size, str::to_owned)
    }
}

#[cfg(feature = "alloc")]
crate::macros::decode_from_sized!(String);
crate::macros::decode_from_borrowed!(str);

/// Decode an owned vector of bytes [`Vec<u8>`].
///
/// # Examples
///
/// ```
/// let mut pod = pod::array();
///
/// pod.as_mut().write(*b"hello world")?;
/// pod.as_mut().write(*b"this is right")?;
///
/// let mut pod = pod.as_ref();
/// assert_eq!(pod.as_mut().read_sized::<Vec<u8>>()?, b"hello world");
/// assert_eq!(pod.as_mut().read_sized::<Vec<u8>>()?, b"this is right");
/// # Ok::<_, pod::Error>(())
/// ```
#[cfg(feature = "alloc")]
impl<'de> SizedReadable<'de> for Vec<u8> {
    const TYPE: Type = Type::BYTES;

    #[inline]
    fn read_content(reader: impl Reader<'de>, size: usize) -> Result<Self, Error> {
        <[u8]>::read_content(reader, size, <[u8]>::to_vec)
    }
}

#[cfg(feature = "alloc")]
crate::macros::decode_from_sized!(Vec<u8>);
crate::macros::decode_from_borrowed!([u8]);

/// Decode an owned [`OwnedBitmap`].
///
/// # Examples
///
/// ```
/// use pod::{Bitmap, Pod, OwnedBitmap};
///
/// let mut pod = pod::array();
/// pod.as_mut().write_unsized(Bitmap::new(b"hello world"))?;
/// assert_eq!(pod.as_ref().read_sized::<OwnedBitmap>()?.as_bytes(), b"hello world");
/// # Ok::<_, pod::Error>(())
/// ```
#[cfg(feature = "alloc")]
impl<'de> SizedReadable<'de> for OwnedBitmap {
    const TYPE: Type = Type::BITMAP;

    #[inline]
    fn read_content(reader: impl Reader<'de>, size: usize) -> Result<Self, Error> {
        Bitmap::read_content(reader, size, Bitmap::to_owned)
    }
}

#[cfg(feature = "alloc")]
crate::macros::decode_from_sized!(OwnedBitmap);
crate::macros::decode_from_borrowed!(Bitmap);

/// [`Decode`] implementation for [`Pointer`].
///
/// # Examples
///
/// ```
/// use pod::{Pod, Pointer};
///
/// let value = 1u32;
///
/// let mut pod = pod::array();
/// pod.as_mut().write(Pointer::new((&value as *const u32).addr()))?;
/// assert_eq!(pod.as_ref().read_sized::<Pointer>()?, Pointer::new((&value as *const u32).addr()));
/// # Ok::<_, pod::Error>(())
/// ```
impl<'de> SizedReadable<'de> for Pointer {
    const TYPE: Type = Type::POINTER;

    #[inline]
    fn read_content(mut reader: impl Reader<'de>, _: usize) -> Result<Self, Error> {
        let [ty, _pad, p1, p2] = reader.read::<[u32; 4]>()?;

        let mut bytes = WordBytes::new();
        bytes.write_half_words([p1, p2]);
        Ok(Pointer::new_with_type(bytes.read_usize(), ty))
    }
}

#[cfg(feature = "alloc")]
crate::macros::decode_from_sized!(Pointer);

/// [`Decode`] implementation for [`Fd`].
///
/// # Examples
///
/// ```
/// use pod::{Pod, Fd};
///
/// let mut pod = pod::array();
/// pod.as_mut().write(Fd::new(4))?;
/// assert_eq!(pod.as_ref().read_sized::<Fd>()?, Fd::new(4));
/// # Ok::<_, pod::Error>(())
/// ```
impl<'de> SizedReadable<'de> for Fd {
    const TYPE: Type = Type::FD;

    #[inline]
    fn read_content(mut reader: impl Reader<'de>, _: usize) -> Result<Self, Error> {
        Ok(Self::new(reader.read::<u64>()?.cast_signed()))
    }
}

#[cfg(feature = "alloc")]
crate::macros::decode_from_sized!(Fd);
