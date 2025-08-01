use std::collections::BTreeMap;

use alloc::vec::Vec;

use anyhow::Result;
use pod::{AsSlice, Object};
use protocol::consts;
use protocol::flags;
use protocol::id;
use protocol::op;
use protocol::{Connection, Properties};
use tracing::Level;

use crate::ports::PortParam;

#[derive(Debug)]
pub struct Client {
    connection: Connection,
    sync_sequence: u32,
}

impl Client {
    #[inline]
    pub fn new(connection: Connection) -> Self {
        Self {
            connection,
            sync_sequence: 1,
        }
    }

    #[inline]
    pub fn connection(&self) -> &Connection {
        &self.connection
    }

    #[inline]
    pub fn connection_mut(&mut self) -> &mut Connection {
        &mut self.connection
    }

    /// Send client hello.
    pub fn core_hello(&mut self) -> Result<()> {
        let mut pod = pod::array();
        pod.as_mut()
            .write_struct(|st| st.field().write_sized(consts::VERSION))?;

        self.connection
            .request(consts::CORE_ID, op::Core::HELLO, pod.as_ref())?;
        Ok(())
    }

    /// Get registry.
    pub fn core_get_registry(&mut self, new_id: u32) -> Result<()> {
        let mut pod = pod::array();

        pod.as_mut().write_struct(|st| {
            st.field().write_sized(consts::REGISTRY_VERSION as i32)?;
            st.field().write_sized(new_id)?;
            Ok(())
        })?;

        self.connection
            .request(consts::CORE_ID, op::Core::GET_REGISTRY, pod.as_ref())?;
        Ok(())
    }

    /// Synchronize.
    pub fn core_sync(&mut self, id: u32) -> Result<u32> {
        let sync_sequence = self.sync_sequence;
        self.sync_sequence = self.sync_sequence.wrapping_add(1);

        let mut pod = pod::array();

        pod.as_mut().write_struct(|st| {
            st.field().write_sized(id)?;
            st.field().write_sized(sync_sequence)?;
            Ok(())
        })?;

        self.connection
            .request(consts::CORE_ID, op::Core::SYNC, pod.as_ref())?;
        Ok(sync_sequence)
    }

    /// Send a pong response to a ping.
    pub fn core_pong(&mut self, id: u32, seq: u32) -> Result<()> {
        let mut pod = pod::array();

        pod.as_mut().write_struct(|st| {
            st.field().write_sized(id)?;
            st.field().write_sized(seq)?;
            Ok(())
        })?;

        self.connection
            .request(consts::CORE_ID, op::Core::PONG, pod.as_ref())?;
        Ok(())
    }

    /// Create an object.
    pub fn core_create_object(
        &mut self,
        factory_name: &str,
        ty: &str,
        version: u32,
        new_id: u32,
    ) -> Result<()> {
        const PROPS: &[(&str, &str)] = &[
            ("node.description", "livemix"),
            ("node.name", "livemix_node"),
            ("media.class", "Audio/Duplex"),
            ("media.type", "Audio"),
            ("media.category", "Duplex"),
            ("media.role", "DSP"),
        ];

        let mut pod = pod::array();

        pod.as_mut().write_struct(|st| {
            st.field().write_unsized(factory_name)?;
            st.field().write_unsized(ty)?;
            st.field().write_sized(version)?;

            st.field().write_struct(|props| {
                props.field().write_sized(PROPS.len() as u32)?;
                props.write(PROPS)?;
                Ok(())
            })?;

            st.field().write_sized(new_id)?;
            Ok(())
        })?;

        self.connection
            .request(consts::CORE_ID, op::Core::CREATE_OBJECT, pod.as_ref())?;
        Ok(())
    }

    /// Update client properties.
    pub fn client_update_properties(&mut self, props: &Properties) -> Result<()> {
        let mut pod = pod::array();

        pod.as_mut().write_struct(|st| {
            st.field().write_struct(|st| {
                st.field().write_sized(props.len() as u32)?;

                for (key, value) in props.iter() {
                    st.write((key, value))?;
                }

                Ok(())
            })
        })?;

        self.connection.request(
            consts::CLIENT_ID,
            op::Client::UPDATE_PROPERTIES,
            pod.as_ref(),
        )?;
        Ok(())
    }

    /// Bind to client node.
    pub fn client_node_get_node(&mut self, id: u32, version: u32, new_id: u32) -> Result<()> {
        let mut pod = pod::array();

        pod.as_mut().write_struct(|st| {
            st.field().write_sized(version)?;
            st.field().write_sized(new_id)?;
            Ok(())
        })?;

        self.connection
            .request(id, op::ClientNode::GET_NODE, pod.as_ref())?;
        Ok(())
    }

    /// Update client node.
    #[tracing::instrument(skip(self, params), fields(params = ?params.keys()), ret(level = Level::DEBUG))]
    pub fn client_node_update(
        &mut self,
        id: u32,
        max_input_ports: u32,
        max_output_ports: u32,
        params: &BTreeMap<id::Param, Vec<Object<impl AsSlice>>>,
    ) -> Result<()> {
        const PARAMS: &[(id::Param, flags::Param)] = &[
            (id::Param::ENUM_FORMAT, flags::Param::READWRITE),
            (id::Param::FORMAT, flags::Param::READWRITE),
            (id::Param::PROP_INFO, flags::Param::WRITE),
            (id::Param::PROPS, flags::Param::WRITE),
            (id::Param::ENUM_PORT_CONFIG, flags::Param::WRITE),
            (id::Param::PORT_CONFIG, flags::Param::WRITE),
            (id::Param::LATENCY, flags::Param::WRITE),
            (id::Param::PROCESS_LATENCY, flags::Param::WRITE),
            (id::Param::TAG, flags::Param::WRITE),
        ];

        const PROPS: &[(&str, &str)] = &[("node.name", "livemix_node")];

        let mut pod = pod::dynamic();

        let mut change_mask = flags::ClientNodeUpdate::NONE;
        change_mask |= flags::ClientNodeUpdate::PARAMS;
        change_mask |= flags::ClientNodeUpdate::INFO;

        let mut node_change_mask = flags::NodeChangeMask::FLAGS;
        node_change_mask |= flags::NodeChangeMask::PROPS;

        if !PARAMS.is_empty() {
            node_change_mask |= flags::NodeChangeMask::PARAMS;
        }

        let node_flags = flags::Node::IN_DYNAMIC_PORTS | flags::Node::OUT_DYNAMIC_PORTS;

        pod.as_mut().write_struct(|st| {
            st.field().write_sized(change_mask)?;

            st.field()
                .write_sized(params.values().map(|p| p.len()).sum::<usize>() as u32)?;

            for (_, params) in params {
                for param in params {
                    st.field().write(param.as_ref())?;
                }
            }

            if change_mask & flags::ClientNodeUpdate::INFO {
                st.field().write_struct(|st| {
                    st.field().write_sized(max_input_ports)?;
                    st.field().write_sized(max_output_ports)?;
                    st.field().write_sized(node_change_mask)?;
                    st.field().write_sized(node_flags)?;

                    st.field().write_sized(PROPS.len() as u32)?;
                    st.write(PROPS)?;

                    st.field().write_sized(PARAMS.len() as u32)?;
                    st.write(PARAMS)?;
                    Ok(())
                })?;
            } else {
                st.field().write_none()?;
            }

            Ok(())
        })?;

        self.connection
            .request(id, op::ClientNode::UPDATE, pod.as_ref())?;
        Ok(())
    }

    /// Update client node port.
    #[tracing::instrument(skip(self, params), fields(params = ?params.keys()), ret(level = Level::DEBUG))]
    pub fn client_node_port_update(
        &mut self,
        id: u32,
        direction: consts::Direction,
        port_id: u32,
        name: &str,
        params: &BTreeMap<id::Param, Vec<PortParam>>,
    ) -> Result<()> {
        const PARAMS: &[(id::Param, flags::Param)] = &[
            (id::Param::ENUM_FORMAT, flags::Param::READWRITE),
            (id::Param::META, flags::Param::WRITE),
            (id::Param::IO, flags::Param::WRITE),
            (id::Param::FORMAT, flags::Param::READWRITE),
            (id::Param::BUFFERS, flags::Param::WRITE),
            (id::Param::LATENCY, flags::Param::WRITE),
        ];

        let mut pod = pod::dynamic();

        let mut change_mask = flags::ClientNodePortUpdate::NONE;
        change_mask |= flags::ClientNodePortUpdate::PARAMS;
        change_mask |= flags::ClientNodePortUpdate::INFO;

        let mut port_change_mask = flags::PortChangeMask::NONE;
        port_change_mask |= flags::PortChangeMask::FLAGS;
        port_change_mask |= flags::PortChangeMask::PROPS;
        port_change_mask |= flags::PortChangeMask::PARAMS;

        let port_flags = flags::Port::NONE;

        pod.as_mut().write_struct(|st| {
            st.write((direction, port_id, change_mask))?;

            // Parameters.
            st.field()
                .write_sized(params.iter().map(|(_, p)| p.len()).sum::<usize>() as u32)?;

            for (_, params) in params {
                for param in params {
                    st.field().write(param.value.as_ref())?;
                }
            }

            if change_mask & flags::ClientNodePortUpdate::INFO {
                st.field().write_struct(|st| {
                    st.field().write_sized(port_change_mask)?;
                    st.field().write_sized(port_flags)?;

                    // Rate num / denom
                    st.field().write_sized(0u32)?;
                    st.field().write_sized(0u32)?;

                    // Properties.
                    st.field().write_sized(2u32)?;
                    st.field().write_unsized("port.name")?;
                    st.field().write_unsized(name)?;

                    st.field().write_unsized("format.dsp")?;
                    st.field().write_unsized("32 bit float mono audio")?;

                    // Parameters.
                    st.field().write_sized(PARAMS.len() as u32)?;
                    st.write(PARAMS)?;
                    Ok(())
                })?;
            } else {
                st.field().write_none()?;
            }

            Ok(())
        })?;

        self.connection
            .request(id, op::ClientNode::PORT_UPDATE, pod.as_ref())?;
        Ok(())
    }

    /// Update the client.
    pub fn client_node_set_active(&mut self, id: u32, active: bool) -> Result<()> {
        let mut pod = pod::array();

        pod.as_mut().write_struct(|st| st.write(active))?;

        self.connection
            .request(id, op::ClientNode::SET_ACTIVE, pod.as_ref())?;
        Ok(())
    }
}
