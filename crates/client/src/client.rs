use std::collections::BTreeMap;

use alloc::boxed::Box;

use anyhow::Result;
use pod::Object;
use pod::Pod;
use protocol::Connection;
use protocol::consts;
use protocol::flags;
use protocol::id;
use protocol::op;
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
        let mut pod = Pod::array();
        pod.as_mut()
            .push_struct(|st| st.field().push(consts::VERSION))?;

        self.connection
            .request(consts::CORE_ID, op::CORE_HELLO, pod)?;
        Ok(())
    }

    /// Get registry.
    pub fn core_get_registry(&mut self, new_id: u32) -> Result<()> {
        let mut pod = Pod::array();

        pod.as_mut().push_struct(|st| {
            st.field().push(consts::REGISTRY_VERSION as i32)?;
            st.field().push(new_id)?;
            Ok(())
        })?;

        self.connection
            .request(consts::CORE_ID, op::CORE_GET_REGISTRY, pod)?;
        Ok(())
    }

    /// Synchronize.
    pub fn core_sync(&mut self, id: u32) -> Result<u32> {
        let sync_sequence = self.sync_sequence;
        self.sync_sequence = self.sync_sequence.wrapping_add(1);

        let mut pod = Pod::array();

        pod.as_mut().push_struct(|st| {
            st.field().push(id)?;
            st.field().push(sync_sequence)?;
            Ok(())
        })?;

        self.connection
            .request(consts::CORE_ID, op::CORE_SYNC, pod)?;
        Ok(sync_sequence)
    }

    /// Send a pong response to a ping.
    pub fn core_pong(&mut self, id: u32, seq: u32) -> Result<()> {
        let mut pod = Pod::array();

        pod.as_mut().push_struct(|st| {
            st.field().push(id)?;
            st.field().push(seq)?;
            Ok(())
        })?;

        self.connection
            .request(consts::CORE_ID, op::CORE_PONG, pod)?;
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

        let mut pod = Pod::array();

        pod.as_mut().push_struct(|st| {
            st.field().push_unsized(factory_name)?;
            st.field().push_unsized(ty)?;
            st.field().push(version)?;

            st.field().push_struct(|props| {
                props.field().push(PROPS.len() as u32)?;
                props.encode(PROPS)?;
                Ok(())
            })?;

            st.field().push(new_id)?;
            Ok(())
        })?;

        self.connection
            .request(consts::CORE_ID, op::CORE_CREATE_OBJECT, pod)?;
        Ok(())
    }

    /// Update client properties.
    pub fn client_update_properties(&mut self) -> Result<()> {
        const PROPS: &[(&str, &str)] = &[
            ("application.name", "livemix"),
            ("node.name", "livemix_node"),
        ];

        let mut pod = Pod::array();

        pod.as_mut().push_struct(|st| {
            st.field().push_struct(|props| {
                props.field().push(PROPS.len() as u32)?;
                props.encode(PROPS)?;
                Ok(())
            })
        })?;

        self.connection
            .request(consts::CLIENT_ID, op::CLIENT_UPDATE_PROPERTIES, pod)?;
        Ok(())
    }

    /// Bind to client node.
    pub fn client_node_get_node(&mut self, id: u32, version: u32, new_id: u32) -> Result<()> {
        let mut pod = Pod::array();

        pod.as_mut().push_struct(|st| {
            st.field().push(version)?;
            st.field().push(new_id)?;
            Ok(())
        })?;

        self.connection.request(id, op::CLIENT_NODE_GET_NODE, pod)?;
        Ok(())
    }

    /// Update client node.
    #[tracing::instrument(skip(self, params), fields(params = ?params.keys()), ret(level = Level::DEBUG))]
    pub fn client_node_update(
        &mut self,
        id: u32,
        max_input_ports: u32,
        max_output_ports: u32,
        params: &BTreeMap<id::Param, Object<Box<[u64]>>>,
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

        let mut pod = Pod::array();

        let mut change_mask = flags::ClientNodeUpdate::NONE;
        change_mask |= flags::ClientNodeUpdate::PARAMS;
        change_mask |= flags::ClientNodeUpdate::INFO;

        let mut node_change_mask = flags::NodeChangeMask::FLAGS;
        node_change_mask |= flags::NodeChangeMask::PROPS;

        if !PARAMS.is_empty() {
            node_change_mask |= flags::NodeChangeMask::PARAMS;
        }

        let node_flags = flags::Node::IN_DYNAMIC_PORTS | flags::Node::OUT_DYNAMIC_PORTS;

        pod.as_mut().push_struct(|st| {
            st.field().push(change_mask)?;

            st.field().push(params.len() as u32)?;

            for (_, value) in params {
                st.field().encode(value.as_ref())?;
            }

            if change_mask & flags::ClientNodeUpdate::INFO {
                st.field().push_struct(|st| {
                    st.field().push(max_input_ports)?;
                    st.field().push(max_output_ports)?;
                    st.field().push(node_change_mask)?;
                    st.field().push(node_flags)?;

                    st.field().push(PROPS.len() as u32)?;
                    st.encode(PROPS)?;

                    st.field().push(PARAMS.len() as u32)?;
                    st.encode(PARAMS)?;
                    Ok(())
                })?;
            } else {
                st.field().push_none()?;
            }

            Ok(())
        })?;

        self.connection.request(id, op::CLIENT_NODE_UPDATE, pod)?;
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
        params: &BTreeMap<id::Param, PortParam>,
    ) -> Result<()> {
        const PARAMS: &[(id::Param, flags::Param)] = &[
            (id::Param::ENUM_FORMAT, flags::Param::READWRITE),
            (id::Param::META, flags::Param::WRITE),
            (id::Param::IO, flags::Param::WRITE),
            (id::Param::FORMAT, flags::Param::READWRITE),
            (id::Param::BUFFERS, flags::Param::WRITE),
            (id::Param::LATENCY, flags::Param::WRITE),
        ];

        let mut pod = Pod::array();

        let mut change_mask = flags::ClientNodePortUpdate::NONE;
        change_mask |= flags::ClientNodePortUpdate::PARAMS;
        change_mask |= flags::ClientNodePortUpdate::INFO;

        let mut port_change_mask = flags::PortChangeMask::NONE;
        port_change_mask |= flags::PortChangeMask::FLAGS;
        port_change_mask |= flags::PortChangeMask::PROPS;
        port_change_mask |= flags::PortChangeMask::PARAMS;

        let port_flags = flags::Port::NONE;

        pod.as_mut().push_struct(|st| {
            st.encode((direction, port_id, change_mask))?;

            // Parameters.
            st.field().push(params.len() as u32)?;

            for (_, param) in params {
                st.field().encode(param.value.as_ref())?;
            }

            if change_mask & flags::ClientNodePortUpdate::INFO {
                st.field().push_struct(|st| {
                    st.field().push(port_change_mask)?;
                    st.field().push(port_flags)?;

                    // Rate num / denom
                    st.field().push(0u32)?;
                    st.field().push(0u32)?;

                    // Properties.
                    st.field().push(2u32)?;
                    st.field().push_unsized("port.name")?;
                    st.field().push_unsized(name)?;

                    st.field().push_unsized("format.dsp")?;
                    st.field().push_unsized("32 bit float mono audio")?;

                    // Parameters.
                    st.field().push(PARAMS.len() as u32)?;
                    st.encode(PARAMS)?;
                    Ok(())
                })?;
            } else {
                st.field().push_none()?;
            }

            Ok(())
        })?;

        self.connection
            .request(id, op::CLIENT_NODE_PORT_UPDATE, pod)?;
        Ok(())
    }

    /// Update the client.
    pub fn client_node_set_active(&mut self, id: u32, active: bool) -> Result<()> {
        let mut pod = Pod::array();

        pod.as_mut().push_struct(|st| st.encode(active))?;

        self.connection
            .request(id, op::CLIENT_NODE_SET_ACTIVE, pod)?;
        Ok(())
    }
}
