use core::mem;
use std::io;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};

use libc::{
    EPOLL_CTL_ADD, EPOLL_CTL_DEL, EPOLL_CTL_MOD, epoll_create1, epoll_ctl, epoll_event, epoll_wait,
};
use tracing::Level;

use crate::events::Events;
use crate::poll::{Interest, PollEvent, Token};

/// A poll structure.
pub struct Poll {
    fd: OwnedFd,
}

impl Poll {
    /// Construct a new poll wrapper.
    pub fn new() -> io::Result<Self> {
        unsafe {
            let fd = epoll_create1(0);

            if fd == -1 {
                return Err(io::Error::last_os_error());
            }

            Ok(Self {
                fd: OwnedFd::from_raw_fd(fd),
            })
        }
    }

    /// Add interest for a file descriptor.
    #[tracing::instrument(skip(self), ret(level = Level::TRACE))]
    pub fn add(&mut self, fd: RawFd, token: Token, interest: Interest) -> io::Result<()> {
        // SAFETY: We're just using c-apis as intended.
        unsafe {
            let mut ev: epoll_event = mem::zeroed();

            ev.events = interest.as_u32();
            ev.u64 = token.0 as u64;

            if epoll_ctl(self.fd.as_raw_fd(), EPOLL_CTL_ADD, fd.as_raw_fd(), &mut ev) == -1 {
                return Err(io::Error::last_os_error());
            }

            Ok(())
        }
    }

    /// Modify interest for the given file descriptor.
    #[tracing::instrument(skip(self), ret(level = Level::TRACE))]
    pub fn modify(&mut self, fd: RawFd, token: Token, interest: Interest) -> io::Result<()> {
        // SAFETY: We're just using c-apis as intended.
        unsafe {
            let mut ev: epoll_event = mem::zeroed();

            ev.events = interest.as_u32();
            ev.u64 = token.0 as u64;

            if epoll_ctl(self.fd.as_raw_fd(), EPOLL_CTL_MOD, fd.as_raw_fd(), &mut ev) == -1 {
                return Err(io::Error::last_os_error());
            }

            Ok(())
        }
    }

    /// Delete interest for the given file descriptor.
    #[tracing::instrument(skip(self), ret(level = Level::TRACE))]
    pub fn delete(&mut self, fd: RawFd, token: Token, interest: Interest) -> io::Result<()> {
        // SAFETY: We're just using c-apis as intended.
        unsafe {
            let mut ev: epoll_event = mem::zeroed();

            ev.events = interest.as_u32();
            ev.u64 = token.0;

            if epoll_ctl(self.fd.as_raw_fd(), EPOLL_CTL_DEL, fd.as_raw_fd(), &mut ev) == -1 {
                return Err(io::Error::last_os_error());
            }

            Ok(())
        }
    }

    /// Poll for the next events.
    pub fn poll(&mut self, out: &mut impl Events<PollEvent>) -> io::Result<()> {
        // SAFETY: We're ensuring safety through type invariants.
        unsafe {
            let mut events = [mem::zeroed(); 4];
            let len = events.len().min(out.remaining_mut());
            let ready = epoll_wait(self.fd.as_raw_fd(), events.as_mut_ptr(), len as i32, -1);

            if ready == -1 {
                return Err(io::Error::last_os_error());
            }

            for e in events.get(..ready as usize).unwrap_or_default() {
                out.push(PollEvent {
                    token: Token(e.u64),
                    interest: Interest(e.events),
                });
            }

            Ok(())
        }
    }
}

impl AsRawFd for Poll {
    #[inline]
    fn as_raw_fd(&self) -> RawFd {
        self.fd.as_raw_fd()
    }
}
