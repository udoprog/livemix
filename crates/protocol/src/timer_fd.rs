use core::ptr;
use core::time::Duration;
use std::io;
use std::mem;
use std::os::unix::io::{AsRawFd, FromRawFd, OwnedFd, RawFd};

/// Event file descriptor.
pub struct TimerFd {
    fd: OwnedFd,
}

impl TimerFd {
    /// Construct a new timer fd.
    pub fn new() -> io::Result<Self> {
        // SAFETY: We're just using c-apis as intended.
        unsafe {
            let fd = libc::timerfd_create(libc::CLOCK_MONOTONIC, 0);

            if fd == -1 {
                return Err(io::Error::last_os_error());
            }

            Ok(Self {
                fd: OwnedFd::from_raw_fd(fd),
            })
        }
    }

    pub fn set_nonblocking(&self, nonblocking: bool) -> io::Result<()> {
        // SAFETY: We're just using c-apis as intended.
        unsafe {
            let mut flags = libc::fcntl(self.fd.as_raw_fd(), libc::F_GETFL);

            if flags == -1 {
                return Err(io::Error::last_os_error());
            }

            if nonblocking {
                flags |= libc::O_NONBLOCK;
            } else {
                flags &= !libc::O_NONBLOCK;
            }

            if libc::fcntl(self.fd.as_raw_fd(), libc::F_SETFL, flags) == -1 {
                return Err(io::Error::last_os_error());
            }

            Ok(())
        }
    }

    /// Set a single timeout.
    pub fn set_timeout(&self, duration: Duration) -> io::Result<()> {
        // SAFETY: We're just using c-apis as intended.
        unsafe {
            let mut value: libc::itimerspec = mem::zeroed();
            value.it_value.tv_sec = duration.as_secs() as _;
            value.it_value.tv_nsec = duration.subsec_nanos() as _;

            let n = libc::timerfd_settime(self.fd.as_raw_fd(), 0, &value, ptr::null_mut());

            if n == -1 {
                return Err(io::Error::last_os_error());
            }

            Ok(())
        }
    }

    /// Set an interval timer.
    pub fn set_interval(&self, duration: Duration) -> io::Result<()> {
        // SAFETY: We're just using c-apis as intended.
        unsafe {
            let mut value: libc::itimerspec = mem::zeroed();
            value.it_value.tv_sec = duration.as_secs() as _;
            value.it_value.tv_nsec = duration.subsec_nanos() as _;

            value.it_interval.tv_sec = duration.as_secs() as _;
            value.it_interval.tv_nsec = duration.subsec_nanos() as _;

            let n = libc::timerfd_settime(self.fd.as_raw_fd(), 0, &value, ptr::null_mut());

            if n == -1 {
                return Err(io::Error::last_os_error());
            }

            Ok(())
        }
    }

    /// Read the number of expirations that have occured.
    ///
    /// Returns `None` if the operation would block.
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

impl AsRawFd for TimerFd {
    #[inline]
    fn as_raw_fd(&self) -> RawFd {
        self.fd.as_raw_fd()
    }
}
