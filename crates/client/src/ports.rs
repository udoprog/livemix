use alloc::string::String;
use alloc::vec::Vec;

use anyhow::{Result, bail};
use protocol::consts;
use tracing::Level;

use crate::{Buffers, Region};

#[derive(Debug)]
pub struct Port {
    id: u32,
    pub name: String,
    buffers: Option<Buffers>,
    io_clock: Option<Region>,
    io_position: Option<Region>,
    io_buffers: Option<Region>,
}

impl Port {
    /// Access the port id.
    #[inline]
    pub(crate) fn id(&self) -> u32 {
        self.id
    }

    /// Replace the current set of buffers for this port.
    #[inline]
    #[tracing::instrument(skip(self), fields(port_id = self.id), ret(level = Level::TRACE))]
    pub(crate) fn replace_buffers(&mut self, buffers: Buffers) -> Option<Buffers> {
        self.buffers.replace(buffers)
    }

    /// Get the io clock buffer mutably.
    #[inline]
    pub(crate) fn io_clock_mut(&mut self) -> &mut Option<Region> {
        &mut self.io_clock
    }

    /// Get the io position buffer mutably.
    #[inline]
    pub(crate) fn io_position_mut(&mut self) -> &mut Option<Region> {
        &mut self.io_position
    }

    /// Get the io io buffers mutably.
    #[inline]
    pub(crate) fn io_buffers_mut(&mut self) -> &mut Option<Region> {
        &mut self.io_buffers
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
    pub(crate) fn inputs(&self) -> &[Port] {
        &self.input_ports
    }

    /// Access output ports.
    pub(crate) fn outputs(&self) -> &[Port] {
        &self.output_ports
    }

    /// Insert a new port in the specified direction.
    pub(crate) fn insert(&mut self, direction: consts::Direction) -> Result<&mut Port> {
        let ports = self.get_direction_mut(direction)?;

        let id = ports.len() as u32;

        ports.push(Port {
            id,
            name: String::new(),
            buffers: None,
            io_clock: None,
            io_position: None,
            io_buffers: None,
        });

        Ok(&mut ports[id as usize])
    }

    /// Get a port.
    fn get(&self, direction: consts::Direction, id: u32) -> Result<&Port> {
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
    pub(crate) fn get_mut(&mut self, direction: consts::Direction, id: u32) -> Result<&mut Port> {
        let Ok(id) = usize::try_from(id) else {
            bail!("Invalid port id: {id}");
        };

        let ports = self.get_direction_mut(direction)?;

        let Some(port) = ports.get_mut(id) else {
            bail!("Port {id} not found in {direction:?} ports");
        };

        Ok(port)
    }

    fn get_direction(&self, direction: consts::Direction) -> Result<&Vec<Port>> {
        match direction {
            consts::Direction::INPUT => Ok(&self.input_ports),
            consts::Direction::OUTPUT => Ok(&self.output_ports),
            directin => panic!("Unknown port direction: {directin:?}"),
        }
    }

    fn get_direction_mut(&mut self, direction: consts::Direction) -> Result<&mut Vec<Port>> {
        match direction {
            consts::Direction::INPUT => Ok(&mut self.input_ports),
            consts::Direction::OUTPUT => Ok(&mut self.output_ports),
            directin => panic!("Unknown port direction: {directin:?}"),
        }
    }
}
