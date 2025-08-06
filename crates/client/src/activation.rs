use core::time::Duration;

use anyhow::Result;
use protocol::EventFd;
use protocol::consts::Activation;
use protocol::ffi;
use tracing::Level;

use crate::memory::Region;
use crate::ptr::{self, atomic, volatile};
use crate::utils;

/// The version of the activation protocol to use.
#[derive(Debug)]
pub enum Version {
    V0,
    V1,
}

#[derive(Debug)]
pub struct PeerActivation {
    pub peer_id: u32,
    pub signal_fd: EventFd,
    pub region: Region<ffi::NodeActivation>,
    pub version: Version,
}

impl PeerActivation {
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
    pub unsafe fn trigger(&self) -> Result<bool> {
        let nsec = utils::get_monotonic_nsec();

        let signaled = match self.version {
            Version::V0 => unsafe { self.signal_v0(nsec)? },
            Version::V1 => unsafe { self.signal_v1(nsec)? },
        };

        Ok(signaled)
    }

    // Port of `trigger_link_v0`.
    pub unsafe fn signal_v0(&self, nsec: u64) -> Result<bool> {
        unsafe {
            if !self.decrement_pending() {
                return Ok(false);
            }

            atomic!(self.region, status).store(Activation::TRIGGERED);
            volatile!(self.region, signal_time).write(nsec);

            if !self.signal_fd.write(1)? {
                tracing::error!("Failed to signal activation");
            }

            Ok(true)
        }
    }

    // Port of `trigger_link_v1`.
    pub unsafe fn signal_v1(&self, nsec: u64) -> Result<bool> {
        unsafe {
            if !self.decrement_pending() {
                return Ok(false);
            }

            let changed = atomic!(self.region, status)
                .compare_exchange(Activation::NOT_TRIGGERED, Activation::TRIGGERED);

            if !changed {
                return Ok(false);
            }

            volatile!(self.region, signal_time).write(nsec);

            if !self.signal_fd.write(1)? {
                tracing::error!("Failed to signal activation");
            }

            Ok(true)
        }
    }
    #[allow(unused)]
    unsafe fn decrement_pending(&self) -> bool {
        let value = unsafe { atomic!(self.region, state[0].pending).fetch_sub(1) };
        value == 1
    }
}
