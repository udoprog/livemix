use anyhow::Result;
use pod::Pod;
use protocol::Connection;
use protocol::consts;
use protocol::flags;
use protocol::id;
use protocol::op;

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

    /// Get modified interest in the connection.
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
        let mut pod = Pod::array();

        pod.as_mut().push_struct(|st| {
            st.field()?.push_unsized(factory_name)?;
            st.field()?.push_unsized(ty)?;
            st.field().push(version)?;

            st.field()?.push_struct(|props| {
                props.field()?.push(6)?;

                props.field()?.push("node.description")?;
                props.field()?.push("livemix")?;

                props.field()?.push("node.name")?;
                props.field()?.push("livemix")?;

                props.field()?.push("media.class")?;
                props.field()?.push("Audio/Duplex")?;

                props.field()?.push("media.type")?;
                props.field()?.push("Audio")?;

                props.field()?.push("media.category")?;
                props.field()?.push("Duplex")?;

                props.field()?.push("media.role")?;
                props.field()?.push("DSP")?;
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
        let mut pod = Pod::array();

        pod.as_mut().push_struct(|st| {
            st.field()?.push_struct(|st| {
                st.field().push(2)?;

                st.field().push("application.name")?;
                st.field().push("livemix")?;

                st.field().push("node.name")?;
                st.field().push("livemix")?;
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
    pub fn client_node_update(
        &mut self,
        id: u32,
        max_input_ports: u32,
        max_output_ports: u32,
    ) -> Result<()> {
        let mut pod = Pod::array();

        let mut change_mask = flags::ClientNodeUpdate::NONE;
        change_mask |= flags::ClientNodeUpdate::PARAMS;
        change_mask |= flags::ClientNodeUpdate::INFO;

        let mut node_change_mask = flags::NodeChangeMask::FLAGS;
        node_change_mask |= flags::NodeChangeMask::PROPS;
        node_change_mask |= flags::NodeChangeMask::PARAMS;

        let node_flags = flags::Node::IN_DYNAMIC_PORTS | flags::Node::OUT_DYNAMIC_PORTS;

        pod.as_mut().push_struct(|st| {
            st.field().push(change_mask)?;

            st.field().push(2)?;

            st.field()?
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

            st.field()?
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

            if change_mask & flags::ClientNodeUpdate::INFO {
                st.field()?.push_struct(|st| {
                    st.field().push(max_input_ports)?;
                    st.field().push(max_output_ports)?;
                    st.field().push(node_change_mask)?;
                    st.field().push(node_flags)?;

                    st.field().push(1u32)?;
                    st.field().push("node.name")?;
                    st.field()?.push_unsized("livemix2")?;

                    st.field().push(4u32)?;
                    st.field().push(id::Param::PROP_INFO)?;
                    st.field().push(flags::Param::NONE)?;

                    st.field().push(id::Param::PROPS)?;
                    st.field().push(flags::Param::WRITE)?;

                    st.field().push(id::Param::ENUM_FORMAT)?;
                    st.field().push(flags::Param::READ)?;

                    st.field().push(id::Param::FORMAT)?;
                    st.field().push(flags::Param::WRITE)?;
                    Ok(())
                })?;
            } else {
                st.field()?.push_none()?;
            }

            Ok(())
        })?;

        self.connection.request(id, op::CLIENT_NODE_UPDATE, pod)?;
        Ok(())
    }

    /// Update client node port.
    #[tracing::instrument(skip(self))]
    pub fn client_node_port_update(
        &mut self,
        id: u32,
        direction: consts::Direction,
        port_id: u32,
        name: &str,
    ) -> Result<()> {
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
            st.field().push(direction)?;
            st.field().push(port_id)?;
            st.field().push(change_mask)?;

            // Parameters.
            st.field().push(2u32)?;

            st.field()?
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

            st.field()?
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

            if change_mask & flags::ClientNodePortUpdate::INFO {
                st.field()?.push_struct(|st| {
                    st.field().push(port_change_mask)?;
                    st.field().push(port_flags)?;

                    // Rate num / denom
                    st.field().push(0u32)?;
                    st.field().push(0u32)?;

                    // Properties.
                    st.field().push(2u32)?;
                    st.field().push("port.name")?;
                    st.field()?.push_unsized(name)?;

                    st.field().push("format.dsp")?;
                    st.field()?.push_unsized("32 bit float mono audio")?;

                    // Parameters.
                    st.field().push(5u32)?;
                    st.field().push(id::Param::ENUM_FORMAT)?;
                    st.field().push(flags::Param::READ)?;

                    st.field().push(id::Param::FORMAT)?;
                    st.field().push(flags::Param::WRITE)?;

                    st.field().push(id::Param::META)?;
                    st.field().push(flags::Param::READ)?;

                    st.field().push(id::Param::IO)?;
                    st.field().push(flags::Param::READ)?;

                    st.field().push(id::Param::BUFFERS)?;
                    st.field().push(flags::Param::NONE)?;
                    Ok(())
                })?;
            } else {
                st.field()?.push_none()?;
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

        pod.as_mut().push_struct(|st| {
            st.field().push(active)?;
            Ok(())
        })?;

        self.connection
            .request(id, op::CLIENT_NODE_SET_ACTIVE, pod)?;
        Ok(())
    }
}
