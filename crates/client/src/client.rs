use std::collections::BTreeMap;
use std::os::fd::{AsRawFd, RawFd};

use alloc::vec::Vec;

use anyhow::Result;
use pod::{AsSlice, Object};
use protocol::buf::RecvBuf;
use protocol::buf::SendBuf;
use protocol::consts;
use protocol::flags;
use protocol::id;
use protocol::op;
use protocol::poll::{ChangeInterest, Interest};
use protocol::{Connection, Properties};
use tracing::Level;

use crate::ports::PortParam;
use crate::{LocalId, Parameters, PortId};

#[derive(Debug)]
pub struct Client {
    connection: Connection,
    sync_sequence: u32,
    outgoing: SendBuf,
}

impl Client {
    #[inline]
    pub fn new(connection: Connection) -> Self {
        Self {
            connection,
            sync_sequence: 1,
            outgoing: SendBuf::new(),
        }
    }

    /// Get the connection interest.
    #[inline]
    pub fn interest(&self) -> Interest {
        self.connection.interest()
    }

    #[inline]
    pub fn modify_interest(&mut self) -> ChangeInterest {
        self.connection.modified()
    }

    /// Receive file descriptors from the server.
    #[inline]
    pub fn recv_with_fds(&mut self, recv: &mut RecvBuf, fds: &mut [RawFd]) -> Result<usize> {
        Ok(self.connection.recv_with_fds(recv, fds)?)
    }

    /// Send data to the server.
    pub fn send(&mut self) -> Result<()> {
        self.connection.send(&mut self.outgoing)?;
        Ok(())
    }

    /// Send client hello.
    pub fn core_hello(&mut self) -> Result<()> {
        let mut pod = pod::array();
        pod.as_mut()
            .write_struct(|st| st.field().write_sized(consts::VERSION))?;

        self.connection.request(
            &mut self.outgoing,
            consts::CORE_ID,
            op::Core::HELLO,
            pod.as_ref(),
        )?;
        Ok(())
    }

    /// Get registry.
    pub fn core_get_registry(&mut self, new_id: LocalId) -> Result<()> {
        let mut pod = pod::array();

        pod.as_mut().write_struct(|st| {
            st.field().write(consts::REGISTRY_VERSION as i32)?;
            st.field().write(new_id.into_u32())?;
            Ok(())
        })?;

        self.connection.request(
            &mut self.outgoing,
            consts::CORE_ID,
            op::Core::GET_REGISTRY,
            pod.as_ref(),
        )?;
        Ok(())
    }

    /// Synchronize.
    pub fn core_sync(&mut self, id: i32) -> Result<u32> {
        let sync_sequence = self.sync_sequence;
        self.sync_sequence = self.sync_sequence.wrapping_add(1);

        let mut pod = pod::array();

        pod.as_mut().write_struct(|st| {
            st.field().write_sized(id)?;
            st.field().write_sized(sync_sequence)?;
            Ok(())
        })?;

        self.connection.request(
            &mut self.outgoing,
            consts::CORE_ID,
            op::Core::SYNC,
            pod.as_ref(),
        )?;
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

        self.connection.request(
            &mut self.outgoing,
            consts::CORE_ID,
            op::Core::PONG,
            pod.as_ref(),
        )?;
        Ok(())
    }

    /// Create an object.
    pub fn core_create_object(
        &mut self,
        factory_name: &str,
        ty: &str,
        version: u32,
        new_id: LocalId,
        properties: &Properties,
    ) -> Result<()> {
        let mut pod = pod::array();

        pod.as_mut().write_struct(|st| {
            st.field().write_unsized(factory_name)?;
            st.field().write_unsized(ty)?;
            st.field().write_sized(version)?;

            st.field().write_struct(|props| {
                props.field().write(properties.len() as u32)?;

                for pair in properties {
                    props.write(pair)?;
                }

                Ok(())
            })?;

            st.field().write_sized(new_id.into_u32())?;
            Ok(())
        })?;

        self.connection.request(
            &mut self.outgoing,
            consts::CORE_ID,
            op::Core::CREATE_OBJECT,
            pod.as_ref(),
        )?;
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
            })?;

            Ok(())
        })?;

        self.connection.request(
            &mut self.outgoing,
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

        self.connection.request(
            &mut self.outgoing,
            id,
            op::ClientNode::GET_NODE,
            pod.as_ref(),
        )?;
        Ok(())
    }

    /// Update client node.
    #[tracing::instrument(skip(self), ret(level = Level::TRACE))]
    pub fn client_node_update(
        &mut self,
        id: LocalId,
        max_input_ports: u32,
        max_output_ports: u32,
        properties: &mut Properties,
        parameters: &Parameters,
    ) -> Result<()> {
        let mut pod = pod::dynamic();

        let mut change_mask = flags::ClientNodeUpdate::NONE;
        change_mask |= flags::ClientNodeUpdate::PARAMS;
        change_mask |= flags::ClientNodeUpdate::INFO;

        let mut node_change_mask = flags::NodeChangeMask::FLAGS;

        let props_modified = properties.take_modified();

        if props_modified {
            node_change_mask |= flags::NodeChangeMask::PROPS;
        }

        if parameters.flags().len() > 0 {
            node_change_mask |= flags::NodeChangeMask::PARAMS;
        }

        let node_flags = flags::Node::IN_DYNAMIC_PORTS | flags::Node::OUT_DYNAMIC_PORTS;

        pod.as_mut().write_struct(|st| {
            st.field().write_sized(change_mask)?;

            st.field()
                .write_sized(parameters.values().map(|p| p.len()).sum::<usize>() as u32)?;

            for params in parameters.values() {
                for param in params {
                    st.field().write(param.value.as_ref())?;
                }
            }

            if change_mask & flags::ClientNodeUpdate::INFO {
                st.field().write_struct(|st| {
                    st.field().write_sized(max_input_ports)?;
                    st.field().write_sized(max_output_ports)?;
                    st.field().write_sized(node_change_mask)?;
                    st.field().write_sized(node_flags)?;

                    if props_modified {
                        st.field().write_sized(properties.len() as u32)?;

                        for (key, value) in properties.iter() {
                            st.write((key, value))?;
                        }
                    } else {
                        st.field().write(0u32)?;
                    }

                    st.field().write_sized(parameters.flags().len() as u32)?;

                    for (id, value) in parameters.flags() {
                        st.write(id)?;
                        st.write(value)?;
                    }

                    Ok(())
                })?;
            } else {
                st.field().write_none()?;
            }

            Ok(())
        })?;

        self.connection.request(
            &mut self.outgoing,
            id.into_u32(),
            op::ClientNode::UPDATE,
            pod.as_ref(),
        )?;
        Ok(())
    }

    /// Update client node port.
    #[tracing::instrument(skip(self), ret(level = Level::TRACE))]
    pub fn client_node_port_update(
        &mut self,
        id: LocalId,
        direction: consts::Direction,
        port_id: PortId,
        properties: &mut Properties,
        parameters: &mut Parameters,
    ) -> Result<()> {
        let mut pod = pod::dynamic();

        let mut change_mask = flags::ClientNodePortUpdate::NONE;

        if parameters.values().len() > 0 {
            change_mask |= flags::ClientNodePortUpdate::PARAMS;
        }

        change_mask |= flags::ClientNodePortUpdate::INFO;

        let mut port_change_mask = flags::PortChangeMask::NONE;
        port_change_mask |= flags::PortChangeMask::FLAGS;

        let props_modified = properties.take_modified();
        let params_modified = parameters.take_modified();

        if props_modified {
            port_change_mask |= flags::PortChangeMask::PROPS;
        }

        if params_modified {
            port_change_mask |= flags::PortChangeMask::PARAMS;
        }

        let port_flags = flags::Port::NONE;

        pod.as_mut().write_struct(|st| {
            st.write((direction, port_id))?;

            st.write(change_mask)?;

            // Parameters.
            st.write(parameters.values().map(|p| p.len()).sum::<usize>() as u32)?;

            for params in parameters.values() {
                for param in params {
                    st.field().write(param.value.as_ref())?;
                }
            }

            if change_mask & flags::ClientNodePortUpdate::INFO {
                st.field().write_struct(|st| {
                    st.write((port_change_mask, port_flags))?;

                    // Rate num / denom
                    st.field().write((0u32, 0u32))?;

                    // Properties.
                    if props_modified {
                        st.field().write_sized(properties.len() as u32)?;

                        for pair in properties.iter() {
                            st.write(pair)?;
                        }
                    } else {
                        st.write(0u32)?;
                    }

                    // Parameters.
                    if params_modified {
                        st.write(parameters.flags().len() as u32)?;

                        for (id, flag) in parameters.flags() {
                            st.write((id, flag))?;
                        }
                    } else {
                        st.write(0u32)?;
                    }

                    Ok(())
                })?;
            } else {
                st.field().write_none()?;
            }

            Ok(())
        })?;

        self.connection.request(
            &mut self.outgoing,
            id.into_u32(),
            op::ClientNode::PORT_UPDATE,
            pod.as_ref(),
        )?;
        Ok(())
    }

    /// Update the client.
    pub fn client_node_set_active(&mut self, id: LocalId, active: bool) -> Result<()> {
        let mut pod = pod::array();

        pod.as_mut().write_struct(|st| st.write(active))?;

        self.connection.request(
            &mut self.outgoing,
            id.into_u32(),
            op::ClientNode::SET_ACTIVE,
            pod.as_ref(),
        )?;
        Ok(())
    }
}

impl AsRawFd for Client {
    #[inline]
    fn as_raw_fd(&self) -> RawFd {
        self.connection.as_raw_fd()
    }
}
