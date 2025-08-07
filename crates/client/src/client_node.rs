use core::fmt;
use core::mem;

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::vec::Vec;

use anyhow::{Result, bail};
use pod::{AsSlice, DynamicBuf, Object};
use protocol::id::Param;
use protocol::poll::Token;
use protocol::{EventFd, ffi};
use slab::Slab;

use crate::activation;
use crate::memory::Region;
use crate::ptr::volatile;
use crate::{PeerActivation, Ports};

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

    /// Iterate over all client nodes.
    pub(crate) fn iter(&mut self) -> impl Iterator<Item = &ClientNode> {
        self.data.iter().map(|(_, node)| node)
    }

    /// Iterate over all client nodes mutably.
    pub(crate) fn iter_mut(&mut self) -> impl Iterator<Item = &mut ClientNode> {
        self.data.iter_mut().map(|(_, node)| node)
    }

    /// Get a reference to the client node with the given ID.
    #[inline]
    pub fn get(&self, id: ClientNodeId) -> Result<&ClientNode> {
        let Some(node) = self.data.get(id.index()) else {
            bail!("No client node found for id {}", id);
        };

        Ok(node)
    }

    /// Get a mutable reference to the client node with the given ID.
    #[inline]
    pub fn get_mut(&mut self, id: ClientNodeId) -> Result<&mut ClientNode> {
        let Some(node) = self.data.get_mut(id.index()) else {
            bail!("No client node found for id {}", id);
        };

        Ok(node)
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
    pub peer_activations: Vec<PeerActivation>,
    /// Ports associated with the client node.
    pub ports: Ports,
    pub read_fd: Option<EventFd>,
    pub(super) read_token: Token,
    pub write_fd: Option<EventFd>,
    pub(super) write_token: Token,
    pub(super) params: BTreeMap<Param, Vec<Object<DynamicBuf>>>,
    pub io_clock: Option<Region<ffi::IoClock>>,
    pub io_control: Option<Region<[u8]>>,
    pub io_position: Option<Region<ffi::IoPosition>>,
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
            peer_activations: Vec::new(),
            params,
            io_control: None,
            io_clock: None,
            io_position: None,
            modified: true,
        })
    }

    /// Replace the activation area for this node.
    #[inline]
    pub(crate) fn take_activation(&mut self) -> Option<Region<ffi::NodeActivation>> {
        let old = self.activation.take();
        self.update_activation_record();
        old
    }

    /// Replace the activation area for this node.
    #[inline]
    pub(crate) fn replace_activation(
        &mut self,
        activation: Region<ffi::NodeActivation>,
    ) -> Option<Region<ffi::NodeActivation>> {
        let old = self.activation.replace(activation);
        self.update_activation_record();
        old
    }

    /// Take the IO position for this node.
    pub(crate) fn take_io_position(&mut self) -> Option<Region<ffi::IoPosition>> {
        let old = self.io_position.take();
        self.update_activation_record();
        old
    }

    /// Replace the activation area for this node.
    #[inline]
    pub(crate) fn replace_io_position(
        &mut self,
        io_position: Region<ffi::IoPosition>,
    ) -> Option<Region<ffi::IoPosition>> {
        let old = self.io_position.replace(io_position);
        self.update_activation_record();
        old
    }

    /// It is important to update some fields in the activation area when the node is in a certain state.
    fn update_activation_record(&mut self) {
        // NB: Do nothing if there is no activation area.
        let Some(a) = &mut self.activation else {
            return;
        };

        let active_driver_id = unsafe { volatile!(a, active_driver_id) };

        let Some(io_position) = &mut self.io_position else {
            // NB: This is equivalent to SPA_ID_INVALID.
            active_driver_id.write(u32::MAX);
            return;
        };

        let id = unsafe { volatile!(io_position, clock.id).read() };
        active_driver_id.write(id);
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
