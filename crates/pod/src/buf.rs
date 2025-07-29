mod array_buf;
pub use self::array_buf::{ArrayBuf, CapacityError};

#[cfg(feature = "alloc")]
mod dynamic_buf;
#[cfg(feature = "alloc")]
pub use self::dynamic_buf::{AllocError, DynamicBuf};
