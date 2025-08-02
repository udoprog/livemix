use core::mem;

use std::collections::BTreeMap;

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use anyhow::{Result, bail};
use pod::AsSlice;
use pod::{ChoiceType, DynamicBuf, Object, Type};
use protocol::consts::{self, Direction};
use protocol::ffi;
use protocol::id::{
    self, AudioFormat, Format, MediaSubType, MediaType, ObjectType, Param, ParamBuffers, ParamIo,
    ParamMeta,
};
use tracing::Level;

use crate::{Buffers, Region};

#[derive(Debug)]
#[non_exhaustive]
pub struct PortParam<B = DynamicBuf>
where
    B: AsSlice,
{
    pub value: Object<B>,
    pub flags: u32,
}

impl<B> PortParam<B>
where
    B: AsSlice,
{
    /// Construct a port parameter with empty flags.
    #[inline]
    pub fn new(value: Object<B>) -> Self {
        Self { value, flags: 0 }
    }

    /// Construct a port parameter with associated flags.
    #[inline]
    pub fn with_flags(value: Object<B>, flags: u32) -> Self {
        Self { value, flags }
    }
}

/// The definition of a port.
#[derive(Debug)]
#[non_exhaustive]
pub struct Port {
    /// Identifier of the port, unique per direction.
    pub id: u32,
    /// The direction of the port.
    pub dir: Direction,
    /// The name of the port.
    pub name: String,
    /// Buffers associated with the port.
    pub buffers: Option<Buffers>,
    /// The IO clock region for the port.
    pub io_clock: Option<Region<ffi::IoClock>>,
    /// The IO position region for the port.
    pub io_position: Option<Region<ffi::IoPosition>>,
    /// The IO buffers region for the port.
    pub io_buffers: Option<Region<ffi::IoBuffers>>,
    modified: bool,
    params: BTreeMap<Param, Vec<PortParam<DynamicBuf>>>,
}

impl Port {
    /// Access the port id.
    #[inline]
    pub(crate) fn id(&self) -> u32 {
        self.id
    }

    /// Take the modified state of the port.
    #[inline]
    pub(crate) fn take_modified(&mut self) -> bool {
        mem::take(&mut self.modified)
    }

    /// Set a parameter on the port.
    #[inline]
    pub fn set_param(
        &mut self,
        id: Param,
        values: impl IntoIterator<Item = PortParam<impl AsSlice>, IntoIter: ExactSizeIterator>,
    ) -> Result<()> {
        let mut iter = values.into_iter();
        let mut params = Vec::with_capacity(iter.len());

        for param in iter {
            params.push(PortParam::with_flags(
                param.value.as_ref().to_owned()?,
                param.flags,
            ));
        }

        self.params.insert(id, params);
        self.modified = true;
        Ok(())
    }

    /// Set a parameter on the port.
    #[inline]
    pub fn add_param(&mut self, id: Param, value: PortParam<impl AsSlice>) -> Result<()> {
        self.params
            .entry(id)
            .or_default()
            .push(PortParam::with_flags(
                value.value.as_ref().to_owned()?,
                value.flags,
            ));
        self.modified = true;
        Ok(())
    }

    /// Remove a parameter from the port and return the values of the removed
    /// parameter if it exists.
    #[inline]
    pub fn remove_param(&mut self, id: Param) -> Option<Vec<PortParam>> {
        let param = self.params.remove(&id)?;
        self.modified = true;
        Some(param)
    }

    /// Get parameters from the port.
    pub(crate) fn params(&self) -> &BTreeMap<Param, Vec<PortParam<impl AsSlice>>> {
        &self.params
    }

    /// Replace the current set of buffers for this port.
    #[inline]
    #[tracing::instrument(skip(self), fields(port_id = self.id), ret(level = Level::DEBUG))]
    pub(crate) fn replace_buffers(&mut self, buffers: Buffers) -> Option<Buffers> {
        self.buffers.replace(buffers)
    }
}

#[derive(Default, Debug)]
pub struct Ports {
    input_ports: Vec<Port>,
    output_ports: Vec<Port>,
}

impl Ports {
    /// Construct a new collection of ports.
    #[inline]
    pub(crate) fn new() -> Self {
        Self {
            input_ports: Vec::new(),
            output_ports: Vec::new(),
        }
    }

    /// Access input ports.
    pub fn inputs(&self) -> &[Port] {
        &self.input_ports
    }

    /// Access input ports mutably.
    pub fn inputs_mut(&mut self) -> &mut [Port] {
        &mut self.input_ports
    }

    /// Access output ports.
    pub fn outputs(&self) -> &[Port] {
        &self.output_ports
    }

    /// Access output ports mutably.
    pub fn outputs_mut(&mut self) -> &mut [Port] {
        &mut self.output_ports
    }

    /// Insert a new port in the specified direction and return the inserted
    /// port for configuration.
    pub fn insert(&mut self, dir: Direction) -> Result<&mut Port> {
        let ports = self.get_direction_mut(dir)?;

        let id = ports.len() as u32;

        let mut port = Port {
            id,
            dir,
            modified: true,
            name: String::new(),
            buffers: None,
            io_clock: None,
            io_position: None,
            io_buffers: None,
            params: BTreeMap::new(),
        };

        ports.push(port);
        Ok(&mut ports[id as usize])
    }

    /// Get a port.
    fn get(&self, direction: Direction, id: u32) -> Result<&Port> {
        let Ok(id) = usize::try_from(id) else {
            bail!("Invalid port id: {id}");
        };

        let ports = self.get_direction(direction)?;

        let Some(port) = ports.get(id) else {
            bail!("Port {id} not found in {direction:?} ports");
        };

        Ok(port)
    }

    /// Get a port mutably.
    pub(crate) fn get_mut(&mut self, direction: Direction, id: u32) -> Result<&mut Port> {
        let Ok(id) = usize::try_from(id) else {
            bail!("Invalid port id: {id}");
        };

        let ports = self.get_direction_mut(direction)?;

        let Some(port) = ports.get_mut(id) else {
            bail!("Port {id} not found in {direction:?} ports");
        };

        Ok(port)
    }

    #[inline]
    fn get_direction(&self, dir: Direction) -> Result<&Vec<Port>> {
        match dir {
            Direction::INPUT => Ok(&self.input_ports),
            Direction::OUTPUT => Ok(&self.output_ports),
            dir => panic!("Unknown port direction: {dir:?}"),
        }
    }

    #[inline]
    fn get_direction_mut(&mut self, dir: Direction) -> Result<&mut Vec<Port>> {
        match dir {
            Direction::INPUT => Ok(&mut self.input_ports),
            Direction::OUTPUT => Ok(&mut self.output_ports),
            dir => panic!("Unknown port direction: {dir:?}"),
        }
    }
}
