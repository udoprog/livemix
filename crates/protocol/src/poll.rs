mod linux;
pub use self::linux::Poll;

use core::ops::BitOrAssign;
use core::{mem, ops::BitOr};
use std::fmt;

use libc::{POLLERR, POLLHUP, POLLIN, POLLOUT};

/// The token returned by a poller.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Token(u64);

impl Token {
    /// Construct a new token with the given value.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
}

/// An update to an interest.
#[derive(Debug)]
#[must_use = "Not applying an interest update might lead to the process being stalled"]
pub enum ChangeInterest {
    /// The interest has changed.
    Changed(Interest),
    /// The interest has not changed.
    Unchanged,
}

impl ChangeInterest {
    /// Take polled outcome and replace with unchanged.
    #[inline]
    pub fn take(&mut self) -> ChangeInterest {
        mem::replace(self, ChangeInterest::Unchanged)
    }
}

impl BitOrAssign for ChangeInterest {
    #[inline]
    fn bitor_assign(&mut self, rhs: Self) {
        if matches!((&*self, &rhs), (_, ChangeInterest::Changed(..))) {
            *self = rhs;
        }
    }
}

/// An output poll event.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct PollEvent {
    pub token: Token,
    pub interest: Interest,
}

/// Collection of events.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Interest(u32);

impl Interest {
    /// Empty interest.
    pub const EMPTY: Self = Self::new();
    /// Read only interest.
    pub const READ: Self = Self::new().read();
    /// Write only interest.
    pub const WRITE: Self = Self::new().write();
    /// HUP interest.
    pub const HUP: Self = Self::new().hup();
    /// Error interest.
    pub const ERROR: Self = Self::new().error();

    /// Construct a new ready set.
    const fn new() -> Self {
        Self(0)
    }

    /// Set the specified interest.
    ///
    /// Returns `true` if the interest was modified.
    #[inline]
    pub fn set(&mut self, interest: Interest) -> ChangeInterest {
        if self.0 & interest.0 != 0 {
            return ChangeInterest::Unchanged;
        }

        self.0 |= interest.0;
        ChangeInterest::Changed(*self)
    }

    /// Unset the specified interest.
    ///
    /// Returns `true` if the interest was modified.
    #[inline]
    pub fn unset(&mut self, interest: Interest) -> ChangeInterest {
        if self.0 & interest.0 == 0 {
            return ChangeInterest::Unchanged;
        }

        self.0 &= !interest.0;
        ChangeInterest::Changed(*self)
    }

    /// Make a ready set with read interest.
    #[inline]
    const fn read(self) -> Self {
        Self(self.0 | POLLIN as u32)
    }

    /// Make a ready set with write interest.
    #[inline]
    const fn write(self) -> Self {
        Self(self.0 | POLLOUT as u32)
    }

    /// Make a ready set with hup interest.
    #[inline]
    const fn hup(self) -> Self {
        Self(self.0 | POLLHUP as u32)
    }

    /// Make a ready set with error interest.
    #[inline]
    const fn error(self) -> Self {
        Self(self.0 | POLLERR as u32)
    }

    /// If events are read ready.
    #[inline]
    pub const fn is_read(&self) -> bool {
        self.0 & (POLLIN as u32) != 0
    }

    /// If events is write ready.
    #[inline]
    pub const fn is_write(&self) -> bool {
        self.0 & (POLLOUT as u32) != 0
    }

    /// If events are hup ready.
    #[inline]
    pub const fn is_hup(&self) -> bool {
        self.0 & (POLLHUP as u32) != 0
    }

    /// If events is error ready.
    #[inline]
    pub const fn is_error(&self) -> bool {
        self.0 & (POLLERR as u32) != 0
    }

    /// As raw underlying u32.
    ///
    /// Note that since this is all based on constrained constant values we know
    /// that this is a valid conversion.
    #[inline]
    const fn as_u32(&self) -> u32 {
        self.0
    }
}

impl BitOr for Interest {
    type Output = Self;

    #[inline]
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl fmt::Debug for Interest {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut f = f.debug_tuple("Interest");

        if self.0 & POLLIN as u32 != 0 {
            f.field(&DebugString::new("POLLIN"));
        }

        if self.0 & POLLOUT as u32 != 0 {
            f.field(&DebugString::new("POLLOUT"));
        }

        if self.0 & POLLHUP as u32 != 0 {
            f.field(&DebugString::new("POLLHUP"));
        }

        if self.0 & POLLERR as u32 != 0 {
            f.field(&DebugString::new("POLLERR"));
        }

        return f.finish();

        #[repr(transparent)]
        struct DebugString(str);

        impl DebugString {
            #[inline]
            fn new(s: &str) -> &Self {
                // SAFETY: DebugString is repr(transparent) over str.
                unsafe { &*(s as *const str as *const Self) }
            }
        }

        impl fmt::Debug for DebugString {
            #[inline]
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    }
}
