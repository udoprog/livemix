use core::fmt;

use crate::Type;
#[cfg(feature = "alloc")]
use crate::buf::AllocError;
use crate::buf::CapacityError;

#[derive(PartialEq)]
#[non_exhaustive]
pub struct Error {
    kind: ErrorKind,
}

impl Error {
    /// Create a new `Error` with the specified kind.
    #[inline]
    pub(crate) fn new<K>(kind: K) -> Self
    where
        ErrorKind: From<K>,
    {
        Self {
            kind: ErrorKind::from(kind),
        }
    }

    /// Get the kind of error.
    #[inline]
    #[cfg(all(test, feature = "alloc"))]
    pub(crate) fn kind(&self) -> &ErrorKind {
        &self.kind
    }
}

impl<E> From<E> for Error
where
    ErrorKind: From<E>,
{
    #[inline]
    fn from(e: E) -> Self {
        Self::new(e)
    }
}

#[derive(Debug, PartialEq)]
pub(crate) enum ErrorKind {
    StructUnderflow,
    ObjectUnderflow,
    SizeOverflow,
    BufferUnderflow,
    NonTerminatedString,
    NullContainingString,
    NotUtf8,
    NotSupportedRef,
    InvalidArrayLength,
    UnsizedTypeInArray {
        ty: Type,
    },
    Expected {
        expected: Type,
        actual: Type,
    },
    ReservedSizeMismatch {
        expected: usize,
        actual: usize,
    },
    ReservedOverflow {
        write: usize,
        len: usize,
        capacity: usize,
    },
    ChildSizeMismatch {
        expected: usize,
        actual: usize,
    },
    InvalidUsize {
        value: i32,
    },
    InvalidIsize {
        value: i32,
    },
    InvalidUsizeInt {
        value: usize,
    },
    InvalidIsizeInt {
        value: isize,
    },
    ArraySizeMismatch {
        size: usize,
        child_size: usize,
    },
    CapacityError(CapacityError),
    #[cfg(feature = "alloc")]
    AllocError(AllocError),
}

impl From<CapacityError> for ErrorKind {
    #[inline]
    fn from(e: CapacityError) -> Self {
        ErrorKind::CapacityError(e)
    }
}

#[cfg(feature = "alloc")]
impl From<AllocError> for ErrorKind {
    #[inline]
    fn from(e: AllocError) -> Self {
        ErrorKind::AllocError(e)
    }
}

#[cfg(test)]
impl PartialEq<ErrorKind> for &ErrorKind {
    #[inline]
    fn eq(&self, other: &ErrorKind) -> bool {
        **self == *other
    }
}

impl core::error::Error for Error {}

impl fmt::Debug for Error {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.kind.fmt(f)
    }
}

impl fmt::Display for Error {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            ErrorKind::StructUnderflow => write!(f, "Struct underflow"),
            ErrorKind::ObjectUnderflow => write!(f, "Object underflow"),
            ErrorKind::SizeOverflow => write!(f, "Size overflow"),
            ErrorKind::BufferUnderflow => write!(f, "Buffer underflow"),
            ErrorKind::NonTerminatedString => write!(f, "Non-terminated c-string"),
            ErrorKind::NullContainingString => write!(
                f,
                "Tried to encode UTF-8 string containing an encoded null byte"
            ),
            ErrorKind::NotUtf8 => write!(f, "String does not contain valid UTF-8"),
            ErrorKind::NotSupportedRef => write!(f, "Decoding into reference is not supported"),
            ErrorKind::InvalidArrayLength => write!(f, "Invalid array length"),
            ErrorKind::UnsizedTypeInArray { ty } => write!(
                f,
                "Unsized type {ty:?} in array, use push_unsized_array instead"
            ),
            ErrorKind::Expected { expected, actual } => {
                write!(f, "Expected {expected:?}, but found {actual:?}")
            }
            ErrorKind::ReservedSizeMismatch { expected, actual } => {
                write!(
                    f,
                    "Expected reserved to write {expected} bytes, but found {actual}"
                )
            }
            ErrorKind::ReservedOverflow {
                write,
                len,
                capacity,
            } => {
                write!(
                    f,
                    "Write {len} bytes at {write} overflows dynamic capacity {capacity}"
                )
            }
            ErrorKind::ChildSizeMismatch { expected, actual } => {
                write!(
                    f,
                    "Expected array element size {expected}, but found {actual}"
                )
            }
            ErrorKind::InvalidUsize { value } => {
                write!(f, "Value {value} is a valid usize")
            }
            ErrorKind::InvalidIsize { value } => {
                write!(f, "Value {value} is a valid isize")
            }
            ErrorKind::InvalidUsizeInt { value } => {
                write!(f, "Value {value} is a valid int")
            }
            ErrorKind::InvalidIsizeInt { value } => {
                write!(f, "Value {value} is a valid int")
            }
            ErrorKind::ArraySizeMismatch { size, child_size } => {
                write!(f, "Array size {size} is not a multiple of {child_size}")
            }
            ErrorKind::CapacityError(ref e) => e.fmt(f),
            #[cfg(feature = "alloc")]
            ErrorKind::AllocError(ref e) => e.fmt(f),
        }
    }
}
