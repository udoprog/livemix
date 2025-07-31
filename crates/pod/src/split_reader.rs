#[cfg(feature = "alloc")]
use core::slice;

#[cfg(feature = "alloc")]
use crate::SliceBuf;
#[cfg(feature = "alloc")]
use alloc::vec::Vec;

use crate::Reader;

mod sealed {
    #[cfg(feature = "alloc")]
    use alloc::vec::Vec;

    #[cfg(feature = "alloc")]
    use crate::DynamicBuf;
    use crate::{ArrayBuf, SliceBuf, SplitReader};

    pub trait Sealed {}

    impl<const N: usize> Sealed for ArrayBuf<N> {}
    impl Sealed for DynamicBuf {}
    impl Sealed for Vec<u8> {}
    impl Sealed for SliceBuf<'_> {}
    impl<R> Sealed for &mut R where R: ?Sized + SplitReader {}
}

/// Base trait to convert something into a reader which borrows from `&self`.
pub trait SplitReader
where
    Self: self::sealed::Sealed,
{
    /// A clone of the reader.
    type TakeReader<'this>: Reader<'this>
    where
        Self: 'this;

    /// Borrow the value as a reader.
    fn take_reader(&mut self) -> Self::TakeReader<'_>;
}

#[cfg(feature = "alloc")]
impl SplitReader for Vec<u8> {
    type TakeReader<'this>
        = SliceBuf<'this>
    where
        Self: 'this;

    #[inline]
    fn take_reader(&mut self) -> Self::TakeReader<'_> {
        let ptr = self.as_ptr();
        let len = self.len();
        self.clear();
        // SAFETY: The vector is guaranteed to be initialized up to `len`.
        SliceBuf::new(unsafe { slice::from_raw_parts(ptr, len) })
    }
}

impl<R> SplitReader for &mut R
where
    R: ?Sized + SplitReader,
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
