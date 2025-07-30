use core::mem;

use std::collections::BTreeMap;

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

use anyhow::{Result, bail};
use pod::{Object, Pod};
use protocol::{consts, id};
use tracing::Level;

use crate::{Buffers, Region, ffi};

#[derive(Debug)]
pub struct PortParam {
    pub value: Object<Box<[u64]>>,
    pub flags: u32,
}

impl PortParam {
    #[inline]
    fn new(value: Object<Box<[u64]>>, flags: u32) -> Self {
        Self { flags, value }
    }
}

#[derive(Debug)]
pub struct Port {
    id: u32,
    modified: bool,
    pub name: String,
    buffers: Option<Buffers>,
    pub io_clock: Option<Region<ffi::IoClock>>,
    pub io_position: Option<Region<ffi::IoPosition>>,
    pub io_buffers: Option<Region<ffi::IoBuffers>>,
    params: BTreeMap<id::Param, PortParam>,
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
        id: id::Param,
        value: Object<Box<[u64]>>,
        flags: u32,
    ) -> Result<()> {
        self.params.insert(id, PortParam::new(value, flags));
        self.modified = true;
        Ok(())
    }

    /// Remove a parameter from the port.
    #[inline]
    pub(crate) fn remove_param(&mut self, id: id::Param) -> Result<()> {
        self.params.remove(&id);
        self.modified = true;
        Ok(())
    }

    /// Get parameters from the port.
    pub(crate) fn params(&self) -> &BTreeMap<id::Param, PortParam> {
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
    pub(crate) fn insert(&mut self, direction: consts::Direction) -> Result<&mut Port> {
        let ports = self.get_direction_mut(direction)?;

        let id = ports.len() as u32;

        let mut port = Port {
            id,
            modified: true,
            name: String::new(),
            buffers: None,
            io_clock: None,
            io_position: None,
            io_buffers: None,
            params: BTreeMap::new(),
        };

        let mut pod = Pod::array();

        pod.as_mut()
            .push_object(id::ObjectType::FORMAT, id::Param::ENUM_FORMAT, |obj| {
                obj.property(id::Format::MEDIA_TYPE, 0)?
                    .push(id::MediaType::AUDIO)?;
                obj.property(id::Format::MEDIA_SUB_TYPE, 0)?
                    .push(id::MediaSubType::RAW)?;
                obj.property(id::Format::AUDIO_FORMAT, 0)?
                    .push(id::AudioFormat::S16)?;
                obj.property(id::Format::AUDIO_CHANNELS, 0)?.push(1u32)?;
                obj.property(id::Format::AUDIO_RATE, 0)?.push(44100u32)?;
                Ok(())
            })?;

        port.params.insert(
            id::Param::ENUM_FORMAT,
            PortParam::new(pod.as_ref().into_typed()?.next_object()?.to_owned(), 0),
        );

        pod.clear();

        pod.as_mut()
            .push_object(id::ObjectType::FORMAT, id::Param::FORMAT, |obj| {
                obj.property(id::Format::MEDIA_TYPE, 0)?
                    .push(id::MediaType::AUDIO)?;
                obj.property(id::Format::MEDIA_SUB_TYPE, 0)?
                    .push(id::MediaSubType::RAW)?;
                obj.property(id::Format::AUDIO_FORMAT, 0)?
                    .push(id::AudioFormat::S16)?;
                obj.property(id::Format::AUDIO_CHANNELS, 0)?.push(1u32)?;
                obj.property(id::Format::AUDIO_RATE, 0)?.push(44100u32)?;
                Ok(())
            })?;

        /*
        param = spa_pod_builder_add_object(&b.b,
            SPA_TYPE_OBJECT_ParamMeta, id,
            SPA_PARAM_META_type, SPA_POD_Id(SPA_META_Header),
            SPA_PARAM_META_size, SPA_POD_Int(sizeof(struct spa_meta_header)));
        */

        pod.as_mut()
            .push_object(id::ObjectType::PARAM_META, id::Param::META, |obj| {
                obj.property(id::Format::MEDIA_TYPE, 0)?
                    .push(id::MediaType::AUDIO)?;
                obj.property(id::Format::MEDIA_SUB_TYPE, 0)?
                    .push(id::MediaSubType::RAW)?;
                Ok(())
            })?;

        port.params.insert(
            id::Param::FORMAT,
            PortParam::new(pod.as_ref().into_typed()?.next_object()?.to_owned(), 0),
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
