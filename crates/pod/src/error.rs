use core::fmt;

use crate::Type;
use crate::array::CapacityError;

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

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub(crate) enum ErrorKind {
    StructUnderflow,
    ObjectUnderflow,
    SizeOverflow,
    SizeUnderflow { size: usize, sub: usize },
    BufferUnderflow,
    NonTerminatedString,
    NullContainingString,
    NotUtf8,
    NotSupportedRef,
    InvalidArrayLength,
    UnsizedTypeInArray { ty: Type },
    Expected { expected: Type, actual: Type },
    ReservedSizeMismatch { expected: usize, actual: usize },
    ChildSizeMismatch { expected: usize, actual: usize },
    InvalidUsize { value: i32 },
    InvalidIsize { value: i32 },
    CapacityError(CapacityError),
}

impl From<CapacityError> for ErrorKind {
    #[inline]
    fn from(e: CapacityError) -> Self {
        ErrorKind::CapacityError(e)
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
            ErrorKind::SizeUnderflow { size, sub } => {
                write!(f, "Size {size} underflowed when subtracting {sub}")
            }
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
            ErrorKind::CapacityError(ref e) => e.fmt(f),
        }
    }
}
