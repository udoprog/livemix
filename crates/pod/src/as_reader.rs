#[cfg(feature = "alloc")]
use alloc::boxed::Box;
#[cfg(feature = "alloc")]
use alloc::vec::Vec;

use crate::Reader;

mod sealed {
    #[cfg(feature = "alloc")]
    use alloc::boxed::Box;
    #[cfg(feature = "alloc")]
    use alloc::vec::Vec;

    use crate::{AsReader, Buf};

    pub trait Sealed<T> {}

    #[cfg(feature = "alloc")]
    impl<T> Sealed<T> for Box<[T]> where T: 'static {}

    #[cfg(feature = "alloc")]
    impl<T> Sealed<T> for Vec<T> where T: 'static {}

    impl<T> Sealed<T> for [T] where T: 'static {}

    impl<T, const N: usize> Sealed<T> for Buf<T, N> {}

    impl<R, T> Sealed<T> for &mut R where R: ?Sized + AsReader<T> {}

    impl<R, T> Sealed<T> for &R where R: ?Sized + AsReader<T> {}
}

/// Base trait to convert something into a reader which borrows from `&self`.
pub trait AsReader<T>
where
    Self: self::sealed::Sealed<T>,
{
    /// A clone of the reader.
    type Reader<'this>: Reader<'this, T>
    where
        Self: 'this;

    /// Borrow the value as a reader.
    fn as_reader(&self) -> Self::Reader<'_>;
}

#[cfg(feature = "alloc")]
impl<T> AsReader<T> for Box<[T]>
where
    T: 'static,
{
    type Reader<'this>
        = &'this [T]
    where
        Self: 'this;

    #[inline]
    fn as_reader(&self) -> Self::Reader<'_> {
        self
    }
}

#[cfg(feature = "alloc")]
impl<T> AsReader<T> for Vec<T>
where
    T: 'static,
{
    type Reader<'this>
        = &'this [T]
    where
        Self: 'this;

    #[inline]
    fn as_reader(&self) -> Self::Reader<'_> {
        self
    }
}

impl<T> AsReader<T> for [T]
where
    T: 'static,
{
    type Reader<'this>
        = &'this [T]
    where
        Self: 'this;

    #[inline]
    fn as_reader(&self) -> Self::Reader<'_> {
        self
    }
}

impl<R, T> AsReader<T> for &mut R
where
    R: ?Sized + AsReader<T>,
{
    type Reader<'this>
        = R::Reader<'this>
    where
        Self: 'this;

    #[inline]
    fn as_reader(&self) -> Self::Reader<'_> {
        (**self).as_reader()
    }
}

impl<R, T> AsReader<T> for &R
where
    R: ?Sized + AsReader<T>,
{
    type Reader<'this>
        = R::Reader<'this>
    where
        Self: 'this;

    #[inline]
    fn as_reader(&self) -> Self::Reader<'_> {
        (**self).as_reader()
    }
}
