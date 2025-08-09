use core::fmt;

#[cfg(feature = "alloc")]
use crate::buf::AllocError;
use crate::buf::CapacityError;
use crate::{ChoiceType, RawId, Type};

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

    #[inline]
    pub fn expected(expected: Type, actual: Type, size: usize) -> Self {
        Self::new(ErrorKind::Expected {
            expected,
            actual,
            size,
        })
    }

    #[doc(hidden)]
    pub fn __invalid_object_type(expected: impl RawId, actual: impl RawId) -> Self {
        Self::new(ErrorKind::InvalidObjectType {
            expected: expected.into_id(),
            actual: actual.into_id(),
        })
    }

    #[doc(hidden)]
    pub fn __invalid_object_id(expected: impl RawId, actual: impl RawId) -> Self {
        Self::new(ErrorKind::InvalidObjectId {
            expected: expected.into_id(),
            actual: actual.into_id(),
        })
    }

    #[doc(hidden)]
    pub fn __missing_object_field(name: &'static str) -> Self {
        Self::new(ErrorKind::MissingObjectField { name })
    }

    #[doc(hidden)]
    pub fn __missing_object_index(index: usize) -> Self {
        Self::new(ErrorKind::MissingObjectIndex { index })
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

#[non_exhaustive]
pub(crate) struct BufferUnderflow;

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
        size: usize,
    },
    ExpectedNumber {
        actual: Type,
        size: usize,
    },
    ExpectedSize {
        ty: Type,
        expected: usize,
        actual: usize,
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
    InvalidInt {
        ty: &'static str,
        value: i32,
    },
    InvalidLong {
        ty: &'static str,
        value: i64,
    },
    InvalidUsizeInt {
        ty: Type,
        value: usize,
    },
    InvalidIsizeInt {
        ty: Type,
        value: isize,
    },
    ArraySizeMismatch {
        size: usize,
        child_size: usize,
    },
    InvalidObjectType {
        expected: u32,
        actual: u32,
    },
    InvalidObjectId {
        expected: u32,
        actual: u32,
    },
    MissingObjectField {
        name: &'static str,
    },
    MissingObjectIndex {
        index: usize,
    },
    InvalidChoiceType {
        ty: Type,
        expected: ChoiceType,
        actual: ChoiceType,
    },
    ReadNotSupported {
        ty: Type,
    },
    ReadSizedNotSupported {
        ty: Type,
    },
    ReadUnsizedNotSupported {
        ty: Type,
    },
    CapacityError(CapacityError),
    #[cfg(feature = "alloc")]
    AllocError(AllocError),
}

impl From<BufferUnderflow> for ErrorKind {
    #[inline]
    fn from(_: BufferUnderflow) -> Self {
        ErrorKind::BufferUnderflow
    }
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
                "Unsized type {ty:?} in array, use write_unsized_array instead"
            ),
            ErrorKind::Expected {
                expected,
                actual,
                size,
            } => {
                write!(
                    f,
                    "Expected {expected:?}, but found {actual:?} with size {size}"
                )
            }
            ErrorKind::ExpectedNumber { actual, size } => {
                write!(
                    f,
                    "Expected a number type, but found {actual:?} with size {size}"
                )
            }
            ErrorKind::ExpectedSize {
                ty,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "Expected size {expected} for type {ty:?}, but found {actual}"
                )
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
            ErrorKind::InvalidInt { value, ty } => {
                write!(f, "Int value {value} is not a valid {ty}")
            }
            ErrorKind::InvalidLong { value, ty } => {
                write!(f, "Long value {value} is not a valid {ty}")
            }
            ErrorKind::InvalidUsizeInt { ty, value } => {
                write!(f, "The usize value {value} is not a valid {ty}")
            }
            ErrorKind::InvalidIsizeInt { ty, value } => {
                write!(f, "The isize value {value} is not a valid {ty}")
            }
            ErrorKind::ArraySizeMismatch { size, child_size } => {
                write!(f, "Array size {size} is not a multiple of {child_size}")
            }
            ErrorKind::InvalidObjectType { expected, actual } => {
                write!(f, "Expected object type {expected}, but found {actual}")
            }
            ErrorKind::InvalidObjectId { expected, actual } => {
                write!(f, "Expected object id {expected}, but found {actual}")
            }
            ErrorKind::MissingObjectField { name } => {
                write!(f, "Missing object field `{name}`")
            }
            ErrorKind::MissingObjectIndex { index } => {
                write!(f, "Missing object index {index}")
            }
            ErrorKind::InvalidChoiceType {
                ty,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "While decoding type {ty:?}, expected choice type {expected:?}, but found {actual:?}"
                )
            }
            ErrorKind::ReadNotSupported { ty } => {
                write!(f, "Item reading not supported for type {ty:?}")
            }
            ErrorKind::ReadSizedNotSupported { ty } => {
                write!(f, "Item sized reading not supported for type {ty:?}")
            }
            ErrorKind::ReadUnsizedNotSupported { ty } => {
                write!(f, "Item unsized reading not supported for type {ty:?}")
            }
            ErrorKind::CapacityError(ref e) => e.fmt(f),
            #[cfg(feature = "alloc")]
            ErrorKind::AllocError(ref e) => e.fmt(f),
        }
    }
}
