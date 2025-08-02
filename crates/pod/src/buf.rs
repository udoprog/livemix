mod array_buf;

pub use self::array_buf::{ArrayBuf, ArrayBufPos};

mod array_vec;
pub use self::array_vec::ArrayVec;

#[cfg(feature = "alloc")]
mod dynamic_buf;
#[cfg(feature = "alloc")]
pub use self::dynamic_buf::{AllocError, DynamicBuf, DynamicBufPos};

mod slice;
pub use self::slice::Slice;

mod writer_slice;
pub use self::writer_slice::WriterSlice;

use core::error;
use core::fmt;

/// Convenience function to construct a reader from a slice.
pub fn slice(data: &[u8]) -> Slice<'_> {
    Slice::new(data)
}

/// Capacity overflow when writing to an [`ArrayBuf`].
#[derive(Debug, PartialEq)]
#[non_exhaustive]
pub struct CapacityError;

impl error::Error for CapacityError {}

impl fmt::Display for CapacityError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Buffer capacity exceeded")
    }
}
