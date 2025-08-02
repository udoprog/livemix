#[cfg(feature = "alloc")]
use alloc::boxed::Box;
#[cfg(feature = "alloc")]
use alloc::vec::Vec;

use crate::Slice;

mod sealed {
    #[cfg(feature = "alloc")]
    use alloc::boxed::Box;
    #[cfg(feature = "alloc")]
    use alloc::vec::Vec;

    #[cfg(feature = "alloc")]
    use crate::DynamicBuf;
    use crate::{ArrayBuf, AsSlice, Slice, Writer, WriterSlice};

    pub trait Sealed {}

    #[cfg(feature = "alloc")]
    impl Sealed for Box<[u8]> {}
    #[cfg(feature = "alloc")]
    impl Sealed for Vec<u8> {}
    impl Sealed for [u8] {}
    impl Sealed for Slice<'_> {}
    impl<const N: usize> Sealed for ArrayBuf<N> {}
    #[cfg(feature = "alloc")]
    impl Sealed for DynamicBuf {}
    impl<R> Sealed for &mut R where R: ?Sized + AsSlice {}
    impl<R> Sealed for &R where R: ?Sized + AsSlice {}
    impl<B, T> Sealed for WriterSlice<B, T> where B: Writer {}
}

/// Base trait to convert something into a reader which borrows from `&self`.
pub trait AsSlice
where
    Self: self::sealed::Sealed,
{
    /// Borrow the value as a reader.
    fn as_slice(&self) -> Slice<'_>;
}

#[cfg(feature = "alloc")]
impl AsSlice for Box<[u8]> {
    #[inline]
    fn as_slice(&self) -> Slice<'_> {
        Slice::new(self)
    }
}

#[cfg(feature = "alloc")]
impl AsSlice for Vec<u8> {
    #[inline]
    fn as_slice(&self) -> Slice<'_> {
        Slice::new(self)
    }
}

impl AsSlice for [u8] {
    #[inline]
    fn as_slice(&self) -> Slice<'_> {
        Slice::new(self)
    }
}

impl<R> AsSlice for &mut R
where
    R: ?Sized + AsSlice,
{
    #[inline]
    fn as_slice(&self) -> Slice<'_> {
        (**self).as_slice()
    }
}

impl<R> AsSlice for &R
where
    R: ?Sized + AsSlice,
{
    #[inline]
    fn as_slice(&self) -> Slice<'_> {
        (**self).as_slice()
    }
}
