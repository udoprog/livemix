#[cfg(feature = "alloc")]
use core::slice;

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

use crate::Reader;

mod sealed {
    #[cfg(feature = "alloc")]
    use alloc::vec::Vec;

    #[cfg(feature = "alloc")]
    use crate::DynamicBuf;
    use crate::{ArrayBuf, SplitReader};

    pub trait Sealed<T> {}

    impl<T, const N: usize> Sealed<T> for ArrayBuf<T, N> where T: 'static {}
    impl<T> Sealed<T> for DynamicBuf<T> where T: 'static {}
    impl<T> Sealed<T> for Vec<T> where T: 'static {}
    impl<T> Sealed<T> for &[T] where T: 'static {}
    impl<R, T> Sealed<T> for &mut R where R: ?Sized + SplitReader<T> {}
}

/// Base trait to convert something into a reader which borrows from `&self`.
pub trait SplitReader<T>
where
    Self: self::sealed::Sealed<T>,
{
    /// A clone of the reader.
    type TakeReader<'this>: Reader<'this, T>
    where
        Self: 'this;

    /// Borrow the value as a reader.
    fn take_reader(&mut self) -> Self::TakeReader<'_>;
}

#[cfg(feature = "alloc")]
impl<T> SplitReader<T> for Vec<T>
where
    T: 'static,
{
    type TakeReader<'this>
        = &'this [T]
    where
        Self: 'this;

    #[inline]
    fn take_reader(&mut self) -> Self::TakeReader<'_> {
        let ptr = self.as_ptr();
        let len = self.len();
        self.clear();
        // SAFETY: The vector is guaranteed to be initialized up to `len`.
        unsafe { slice::from_raw_parts(ptr, len) }
    }
}

impl<T> SplitReader<T> for &[T]
where
    T: 'static,
{
    type TakeReader<'this>
        = &'this [T]
    where
        Self: 'this;

    #[inline]
    fn take_reader(&mut self) -> Self::TakeReader<'_> {
        let (this, rest) = self.split_at(0);
        *self = this;
        rest
    }
}

impl<R, T> SplitReader<T> for &mut R
where
    R: ?Sized + SplitReader<T>,
{
    type TakeReader<'this>
        = R::TakeReader<'this>
    where
        Self: 'this;

    #[inline]
    fn take_reader(&mut self) -> Self::TakeReader<'_> {
        (**self).take_reader()
    }
}
