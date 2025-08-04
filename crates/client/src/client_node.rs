use core::fmt;
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

/// Collection of data related to client nodes.
pub struct ClientNodes {
    data: Slab<ClientNode>,
}

impl ClientNodes {
    /// Create a new `ClientNodes` instance.
    #[inline]
    pub fn new() -> Self {
        Self { data: Slab::new() }
    }

    /// Insert a new client node into the collection.
    pub(crate) fn insert(&mut self, node: ClientNode) -> ClientNodeId {
        let id = self.data.insert(node);
        ClientNodeId::new(id as u32)
    }

    /// Remove a client node from the collection by its identifier.
    pub(crate) fn remove(&mut self, id: ClientNodeId) -> Option<ClientNode> {
        self.data.try_remove(id.index())
    }

    /// Get a reference to the client node with the given ID.
    #[inline]
    pub fn get(&self, id: ClientNodeId) -> Option<&ClientNode> {
        self.data.get(id.index())
    }

    /// Get a mutable reference to the client node with the given ID.
    #[inline]
    pub fn get_mut(&mut self, id: ClientNodeId) -> Option<&mut ClientNode> {
        self.data.get_mut(id.index())
    }
}

/// A client node identifier.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ClientNodeId(u32);

impl ClientNodeId {
    /// Create a new `ClientNodeId` from a `u32`.
    #[inline]
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    /// Convert the `ClientNodeId` into a `u32`.
    #[inline]
    pub(crate) fn into_u32(self) -> u32 {
        self.0
    }

    /// Get the index of the client node.
    ///
    /// Since it was constructed from a `u32`, it can always be safely coerced
    /// into one.
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

impl fmt::Display for ClientNodeId {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl fmt::Debug for ClientNodeId {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

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
