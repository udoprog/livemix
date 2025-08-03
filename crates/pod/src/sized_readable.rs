use core::any;
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
    /// Read the content of a type.
    #[doc(hidden)]
    fn read_content(reader: impl Reader<'de>, ty: Type, size: usize) -> Result<Self, Error>;
}

#[inline]
fn read_integer<'de, T>(mut reader: impl Reader<'de>, ty: Type, size: usize) -> Result<T, Error>
where
    T: SizedReadable<'de>,
    T: TryFrom<i32> + TryFrom<i64>,
{
    match (ty, size) {
        (Type::INT, 4) => {
            let value = reader.read::<i32>()?;

            let Ok(value) = T::try_from(value) else {
                return Err(Error::new(ErrorKind::InvalidInt {
                    value,
                    ty: any::type_name::<T>(),
                }));
            };

            Ok(value)
        }
        (Type::LONG, 8) => {
            let value = reader.read::<i64>()?;

            let Ok(value) = T::try_from(value) else {
                return Err(Error::new(ErrorKind::InvalidLong {
                    value,
                    ty: any::type_name::<T>(),
                }));
            };

            Ok(value)
        }
        (ty, size) => Err(Error::new(ErrorKind::ExpectedNumber { actual: ty, size })),
    }
}

macro_rules! signed {
    ($($ty:ty),* $(,)?) => {
        $(
            #[doc = concat!(" [`SizedReadable`] implementation for `", stringify!($ty), "`.")]
            ///
            /// This is decoded as an `Int` or `Long` and will be checked that it's
            /// in bounds.
            ///
            /// # Examples
            ///
            /// ```
            /// let mut pod = pod::array();
            /// pod.as_mut().write(-10i32)?;
            #[doc = concat!(" assert_eq!(pod.as_ref().read_sized::<", stringify!($ty), ">()?, -10);")]
            ///
            /// let mut pod = pod::array();
            /// pod.as_mut().write(-10i64)?;
            #[doc = concat!(" assert_eq!(pod.as_ref().read_sized::<", stringify!($ty), ">()?, -10);")]
            /// # Ok::<_, pod::Error>(())
            /// ```
            impl<'de> SizedReadable<'de> for $ty {
                #[inline]
                fn read_content(reader: impl Reader<'de>, ty: Type, size: usize) -> Result<Self, Error> {
                    read_integer(reader, ty, size)
                }
            }

            crate::macros::decode_from_sized!($ty);
        )*
    }
}

macro_rules! unsigned {
    ($($ty:ty),* $(,)?) => {
        $(
            #[doc = concat!(" [`SizedReadable`] implementation for `", stringify!($ty), "`.")]
            ///
            /// This is decoded as an `Int` or `Long` and will be checked that it's
            /// in bounds.
            ///
            /// # Examples
            ///
            /// ```
            /// let mut pod = pod::array();
            /// pod.as_mut().write(10i32)?;
            #[doc = concat!(" assert_eq!(pod.as_ref().read_sized::<", stringify!($ty), ">()?, 10);")]
            ///
            /// let mut pod = pod::array();
            /// pod.as_mut().write(10i64)?;
            #[doc = concat!(" assert_eq!(pod.as_ref().read_sized::<", stringify!($ty), ">()?, 10);")]
            /// # Ok::<_, pod::Error>(())
            /// ```
            impl<'de> SizedReadable<'de> for $ty {
                #[inline]
                fn read_content(reader: impl Reader<'de>, ty: Type, size: usize) -> Result<Self, Error> {
                    read_integer(reader, ty, size)
                }
            }

            crate::macros::decode_from_sized!($ty);
        )*
    }
}

/// [`SizedReadable`] implementation for `bool`.
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
    #[inline]
    fn read_content(mut reader: impl Reader<'de>, ty: Type, size: usize) -> Result<Self, Error> {
        if Type::BOOL != ty {
            return Err(Error::expected(Type::BOOL, ty, size));
        }

        Ok(reader.read::<u32>()? != 0)
    }
}

crate::macros::decode_from_sized!(bool);

/// [`SizedReadable`] implementation for any type that can be converted into an
/// [`Id`].
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
    #[inline]
    fn read_content(mut reader: impl Reader<'de>, ty: Type, size: usize) -> Result<Self, Error> {
        if Type::ID != ty {
            return Err(Error::expected(Type::ID, ty, size));
        }

        Ok(Id(I::from_id(reader.read()?)))
    }
}

crate::macros::decode_from_sized!(impl [I] Id<I> where I: RawId);

signed!(i16, i32, i64, i128, isize);
unsigned!(u16, u32, u64, u128, usize);

/// [`SizedReadable`] implementation for `f32`.
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
    #[inline]
    fn read_content(mut reader: impl Reader<'de>, ty: Type, size: usize) -> Result<Self, Error> {
        if Type::FLOAT != ty || size != 4 {
            return Err(Error::new(ErrorKind::Expected {
                expected: Type::FLOAT,
                actual: ty,
                size,
            }));
        }

        Ok(f32::from_bits(reader.read()?))
    }
}

crate::macros::decode_from_sized!(f32);

/// [`SizedReadable`] implementation for `f64`.
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
    #[inline]
    fn read_content(mut reader: impl Reader<'de>, ty: Type, size: usize) -> Result<Self, Error> {
        if Type::DOUBLE != ty {
            return Err(Error::expected(Type::DOUBLE, ty, size));
        }

        Ok(f64::from_bits(reader.read::<u64>()?))
    }
}

crate::macros::decode_from_sized!(f64);

/// [`SizedReadable`] implementation for [`Rectangle`].
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
    #[inline]
    fn read_content(mut reader: impl Reader<'de>, ty: Type, size: usize) -> Result<Self, Error> {
        if Type::RECTANGLE != ty {
            return Err(Error::expected(Type::RECTANGLE, ty, size));
        }

        let [width, height] = reader.read()?;
        Ok(Rectangle::new(width, height))
    }
}

crate::macros::decode_from_sized!(Rectangle);

/// [`SizedReadable`] implementation for a [`Fraction`].
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
    #[inline]
    fn read_content(mut reader: impl Reader<'de>, ty: Type, size: usize) -> Result<Self, Error> {
        if Type::FRACTION != ty {
            return Err(Error::expected(Type::FRACTION, ty, size));
        }

        let [num, denom] = reader.read()?;
        Ok(Fraction::new(num, denom))
    }
}

crate::macros::decode_from_sized!(Fraction);

/// [`SizedReadable`] a an array of bytes `[u8; N]`.
///
/// # Errors
///
/// Decoding a fixed-size array of the wrong size will return an error.
///
/// ```
/// use pod::Pod;
///
/// let mut pod = pod::array();
/// pod.as_mut().write(*b"hello")?;
/// assert!(pod.as_ref().read_sized::<[u8; 42]>().is_err());
/// # Ok::<_, pod::Error>(())
/// ```
///
/// # Examples
///
/// ```
/// use pod::Pod;
///
/// let mut pod = pod::array();
/// pod.as_mut().write(*b"hello world")?;
/// assert_eq!(pod.as_ref().read_sized::<[u8; 11]>()?, *b"hello world");
/// assert_eq!(pod.as_ref().read_unsized::<[u8]>()?, b"hello world");
/// # Ok::<_, pod::Error>(())
/// ```
impl<'de, const N: usize> SizedReadable<'de> for [u8; N] {
    #[inline]
    fn read_content(reader: impl Reader<'de>, ty: Type, size: usize) -> Result<Self, Error> {
        if Type::BYTES != ty {
            return Err(Error::expected(Type::BYTES, ty, size));
        }

        if size != N {
            return Err(Error::new(ErrorKind::ExpectedSize {
                ty,
                expected: N,
                actual: size,
            }));
        }

        let mut buf = ArrayVec::<u8, N>::new();

        <[u8]>::read_content(reader, N, |data: &[u8]| buf.extend_from_slice(data))??;

        let Some(array) = buf.into_inner() else {
            return Err(Error::new(ErrorKind::InvalidArrayLength));
        };

        Ok(array)
    }
}

crate::macros::decode_from_sized!(impl [const N: usize] [u8; N]);

/// [`SizedReadable`] implementation for an owned [`CString`].
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
    #[inline]
    fn read_content(reader: impl Reader<'de>, ty: Type, size: usize) -> Result<Self, Error> {
        if Type::STRING != ty {
            return Err(Error::expected(Type::STRING, ty, size));
        }

        CStr::read_content(reader, size, CStr::to_owned)
    }
}

#[cfg(feature = "alloc")]
crate::macros::decode_from_sized!(CString);
crate::macros::decode_from_borrowed!(CStr);

/// Read an owned [`String`].
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
    #[inline]
    fn read_content(reader: impl Reader<'de>, ty: Type, size: usize) -> Result<Self, Error> {
        if Type::STRING != ty {
            return Err(Error::expected(Type::STRING, ty, size));
        }

        str::read_content(reader, size, str::to_owned)
    }
}

#[cfg(feature = "alloc")]
crate::macros::decode_from_sized!(String);
crate::macros::decode_from_borrowed!(str);

/// Read an owned vector of bytes [`Vec<u8>`].
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
    #[inline]
    fn read_content(reader: impl Reader<'de>, ty: Type, size: usize) -> Result<Self, Error> {
        if Type::BYTES != ty {
            return Err(Error::expected(Type::BYTES, ty, size));
        }

        <[u8]>::read_content(reader, size, <[u8]>::to_vec)
    }
}

#[cfg(feature = "alloc")]
crate::macros::decode_from_sized!(Vec<u8>);
crate::macros::decode_from_borrowed!([u8]);

/// Read an owned [`OwnedBitmap`].
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
    #[inline]
    fn read_content(reader: impl Reader<'de>, ty: Type, size: usize) -> Result<Self, Error> {
        if Type::BITMAP != ty {
            return Err(Error::expected(Type::BITMAP, ty, size));
        }

        Bitmap::read_content(reader, size, Bitmap::to_owned)
    }
}

#[cfg(feature = "alloc")]
crate::macros::decode_from_sized!(OwnedBitmap);
crate::macros::decode_from_borrowed!(Bitmap);

/// [`SizedReadable`] implementation for [`Pointer`].
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
    #[inline]
    fn read_content(mut reader: impl Reader<'de>, ty: Type, size: usize) -> Result<Self, Error> {
        if Type::POINTER != ty {
            return Err(Error::expected(Type::POINTER, ty, size));
        }

        let [ty, _pad, p1, p2] = reader.read::<[u32; 4]>()?;

        let mut bytes = WordBytes::new();
        bytes.write_half_words([p1, p2]);
        Ok(Pointer::new_with_type(bytes.read_usize(), ty))
    }
}

#[cfg(feature = "alloc")]
crate::macros::decode_from_sized!(Pointer);

/// [`SizedReadable`] implementation for [`Fd`].
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
    #[inline]
    fn read_content(mut reader: impl Reader<'de>, ty: Type, size: usize) -> Result<Self, Error> {
        if Type::FD != ty {
            return Err(Error::expected(Type::FD, ty, size));
        }

        Ok(Self::new(reader.read::<u64>()?.cast_signed()))
    }
}

#[cfg(feature = "alloc")]
crate::macros::decode_from_sized!(Fd);
