use core::fmt;

use super::ty::Type;

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
    SizeOverflow,
    BufferOverflow,
    BufferUnderflow,
    NonTerminatedString,
    NullContainingString,
    NotUtf8,
    NotSupportedRef,
    Expected { expected: Type, actual: Type },
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
            ErrorKind::SizeOverflow => write!(f, "Size overflow"),
            ErrorKind::BufferOverflow => write!(f, "Buffer overflow"),
            ErrorKind::BufferUnderflow => write!(f, "Buffer underflow"),
            ErrorKind::NonTerminatedString => write!(f, "Non-terminated c-string"),
            ErrorKind::NullContainingString => write!(
                f,
                "Tried to encode UTF-8 string containing an encoded null byte"
            ),
            ErrorKind::NotUtf8 => write!(f, "String does not contain valid UTF-8"),
            ErrorKind::NotSupportedRef => write!(f, "Decoding into reference is not supported"),
            ErrorKind::Expected { expected, actual } => {
                write!(f, "Expected {expected:?}, but found {actual:?}")
            }
        }
    }
}
