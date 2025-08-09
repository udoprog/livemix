//! Various utility functions for working with pipewire clients.

use std::io;
use std::os::fd::RawFd;

/// Get the current monotonic time in nanoseconds.
pub fn get_monotonic_nsec() -> io::Result<u64> {
    const NSEC_PER_SEC: u64 = 1_000_000_000u64;

    let mut time_spec = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };

    // SAFETY: We're just using c-apis as intended.
    unsafe {
        if libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut time_spec) == -1 {
            return Err(io::Error::last_os_error());
        }
    }

    Ok((time_spec.tv_sec as u64)
        .saturating_mul(NSEC_PER_SEC)
        .saturating_add(time_spec.tv_nsec as u64))
}

/// Test if a given file descriptor is non-blocking.
pub fn is_nonblocking(fd: RawFd) -> io::Result<bool> {
    // SAFETY: We're just using c-apis as intended.
    unsafe {
        let flags = libc::fcntl(fd, libc::F_GETFL);

        if flags == -1 {
            return Err(io::Error::last_os_error());
        }

        Ok(flags & libc::O_NONBLOCK != 0)
    }
}
