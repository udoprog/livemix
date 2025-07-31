use core::mem;

use std::collections::BTreeMap;

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use anyhow::{Result, bail};
use pod::{ChoiceType, DynamicBuf, Object, Type};
use protocol::{
    consts,
    id::{
        self, AudioFormat, Format, MediaSubType, MediaType, ObjectType, Param, ParamBuffers,
        ParamIo, ParamMeta,
    },
};
use tracing::Level;

use crate::{Buffers, Region, ffi};

const BUFFER_SAMPLES: u32 = 128;

#[derive(Debug)]
pub struct PortParam {
    pub value: Object<DynamicBuf>,
    pub flags: u32,
}

impl PortParam {
    #[inline]
    fn new(value: Object<DynamicBuf>, flags: u32) -> Self {
        Self { flags, value }
    }
}

#[derive(Debug)]
pub struct Port {
    id: u32,
    dir: consts::Direction,
    modified: bool,
    pub name: String,
    buffers: Option<Buffers>,
    pub io_clock: Option<Region<ffi::IoClock>>,
    pub io_position: Option<Region<ffi::IoPosition>>,
    pub io_buffers: Option<Region<ffi::IoBuffers>>,
    params: BTreeMap<Param, Vec<PortParam>>,
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
    pub(crate) fn set_param(
        &mut self,
        id: Param,
        value: Object<DynamicBuf>,
        flags: u32,
    ) -> Result<()> {
        self.params.insert(id, vec![PortParam::new(value, flags)]);
        self.modified = true;
        Ok(())
    }

    /// Remove a parameter from the port.
    #[inline]
    pub(crate) fn remove_param(&mut self, id: Param) -> Result<()> {
        self.params.remove(&id);
        self.modified = true;
        Ok(())
    }

    /// Get parameters from the port.
    pub(crate) fn params(&self) -> &BTreeMap<Param, Vec<PortParam>> {
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
    pub(crate) fn inputs(&self) -> &[Port] {
        &self.input_ports
    }

    /// Access input ports mutably.
    pub(crate) fn inputs_mut(&mut self) -> &mut [Port] {
        &mut self.input_ports
    }

    /// Access output ports.
    pub(crate) fn outputs(&self) -> &[Port] {
        &self.output_ports
    }

    /// Access output ports mutably.
    pub(crate) fn outputs_mut(&mut self) -> &mut [Port] {
        &mut self.output_ports
    }

    /// Insert a new port in the specified direction.
    pub(crate) fn insert(&mut self, dir: consts::Direction) -> Result<&mut Port> {
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

        let mut pod = pod::array();

        pod.as_mut()
            .push_object(ObjectType::FORMAT, Param::ENUM_FORMAT, |obj| {
                obj.property(Format::MEDIA_TYPE).push(MediaType::AUDIO)?;
                obj.property(Format::MEDIA_SUB_TYPE)
                    .push(MediaSubType::RAW)?;
                obj.property(Format::AUDIO_FORMAT).push(AudioFormat::S16)?;
                obj.property(Format::AUDIO_CHANNELS).push(1u32)?;
                obj.property(Format::AUDIO_RATE).push(44100u32)?;
                Ok(())
            })?;

        port.params.insert(
            Param::ENUM_FORMAT,
            vec![PortParam::new(pod.take().next_object()?.to_owned()?, 0)],
        );

        pod.as_mut()
            .push_object(ObjectType::FORMAT, Param::FORMAT, |obj| {
                obj.property(Format::MEDIA_TYPE).push(MediaType::AUDIO)?;
                obj.property(Format::MEDIA_SUB_TYPE)
                    .push(MediaSubType::RAW)?;
                obj.property(Format::AUDIO_FORMAT).push(AudioFormat::S16)?;
                obj.property(Format::AUDIO_CHANNELS).push(1u32)?;
                obj.property(Format::AUDIO_RATE).push(44100u32)?;
                Ok(())
            })?;

        port.params.insert(
            Param::FORMAT,
            vec![PortParam::new(pod.take().next_object()?.to_owned()?, 0)],
        );

        pod.as_mut()
            .push_object(ObjectType::PARAM_META, Param::META, |obj| {
                obj.property(ParamMeta::TYPE).push(id::Meta::HEADER)?;
                obj.property(ParamMeta::SIZE)
                    .push(mem::size_of::<ffi::MetaHeader>())?;
                Ok(())
            })?;

        port.params.insert(
            Param::META,
            vec![PortParam::new(pod.take().next_object()?.to_owned()?, 0)],
        );

        {
            let mut params = Vec::new();

            pod.as_mut()
                .push_object(ObjectType::PARAM_IO, Param::IO, |obj| {
                    obj.property(ParamIo::ID).push(id::IoType::CLOCK)?;
                    obj.property(ParamIo::SIZE)
                        .push(mem::size_of::<ffi::IoClock>())?;
                    Ok(())
                })?;

            params.push(PortParam::new(pod.take().next_object()?.to_owned()?, 0));

            pod.as_mut()
                .push_object(ObjectType::PARAM_IO, Param::IO, |obj| {
                    obj.property(ParamIo::ID).push(id::IoType::POSITION)?;
                    obj.property(ParamIo::SIZE)
                        .push(mem::size_of::<ffi::IoPosition>())?;
                    Ok(())
                })?;

            params.push(PortParam::new(pod.take().next_object()?.to_owned()?, 0));

            port.params.insert(Param::IO, params);
        }

        pod.as_mut()
            .push_object(ObjectType::PARAM_BUFFERS, Param::BUFFERS, |obj| {
                obj.property(ParamBuffers::BUFFERS).push_choice(
                    ChoiceType::RANGE,
                    Type::INT,
                    |choice| {
                        choice.child().push(1u32)?;
                        choice.child().push(1u32)?;
                        choice.child().push(32u32)?;
                        Ok(())
                    },
                )?;

                obj.property(ParamBuffers::BLOCKS).push(1i32)?;

                obj.property(ParamBuffers::SIZE).push_choice(
                    ChoiceType::RANGE,
                    Type::INT,
                    |choice| {
                        choice
                            .child()
                            .push(BUFFER_SAMPLES * mem::size_of::<f32>() as u32)?;
                        choice.child().push(32)?;
                        choice.child().push(i32::MAX)?;
                        Ok(())
                    },
                )?;

                obj.property(ParamBuffers::STRIDE)
                    .push(mem::size_of::<f32>())?;
                Ok(())
            })?;

        port.params.insert(
            Param::BUFFERS,
            vec![PortParam::new(pod.take().next_object()?.to_owned()?, 0)],
        );

        ports.push(port);
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

    #[inline]
    fn get_direction(&self, dir: consts::Direction) -> Result<&Vec<Port>> {
        match dir {
            consts::Direction::INPUT => Ok(&self.input_ports),
            consts::Direction::OUTPUT => Ok(&self.output_ports),
            dir => panic!("Unknown port direction: {dir:?}"),
        }
    }

    #[inline]
    fn get_direction_mut(&mut self, dir: consts::Direction) -> Result<&mut Vec<Port>> {
        match dir {
            consts::Direction::INPUT => Ok(&mut self.input_ports),
            consts::Direction::OUTPUT => Ok(&mut self.output_ports),
            dir => panic!("Unknown port direction: {dir:?}"),
        }
    }
}
