use core::mem;

use std::collections::BTreeMap;
use std::vec::Vec;

use anyhow::{Result, bail};
use pod::{AsSlice, DynamicBuf, Object};
use protocol::id::Param;
use protocol::poll::Token;
use protocol::{EventFd, ffi};
use slab::Slab;

use crate::memory::Region;
use crate::{Activation, Ports};

/// A client node.
#[non_exhaustive]
pub struct ClientNode {
    /// The unique identifier for this node.
    pub id: u32,
    /// Activation record for this node.
    pub activation: Option<Region<ffi::NodeActivation>>,
    /// Activation records for dependent nodes.
    pub peer_activations: Slab<Activation>,
    /// Ports associated with the client node.
    pub ports: Ports,
    pub(super) read_fd: Option<EventFd>,
    pub(super) write_token: Token,
    pub(super) write_fd: Option<EventFd>,
    pub(super) read_token: Token,
    pub(super) params: BTreeMap<Param, Vec<Object<DynamicBuf>>>,
    pub(super) io_clock: Option<Region<ffi::IoClock>>,
    pub(super) io_control: Option<Region<[u8]>>,
    pub(super) io_position: Option<Region<ffi::IoPosition>>,
    pub(super) modified: bool,
}

impl ClientNode {
    pub(crate) fn new(
        id: u32,
        ports: Ports,
        write_token: Token,
        read_token: Token,
    ) -> Result<Self> {
        let mut params = BTreeMap::new();

        Ok(Self {
            id,
            ports,
            write_fd: None,
            read_fd: None,
            write_token,
            read_token,
            activation: None,
            peer_activations: Slab::new(),
            params,
            io_control: None,
            io_clock: None,
            io_position: None,
            modified: true,
        })
    }

    /// Set a parameter for the node.
    #[inline]
    pub fn set_param(
        &mut self,
        param: Param,
        values: impl IntoIterator<Item = Object<impl AsSlice>, IntoIter: ExactSizeIterator>,
    ) -> Result<()> {
        let mut iter = values.into_iter();
        let mut params = Vec::with_capacity(iter.len());

        for pod in iter {
            params.push(pod.as_ref().to_owned()?);
        }

        self.params.insert(param, params);
        self.modified = true;
        Ok(())
    }

    /// Remove a parameter for the node.
    #[inline]
    pub fn remove_param(&mut self, param: Param) {
        self.params.remove(&param);
        self.modified = true;
    }

    /// Take and return the modified state of the node.
    #[inline]
    pub(super) fn take_modified(&mut self) -> bool {
        mem::take(&mut self.modified)
    }
}
