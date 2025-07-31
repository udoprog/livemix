use core::ffi::CStr;

use crate::error::ErrorKind;
use crate::{Bitmap, Error, Reader, Type, Visitor};

mod sealed {
    use core::ffi::CStr;

    use super::Bitmap;

    pub trait Sealed {}

    impl Sealed for Bitmap {}
    impl Sealed for [u8] {}
    impl Sealed for CStr {}
    impl Sealed for str {}
}

/// A trait for unsized types that can be decoded.
pub trait UnsizedReadable<'de>
where
    Self: self::sealed::Sealed,
{
    const TYPE: Type;

    #[doc(hidden)]
    fn read_content<V>(reader: impl Reader<'de>, size: usize, visitor: V) -> Result<V::Ok, Error>
    where
        V: Visitor<'de, Self>;

    #[inline]
    #[doc(hidden)]
    fn read_borrowed(reader: impl Reader<'de>, size: usize) -> Result<&'de Self, Error> {
        struct LocalVisitor;

        impl<'de, T> Visitor<'de, T> for LocalVisitor
        where
            T: 'de + ?Sized,
        {
            type Ok = &'de T;

            #[inline]
            fn visit_borrowed(self, value: &'de T) -> Result<Self::Ok, Error> {
                Ok(value)
            }
        }

        Self::read_content(reader, size, LocalVisitor)
    }
}

/// [`UnsizedReadable`] implementation for an unsized [`CStr`].
///
/// # Examples
///
/// ```
/// use core::ffi::CStr;
/// let mut pod = pod::array();
/// pod.as_mut().write_unsized(c"hello world")?;
/// assert_eq!(pod.as_ref().read_unsized::<CStr>()?, c"hello world");
/// # Ok::<_, pod::Error>(())
/// ```
impl<'de> UnsizedReadable<'de> for CStr {
    const TYPE: Type = Type::STRING;

    #[inline]
    fn read_content<V>(
        mut reader: impl Reader<'de>,
        size: usize,
        visitor: V,
    ) -> Result<V::Ok, Error>
    where
        V: Visitor<'de, Self>,
    {
        struct LocalVisitor<V> {
            visitor: V,
        }

        impl<'de, V> Visitor<'de, [u8]> for LocalVisitor<V>
        where
            V: Visitor<'de, CStr>,
        {
            type Ok = V::Ok;

            #[inline]
            fn visit_borrowed(self, bytes: &'de [u8]) -> Result<Self::Ok, Error> {
                let Ok(str) = CStr::from_bytes_with_nul(bytes) else {
                    return Err(Error::new(ErrorKind::NonTerminatedString));
                };

                self.visitor.visit_borrowed(str)
            }

            #[inline]
            fn visit_ref(self, bytes: &[u8]) -> Result<Self::Ok, Error> {
                let Ok(str) = CStr::from_bytes_with_nul(bytes) else {
                    return Err(Error::new(ErrorKind::NonTerminatedString));
                };

                self.visitor.visit_ref(str)
            }
        }

        let visitor = LocalVisitor { visitor };
        reader.read_bytes(size, visitor)
    }
}

/// [`UnsizedReadable`] implementation for an unsized [`str`].
///
/// # Examples
///
/// ```
/// let mut pod = pod::array();
/// pod.as_mut().write_unsized("hello world")?;
/// assert_eq!(pod.as_ref().read_unsized::<str>()?, "hello world");
/// # Ok::<_, pod::Error>(())
/// ```
impl<'de> UnsizedReadable<'de> for str {
    const TYPE: Type = Type::STRING;

    #[inline]
    fn read_content<V>(
        mut reader: impl Reader<'de>,
        size: usize,
        visitor: V,
    ) -> Result<V::Ok, Error>
    where
        V: Visitor<'de, Self>,
    {
        struct LocalVisitor<V>(V);

        impl<'de, V> Visitor<'de, [u8]> for LocalVisitor<V>
        where
            V: Visitor<'de, str>,
        {
            type Ok = V::Ok;

            #[inline]
            fn visit_borrowed(self, bytes: &'de [u8]) -> Result<Self::Ok, Error> {
                self.0.visit_borrowed(read_string(bytes)?)
            }

            #[inline]
            fn visit_ref(self, bytes: &[u8]) -> Result<Self::Ok, Error> {
                self.0.visit_ref(read_string(bytes)?)
            }
        }

        reader.read_bytes(size, LocalVisitor(visitor))
    }
}

/// [`UnsizedReadable`] implementation for an unsized `[u8]`.
///
/// # Examples
///
/// ```
/// let mut pod = pod::array();
/// pod.as_mut().write_unsized(&b"hello world"[..])?;
/// assert_eq!(pod.as_ref().read_unsized::<[u8]>()?, b"hello world");
/// # Ok::<_, pod::Error>(())
/// ```
impl<'de> UnsizedReadable<'de> for [u8] {
    const TYPE: Type = Type::BYTES;

    #[inline]
    fn read_content<V>(
        mut reader: impl Reader<'de>,
        size: usize,
        visitor: V,
    ) -> Result<V::Ok, Error>
    where
        V: Visitor<'de, Self>,
    {
        reader.read_bytes(size, visitor)
    }
}

/// [`UnsizedReadable`] implementation for an unsized [`Bitmap`].
///
/// # Examples
///
/// ```
/// use pod::{Bitmap, Pod};
///
/// let mut pod = pod::array();
/// pod.as_mut().write_unsized(Bitmap::new(b"asdfasdf"))?;
/// assert_eq!(pod.as_ref().read_unsized::<Bitmap>()?, b"asdfasdf");
/// # Ok::<_, pod::Error>(())
/// ```
impl<'de> UnsizedReadable<'de> for Bitmap {
    const TYPE: Type = Type::BITMAP;

    #[inline]
    fn read_content<V>(
        mut reader: impl Reader<'de>,
        size: usize,
        visitor: V,
    ) -> Result<V::Ok, Error>
    where
        V: Visitor<'de, Self>,
    {
        struct LocalVisitor<V>(V);

        impl<'de, V> Visitor<'de, [u8]> for LocalVisitor<V>
        where
            V: Visitor<'de, Bitmap>,
        {
            type Ok = V::Ok;

            #[inline]
            fn visit_borrowed(self, value: &'de [u8]) -> Result<Self::Ok, Error> {
                self.0.visit_borrowed(Bitmap::new(value))
            }

            #[inline]
            fn visit_ref(self, value: &[u8]) -> Result<Self::Ok, Error> {
                self.0.visit_ref(Bitmap::new(value))
            }
        }

        reader.read_bytes(size, LocalVisitor(visitor))
    }
}

fn read_string(bytes: &[u8]) -> Result<&str, Error> {
    let bytes = match bytes {
        [head @ .., 0] => head,
        _ => return Err(Error::new(ErrorKind::NonTerminatedString)),
    };

    let Ok(str) = str::from_utf8(bytes) else {
        return Err(Error::new(ErrorKind::NotUtf8));
    };

    Ok(str)
}
