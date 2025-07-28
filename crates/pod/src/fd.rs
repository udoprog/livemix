use core::fmt;

/// A pointer stored in a pod.
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(C, align(8))]
pub struct Fd {
    fd: i64,
}

impl Fd {
    /// Construct a new file descriptor.
    #[inline]
    pub const fn new(fd: i64) -> Self {
        Self { fd }
    }

    /// Returns the file descriptor.
    #[inline]
    pub const fn fd(&self) -> i64 {
        self.fd
    }
}

impl fmt::Debug for Fd {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Fd").field(&self.fd).finish()
    }
}
