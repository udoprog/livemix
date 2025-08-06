//! Various utility functions for working with pipewire clients.

/// Get the current monotonic time in nanoseconds.
pub fn get_monotonic_nsec() -> u64 {
    const NSEC_PER_SEC: u64 = 1_000_000_000u64;

    let mut time_spec = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };

    unsafe {
        libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut time_spec);
    }

    (time_spec.tv_sec as u64)
        .saturating_mul(NSEC_PER_SEC)
        .saturating_add(time_spec.tv_nsec as u64)
}
