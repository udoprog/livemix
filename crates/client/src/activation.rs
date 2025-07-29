use anyhow::Result;
use protocol::EventFd;
use protocol::consts::ActivationStatus;
use tracing::Level;

use crate::ffi;
use crate::memory::Region;
use crate::ptr;

/// The version of the activation protocol to use.
#[derive(Debug)]
pub enum Version {
    V0,
    V1,
}

#[derive(Debug)]
pub struct Activation {
    peer_id: u32,
    signal_fd: EventFd,
    pub region: Region<ffi::NodeActivation>,
    version: Version,
}

impl Activation {
    /// Construct a new activation record.
    #[inline]
    #[tracing::instrument(fields(self.version), ret(level = Level::TRACE))]
    pub fn new(peer_id: u32, signal_fd: EventFd, region: Region<ffi::NodeActivation>) -> Self {
        let server_version = ptr::volatile!(region, server_version).read();

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
    #[tracing::instrument(skip(self), fields(self.version), ret(level = Level::TRACE))]
    pub fn signal(&self) -> Result<()> {
        match self.version {
            Version::V0 => {
                self.signal_v0()?;
            }
            Version::V1 => {
                self.signal_v1()?;
            }
        }

        Ok(())
    }

    // Port of `trigger_link_v0`.
    pub fn signal_v0(&self) -> Result<()> {
        let pending = ptr::atomic!(self.region, state[0].pending).sub(1);
        tracing::trace!(?pending);

        if pending == 1 {
            ptr::atomic!(self.region, status).store(ActivationStatus::TRIGGERED);

            if !self.signal_fd.write(1)? {
                tracing::error!("Failed to signal activation");
            }
        }

        Ok(())
    }

    // Port of `trigger_link_v1`.
    pub fn signal_v1(&self) -> Result<()> {
        let pending = ptr::atomic!(self.region, state[0].pending).sub(1);
        tracing::trace!(?pending);

        if pending == 1 {
            if ptr::atomic!(self.region, status)
                .compare_exchange(ActivationStatus::NOT_TRIGGERED, ActivationStatus::TRIGGERED)
            {
                if !self.signal_fd.write(1)? {
                    tracing::error!("Failed to signal activation");
                }
            }
        }

        Ok(())
    }
}
