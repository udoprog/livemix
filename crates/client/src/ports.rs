use core::fmt;
use core::mem;

use std::collections::BTreeMap;
use std::collections::VecDeque;
use std::collections::btree_map::Entry;

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use anyhow::{Result, bail};
use pod::AsSlice;
use pod::PodItem;
use pod::PodSink;
use pod::PodStream;
use pod::Readable;
use pod::Writable;
use pod::{ChoiceType, DynamicBuf, Object, Type};
use protocol::consts::{self, Direction};
use protocol::flags::ParamFlag;
use protocol::id::{
    self, AudioFormat, Format, MediaSubType, MediaType, ObjectType, Param, ParamBuffers, ParamIo,
    ParamMeta,
};
use protocol::{ffi, flags, object};
use tracing::Level;

use crate::buffer::Buffer;
use crate::{Buffers, Region};

/// The identifier of a port.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct PortId(u32);

impl PortId {
    /// Get the index of the port.
    ///
    /// Since it was constructed from a `usize`, it can always be safely coerced
    /// into one.
    fn index(self) -> usize {
        self.0 as usize
    }
}

impl fmt::Display for PortId {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Writable for PortId {
    #[inline]
    fn write_into(&self, pod: &mut impl PodSink) -> Result<(), pod::Error> {
        pod.next()?.write(self.0)
    }
}

impl<'de> Readable<'de> for PortId {
    #[inline]
    fn read_from(pod: &mut impl PodStream<'de>) -> Result<Self, pod::Error> {
        let pod = pod.next()?;
        Ok(PortId(pod.read_sized::<u32>()?))
    }
}

/// A port parameter with associated flags.
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

/// A set of allocated buffers for a port.
#[derive(Default)]
pub struct PortBuffers {
    /// The buffers associated with the port.
    buffers: Option<Buffers>,
    /// The available buffers.
    available: VecDeque<u32>,
}

impl PortBuffers {
    /// Free the given buffer by id.
    pub fn free(&mut self, id: u32) {
        self.available.push_back(id);
    }

    /// Get the next free buffer in the set.
    pub fn next(&mut self) -> Option<&mut Buffer> {
        let buffers = self.buffers.as_mut()?;
        let id = self.available.pop_front()?;
        buffers.buffers.get_mut(id as usize)
    }

    /// Just get the specified buffer by id.
    pub fn get_mut(&mut self, id: u32) -> Option<&mut Buffer> {
        let buffers = self.buffers.as_mut()?;
        let index = usize::try_from(id).ok()?;
        buffers.buffers.get_mut(index)
    }
}

/// The definition of a port.
#[non_exhaustive]
pub struct Port {
    /// The direction of the port.
    pub direction: Direction,
    /// Identifier of the port, unique per direction.
    pub id: PortId,
    /// The name of the port.
    pub name: String,
    /// List of available buffers for the port.
    pub buffers: PortBuffers,
    /// The IO clock region for the port.
    pub io_clock: Option<Region<ffi::IoClock>>,
    /// The IO position region for the port.
    pub io_position: Option<Region<ffi::IoPosition>>,
    /// The IO buffers region for the port.
    pub io_buffers: Option<Region<ffi::IoBuffers>>,
    /// The audio format of the port.
    pub format: Option<object::AudioFormat>,
    modified: bool,
    param_values: BTreeMap<Param, Vec<PortParam<DynamicBuf>>>,
    param_flags: BTreeMap<Param, ParamFlag>,
}

impl Port {
    /// Take the modified state of the port.
    #[inline]
    pub(crate) fn take_modified(&mut self) -> bool {
        mem::take(&mut self.modified)
    }

    /// Set a parameter flag.
    fn set_flag(&mut self, id: Param, flag: flags::ParamFlag) {
        match self.param_flags.entry(id) {
            Entry::Vacant(e) => {
                e.insert(flag);
            }
            Entry::Occupied(e) => {
                if e.get().contains(flag) {
                    return;
                }

                *e.into_mut() |= flag;
            }
        }

        self.modified = true;
    }

    /// Set a parameter flag.
    pub fn set_read(&mut self, id: Param) {
        self.set_flag(id, flags::ParamFlag::READ);
    }

    /// Set that a parameter is writable.
    pub fn set_write(&mut self, id: Param) {
        self.set_flag(id, flags::ParamFlag::WRITE);
    }

    /// Set a parameter on the port to the given values.
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

        self.param_values.insert(id, params);
        self.set_flag(id, flags::ParamFlag::READ);
        self.modified = true;
        Ok(())
    }

    /// Push a parameter.
    ///
    /// This will append the value to the existing set of parameters of the
    /// given type.
    #[inline]
    pub fn push_param(&mut self, id: Param, value: PortParam<impl AsSlice>) -> Result<()> {
        self.param_values
            .entry(id)
            .or_default()
            .push(PortParam::with_flags(
                value.value.as_ref().to_owned()?,
                value.flags,
            ));

        self.set_flag(id, flags::ParamFlag::READ);
        self.modified = true;
        Ok(())
    }

    /// Remove a parameter from the port and return the values of the removed
    /// parameter if it exists.
    #[inline]
    pub fn remove_param(&mut self, id: Param) -> Option<Vec<PortParam>> {
        let param = self.param_values.remove(&id)?;

        // If we remove a parameter it is no longer readable.
        let flag = self.param_flags.entry(id).or_default();
        *flag ^= flags::ParamFlag::READ;

        self.modified = true;
        Some(param)
    }

    /// Get the values of a parameter.
    pub fn get_param(&self, id: Param) -> &[PortParam<DynamicBuf>] {
        self.param_values
            .get(&id)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    /// Get parameters from the port.
    pub(crate) fn param_values(&self) -> &BTreeMap<Param, Vec<PortParam<impl AsSlice>>> {
        &self.param_values
    }

    /// Get parameters from the port.
    pub(crate) fn param_flags(&self) -> &BTreeMap<Param, flags::ParamFlag> {
        &self.param_flags
    }

    /// Replace the current set of buffers for this port.
    #[inline]
    #[tracing::instrument(skip(self), fields(port_id = ?self.id), ret(level = Level::TRACE))]
    pub(crate) fn replace_buffers(&mut self, buffers: Buffers) -> Option<Buffers> {
        let len = buffers.buffers.len();
        let old = self.buffers.buffers.replace(buffers);

        self.buffers.available.clear();

        for id in 0..len {
            self.buffers.available.push_back(id as u32);
        }

        old
    }
}

#[derive(Default)]
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
    pub fn insert(&mut self, direction: Direction) -> Result<&mut Port> {
        let ports = self.get_direction_mut(direction)?;

        let Ok(id) = u32::try_from(ports.len()) else {
            bail!("Too many ports in {direction:?} direction");
        };

        let id = PortId(id);

        let mut port = Port {
            direction,
            id,
            modified: true,
            name: String::new(),
            buffers: PortBuffers::default(),
            io_clock: None,
            io_position: None,
            io_buffers: None,
            format: None,
            param_values: BTreeMap::new(),
            param_flags: BTreeMap::new(),
        };

        ports.push(port);
        Ok(&mut ports[id.index()])
    }

    /// Get a port.
    pub fn get(&self, direction: Direction, id: PortId) -> Result<&Port> {
        let ports = self.get_direction(direction)?;

        let Some(port) = ports.get(id.index()) else {
            bail!("Port {id} not found in {direction:?} ports");
        };

        Ok(port)
    }

    /// Get a port mutably.
    pub fn get_mut(&mut self, direction: Direction, id: PortId) -> Result<&mut Port> {
        let ports = self.get_direction_mut(direction)?;

        let Some(port) = ports.get_mut(id.index()) else {
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
