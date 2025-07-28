use core::fmt;

use crate::Type;

#[non_exhaustive]
pub struct Error {
    kind: ErrorKind,
}

impl Error {
    /// Create a new `Error` with the specified kind.
    #[inline]
    pub(crate) fn new(kind: ErrorKind) -> Self {
        Self { kind }
    }

    /// Get the kind of error.
    #[inline]
    #[cfg(all(test, feature = "alloc"))]
    pub(crate) fn kind(&self) -> &ErrorKind {
        &self.kind
    }
}

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub(crate) enum ErrorKind {
    StructUnderflow,
    ObjectUnderflow,
    SizeOverflow,
    SizeUnderflow { size: usize, sub: usize },
    BufferOverflow,
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
            ErrorKind::BufferOverflow => write!(f, "Buffer overflow"),
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
        }
    }
}
