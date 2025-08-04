use anyhow::Result;
use protocol::EventFd;
use protocol::consts::ActivationStatus;
use protocol::ffi;
use tracing::Level;

use crate::memory::Region;
use crate::ptr::{self, atomic, volatile};

/// The version of the activation protocol to use.
#[derive(Debug)]
pub enum Version {
    V0,
    V1,
}

#[derive(Debug)]
pub struct Activation {
    pub peer_id: u32,
    pub signal_fd: EventFd,
    pub region: Region<ffi::NodeActivation>,
    pub version: Version,
}

impl Activation {
    /// Construct a new activation record.
    ///
    /// # Safety
    ///
    /// The caller is responsible for ensuring that the memory being accessed is
    /// a valid activation record.
    #[inline]
    #[tracing::instrument(fields(self.version), ret(level = Level::TRACE))]
    pub unsafe fn new(
        peer_id: u32,
        signal_fd: EventFd,
        region: Region<ffi::NodeActivation>,
    ) -> Self {
        let server_version = unsafe { volatile!(region, server_version).read() };

        let version = match server_version {
            0 => Version::V0,
            _ => Version::V1,
        };

        Self {
            peer_id,
            signal_fd,
            region,
            version,
        }
    }

    /// Signal the activation.
    ///
    /// # Safety
    ///
    /// The caller is responsible for ensuring that this is a valid activation record.
    pub unsafe fn signal(&self) -> Result<()> {
        let nsec = get_monotonic_nsec();

        match self.version {
            Version::V0 => unsafe {
                self.signal_v0(nsec)?;
            },
            Version::V1 => unsafe {
                self.signal_v1(nsec)?;
            },
        }

        Ok(())
    }

    // Port of `trigger_link_v0`.
    pub unsafe fn signal_v0(&self, nsec: u64) -> Result<()> {
        unsafe {
            let pending = atomic!(self.region, state[0].pending).fetch_sub(1);

            if pending == 1 {
                atomic!(self.region, status).store(ActivationStatus::TRIGGERED);
                volatile!(self.region, signal_time).write(nsec);

                if !self.signal_fd.write(1)? {
                    tracing::error!("Failed to signal activation");
                }
            }

            Ok(())
        }
    }

    // Port of `trigger_link_v1`.
    pub unsafe fn signal_v1(&self, nsec: u64) -> Result<()> {
        unsafe {
            let pending = atomic!(self.region, state[0].pending).fetch_sub(1);

            if pending == 1 {
                let changed = atomic!(self.region, status)
                    .compare_exchange(ActivationStatus::NOT_TRIGGERED, ActivationStatus::TRIGGERED);

                if changed {
                    volatile!(self.region, signal_time).write(nsec);

                    if !self.signal_fd.write(1)? {
                        tracing::error!("Failed to signal activation");
                    }
                }
            }

            Ok(())
        }
    }
}

fn get_monotonic_nsec() -> u64 {
    const NSEC_PER_SEC: u64 = 1000000000u64;

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
