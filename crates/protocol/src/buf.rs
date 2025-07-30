#[cfg(test)]
mod tests;

mod recv_buf;
pub use self::recv_buf::RecvBuf;

mod send_buf;
pub use self::send_buf::SendBuf;

use core::error;
use core::fmt;

/// An allocation error has occured when trying to reserve space in the [`RecvBuf`].
#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
#[non_exhaustive]
pub struct AllocError;

impl error::Error for AllocError {}

impl fmt::Display for AllocError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Allocation error")
    }
}

/// Capacity overflow when writing to an [`ArrayVec`].
#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
#[non_exhaustive]
pub struct CapacityError;

impl error::Error for CapacityError {}

impl fmt::Display for CapacityError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Buffer capacity exceeded")
    }
}
