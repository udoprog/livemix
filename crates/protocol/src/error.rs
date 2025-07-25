use core::error;
use core::fmt;

#[cfg(feature = "std")]
use std::io;

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
}

impl From<ErrorKind> for Error {
    #[inline]
    fn from(kind: ErrorKind) -> Self {
        Error::new(kind)
    }
}

impl From<pod::Error> for Error {
    #[inline]
    fn from(e: pod::Error) -> Self {
        Error::new(ErrorKind::PodError(e))
    }
}

#[derive(Debug)]
pub(crate) enum ErrorKind {
    PodError(pod::Error),
    #[cfg(feature = "std")]
    ConnectionFailed(io::Error),
    #[cfg(feature = "std")]
    SendError(io::Error),
    #[cfg(feature = "std")]
    RecvError(io::Error),
    NoSocket,
    SizeOverflow,
}

impl error::Error for Error {
    #[inline]
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match &self.kind {
            ErrorKind::PodError(e) => Some(e),
            #[cfg(feature = "std")]
            ErrorKind::ConnectionFailed(e) => Some(e),
            #[cfg(feature = "std")]
            ErrorKind::SendError(e) => Some(e),
            #[cfg(feature = "std")]
            ErrorKind::RecvError(e) => Some(e),
            _ => None,
        }
    }
}

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
            ErrorKind::PodError(..) => write!(f, "Pod encoding error"),
            #[cfg(feature = "std")]
            ErrorKind::ConnectionFailed(..) => write!(f, "Connection failed"),
            #[cfg(feature = "std")]
            ErrorKind::SendError(..) => write!(f, "Send error"),
            #[cfg(feature = "std")]
            ErrorKind::RecvError(..) => write!(f, "Receive error"),
            ErrorKind::NoSocket => write!(f, "No socket to connect to found"),
            ErrorKind::SizeOverflow => write!(f, "Size overflow"),
        }
    }
}
