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

    #[cfg(feature = "alloc")]
    use crate::DynamicBuf;
    use crate::{ArrayBuf, AsReader};

    pub trait Sealed {}

    #[cfg(feature = "alloc")]
    impl Sealed for Box<[u64]> {}
    #[cfg(feature = "alloc")]
    impl Sealed for Vec<u64> {}
    impl Sealed for [u64] {}
    impl<const N: usize> Sealed for ArrayBuf<N> {}
    #[cfg(feature = "alloc")]
    impl Sealed for DynamicBuf {}
    impl<R> Sealed for &mut R where R: ?Sized + AsReader {}
    impl<R> Sealed for &R where R: ?Sized + AsReader {}
}

/// Base trait to convert something into a reader which borrows from `&self`.
pub trait AsReader
where
    Self: self::sealed::Sealed,
{
    /// A clone of the reader.
    type AsReader<'this>: Reader<'this>
    where
        Self: 'this;

    /// Borrow the value as a reader.
    fn as_reader(&self) -> Self::AsReader<'_>;
}

#[cfg(feature = "alloc")]
impl AsReader for Box<[u64]> {
    type AsReader<'this>
        = &'this [u64]
    where
        Self: 'this;

    #[inline]
    fn as_reader(&self) -> Self::AsReader<'_> {
        self
    }
}

#[cfg(feature = "alloc")]
impl AsReader for Vec<u64> {
    type AsReader<'this>
        = &'this [u64]
    where
        Self: 'this;

    #[inline]
    fn as_reader(&self) -> Self::AsReader<'_> {
        self
    }
}

impl AsReader for [u64] {
    type AsReader<'this>
        = &'this [u64]
    where
        Self: 'this;

    #[inline]
    fn as_reader(&self) -> Self::AsReader<'_> {
        self
    }
}

impl<R> AsReader for &mut R
where
    R: ?Sized + AsReader,
{
    type AsReader<'this>
        = R::AsReader<'this>
    where
        Self: 'this;

    #[inline]
    fn as_reader(&self) -> Self::AsReader<'_> {
        (**self).as_reader()
    }
}

impl<R> AsReader for &R
where
    R: ?Sized + AsReader,
{
    type AsReader<'this>
        = R::AsReader<'this>
    where
        Self: 'this;

    #[inline]
    fn as_reader(&self) -> Self::AsReader<'_> {
        (**self).as_reader()
    }
}
