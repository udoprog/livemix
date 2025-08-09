use std::io;
use std::mem;
use std::os::unix::io::{AsRawFd, FromRawFd, OwnedFd, RawFd};

/// Event file descriptor.
#[derive(Debug)]
pub struct EventFd {
    fd: OwnedFd,
}

impl EventFd {
    /// Construct a new event fd.
    pub fn new(init: u32) -> io::Result<Self> {
        // SAFETY: We're just using c-apis as intended.
        unsafe {
            let fd = libc::eventfd(init, 0);

            if fd == -1 {
                return Err(io::Error::last_os_error());
            }

            Ok(Self {
                fd: OwnedFd::from_raw_fd(fd),
            })
        }
    }

    /// Write a value to the event.
    pub fn write(&self, n: u64) -> io::Result<bool> {
        // SAFETY: We're just using c-apis as intended.
        unsafe {
            let n = libc::write(self.fd.as_raw_fd(), &n as *const _ as *const _, 8);

            if n == -1 {
                return Err(io::Error::last_os_error());
            }

            Ok(n == 8)
        }
    }

    /// Receive a single event.
    ///
    /// Note that if an event is not available, this will block until one is
    /// sent.
    pub fn read(&self) -> io::Result<Option<u64>> {
        unsafe {
            let mut value = mem::MaybeUninit::<u64>::uninit();
            let n = libc::read(self.fd.as_raw_fd(), value.as_mut_ptr() as *mut _, 8);

            if n == -1 {
                match io::Error::last_os_error() {
                    e if e.kind() == io::ErrorKind::WouldBlock => return Ok(None),
                    e => return Err(e),
                }
            }

            if n != 8 {
                return Err(io::Error::other("expected 8 bytes"));
            }

            Ok(Some(value.assume_init()))
        }
    }
}

impl AsRawFd for EventFd {
    #[inline]
    fn as_raw_fd(&self) -> RawFd {
        self.fd.as_raw_fd()
    }
}

/// Coerce an `OwnedFd` into an `EventFd`.
impl From<OwnedFd> for EventFd {
    #[inline]
    fn from(fd: OwnedFd) -> Self {
        Self { fd }
    }
}
