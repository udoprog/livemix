use core::mem;

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::os::fd::{AsRawFd, OwnedFd, RawFd};

use alloc::borrow::ToOwned;
use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

use anyhow::{Context, Result, bail};
use pod::{Fd, Object, Pod, Struct};
use protocol::ids::Ids;
use protocol::op;
use protocol::poll::{Interest, Token};
use protocol::types::Header;
use protocol::{Connection, DynamicBuf};
use protocol::{EventFd, id};
use protocol::{consts, flags};
use slab::Slab;
use tracing::Level;

use crate::activation::Activation;
use crate::buffer::{self, Buffer};
use crate::{Buffers, Client, Memory, Ports, Region, ffi};

const CREATE_CLIENT_NODE: u32 = 0x2000;
const GET_REGISTRY_SYNC: u32 = 0x1000;

macro_rules! tracing_error {
    ($error:expr, $($tt:tt)*) => {{
        tracing::error!(error = ?$error, $($tt)*);

        for error in $error.chain().skip(1) {
            tracing::error!(?error, "Caused by");
        }
    }};
}

#[derive(Default, Debug)]
struct CoreState {
    id: u32,
    cookie: i32,
    user_name: String,
    host_name: String,
    version: String,
    name: String,
    properties: BTreeMap<String, String>,
}

#[derive(Default, Debug)]
struct ClientState {
    id: u32,
    properties: BTreeMap<String, String>,
}

#[derive(Default, Debug)]
struct RegistryState {
    id: u32,
    permissions: i32,
    ty: String,
    version: u32,
    properties: BTreeMap<String, String>,
}

#[derive(Debug)]
#[allow(unused)]
struct ClientNodeState {
    id: u32,
    read_fd: Option<OwnedFd>,
    write_token: Token,
    write_fd: Option<OwnedFd>,
    read_token: Token,
    /// Activation record for this node.
    activation: Option<Region<ffi::NodeActivation>>,
    /// Activation records for dependent nodes.
    node_activations: Slab<Activation>,
    /// Map of peer ids to their activation indices.
    peer_to_activation: BTreeMap<u32, usize>,
    params: BTreeMap<id::Param, Object<Box<[u64]>>>,
    ports: Ports,
    io_clock: Option<Region<ffi::IoClock>>,
    io_control: Option<Region<()>>,
    io_position: Option<Region<ffi::IoPosition>>,
    modified: bool,
}

impl ClientNodeState {
    pub(crate) fn new(
        id: u32,
        ports: Ports,
        write_token: Token,
        read_token: Token,
    ) -> Result<Self> {
        let mut params = BTreeMap::new();
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

        params.insert(
            id::Param::ENUM_FORMAT,
            pod.as_ref().into_typed()?.next_object()?.to_owned(),
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

        params.insert(
            id::Param::FORMAT,
            pod.as_ref().into_typed()?.next_object()?.to_owned(),
        );

        Ok(Self {
            id,
            ports,
            write_fd: None,
            read_fd: None,
            write_token,
            read_token,
            activation: None,
            node_activations: Slab::new(),
            peer_to_activation: BTreeMap::new(),
            params,
            io_control: None,
            io_clock: None,
            io_position: None,
            modified: true,
        })
    }

    /// Set a parameter for the node.
    #[inline]
    fn set_param(&mut self, param: id::Param, value: Object<Box<[u64]>>) {
        self.params.insert(param, value);
        self.modified = true;
    }

    /// Remove a parameter for the node.
    #[inline]
    fn remove_param(&mut self, param: id::Param) {
        self.params.remove(&param);
        self.modified = true;
    }

    /// Take and return the modified state of the node.
    #[inline]
    fn take_modified(&mut self) -> bool {
        mem::take(&mut self.modified)
    }
}

#[derive(Debug)]
enum Kind {
    Registry,
    ClientNode(usize),
}

#[derive(Debug)]
struct ReceivedFd {
    fd: Option<OwnedFd>,
}

#[derive(Debug)]
enum Op {
    CoreHello,
    GetRegistry,
    Pong { id: u32, seq: u32 },
    ConstructNode,
    NodeUpdate { client: usize },
    ReadInterest { client: usize },
}

#[derive(Debug)]
pub struct GlobalMap {
    global_to_local: BTreeMap<u32, u32>,
}

impl GlobalMap {
    #[inline]
    fn new() -> Self {
        Self {
            global_to_local: BTreeMap::new(),
        }
    }

    #[inline]
    fn insert(&mut self, local_id: u32, global_id: u32) {
        self.global_to_local.insert(global_id, local_id);
    }

    /// Map a global to a local id.
    #[inline]
    fn by_global(&self, global_id: u32) -> Option<u32> {
        self.global_to_local.get(&global_id).copied()
    }

    #[inline]
    fn remove_by_global(&mut self, global_id: u32) -> Option<u32> {
        self.global_to_local.remove(&global_id)
    }
}

/// The local connection state.
#[derive(Debug)]
pub struct State {
    c: Client,
    core: CoreState,
    client: ClientState,
    registries: Slab<RegistryState>,
    id_to_registry: BTreeMap<u32, usize>,
    factories: BTreeMap<String, usize>,
    globals: GlobalMap,
    client_nodes: Slab<ClientNodeState>,
    local_id_to_kind: BTreeMap<u32, Kind>,
    has_header: bool,
    header: Header,
    ids: Ids,
    tokens: Ids,
    read_to_client: HashMap<Token, usize>,
    write_to_client: HashMap<Token, usize>,
    fds: VecDeque<ReceivedFd>,
    ops: VecDeque<Op>,
    memory: Memory,
    add_interest: VecDeque<(RawFd, Token, Interest)>,
    modify_interest: VecDeque<(RawFd, Token, Interest)>,
}

impl State {
    pub fn new(connection: Connection) -> Self {
        let mut ids = Ids::new();

        // Well-known identifiers.
        ids.set(consts::CORE_ID);
        ids.set(consts::CLIENT_ID);

        Self {
            c: Client::new(connection),
            core: CoreState::default(),
            client: ClientState::default(),
            registries: Slab::new(),
            id_to_registry: BTreeMap::new(),
            factories: BTreeMap::new(),
            globals: GlobalMap::new(),
            client_nodes: Slab::new(),
            local_id_to_kind: BTreeMap::new(),
            has_header: false,
            header: Header::default(),
            ids,
            tokens: Ids::new(),
            read_to_client: HashMap::new(),
            write_to_client: HashMap::new(),
            fds: VecDeque::with_capacity(16),
            ops: VecDeque::from([Op::CoreHello]),
            memory: Memory::new(),
            add_interest: VecDeque::new(),
            modify_interest: VecDeque::new(),
        }
    }

    /// Allocate a unique token.
    #[inline]
    pub fn token(&mut self) -> Result<Token> {
        Ok(Token::new(
            self.tokens.alloc().context("no more tokens")? as u64
        ))
    }

    #[inline]
    pub fn add_interest(&mut self) -> Option<(RawFd, Token, Interest)> {
        self.add_interest.pop_front()
    }

    #[inline]
    pub fn modify_interest(&mut self) -> Option<(RawFd, Token, Interest)> {
        self.modify_interest.pop_front()
    }

    #[inline]
    pub fn connection(&self) -> &Connection {
        self.c.connection()
    }

    #[inline]
    pub fn connection_mut(&mut self) -> &mut Connection {
        self.c.connection_mut()
    }

    /// Add file descriptors.
    pub fn add_fds(&mut self, fds: impl IntoIterator<Item = OwnedFd>) {
        for fd in fds {
            self.fds.push_back(ReceivedFd { fd: Some(fd) });
        }
    }

    /// Process client.
    pub fn run(&mut self, recv: &mut DynamicBuf) -> Result<()> {
        'next: loop {
            while let Some(op) = self.ops.pop_front() {
                match op {
                    Op::CoreHello => {
                        self.c.core_hello()?;
                        self.c.client_update_properties()?;
                    }
                    Op::GetRegistry => {
                        tracing::info!("Getting registry");

                        let local_id = self.ids.alloc().context("ran out of identifiers")?;
                        self.c.core_get_registry(local_id)?;
                        self.local_id_to_kind.insert(local_id, Kind::Registry);
                        self.c.core_sync(GET_REGISTRY_SYNC)?;
                    }
                    Op::Pong { id, seq } => {
                        self.c.core_pong(id, seq)?;
                    }
                    Op::ConstructNode => {
                        if let Err(error) = self.op_construct_node() {
                            tracing_error!(error, "Failed to construct client node");
                        }
                    }
                    Op::NodeUpdate { client } => {
                        if let Some(client) = self.client_nodes.get_mut(client) {
                            if client.take_modified() {
                                self.c.client_node_update(client.id, 4, 4, &client.params)?;
                            }

                            for port in client.ports.inputs_mut() {
                                if !port.take_modified() {
                                    continue;
                                }

                                self.c.client_node_port_update(
                                    client.id,
                                    consts::Direction::INPUT,
                                    port.id(),
                                    &port.name,
                                    port.params(),
                                )?;
                            }

                            for port in client.ports.outputs_mut() {
                                if !port.take_modified() {
                                    continue;
                                }

                                self.c.client_node_port_update(
                                    client.id,
                                    consts::Direction::OUTPUT,
                                    port.id(),
                                    &port.name,
                                    port.params(),
                                )?;
                            }
                        }
                    }
                    Op::ReadInterest { client: index } => {
                        if let Some(client) = self.client_nodes.get(index) {
                            if let Some(read_fd) = &client.read_fd {
                                self.read_to_client.insert(client.read_token, index);
                                self.add_interest.push_back((
                                    read_fd.as_raw_fd(),
                                    client.read_token,
                                    Interest::READ | Interest::HUP | Interest::ERROR,
                                ));
                            }

                            if let Some(write_fd) = &client.write_fd {
                                self.write_to_client.insert(client.write_token, index);
                                self.add_interest.push_back((
                                    write_fd.as_raw_fd(),
                                    client.write_token,
                                    Interest::HUP | Interest::ERROR,
                                ));
                            }
                        }
                    }
                }
            }

            if !self.has_header {
                if let Some(h) = recv.read::<Header>() {
                    self.header = h;
                    self.has_header = true;
                }
            }

            'done: {
                if !self.has_header {
                    break 'done;
                }

                if (self.header.n_fds() as usize) > self.fds.len() {
                    break 'done;
                }

                let Some(pod) = recv.frame(&self.header) else {
                    break 'done;
                };

                let result = match self.header.id() {
                    consts::CORE_ID => self.core(pod),
                    consts::CLIENT_ID => self.client(pod),
                    _ => self.dynamic(pod),
                };

                if self.header.n_fds() > 0 {
                    let n = self.header.n_fds() as usize;

                    for fd in self.fds.drain(..n) {
                        if let Some(fd) = fd.fd {
                            tracing::warn!("Unused file descriptor dropped: {fd:?}");
                        }
                    }
                }

                self.has_header = false;
                result?;
                continue 'next;
            }

            return Ok(());
        }
    }

    /// Process client.
    pub fn tick(&mut self) -> Result<()> {
        for (_, node) in &mut self.client_nodes {
            if let Some(a) = &node.activation {
                if crate::ptr::volatile!(a, status).read() == consts::ActivationStatus::INACTIVE {
                    tracing::warn!("node is not active");
                    self.c.client_node_set_active(node.id, true)?;
                    crate::ptr::volatile!(a, status).write(consts::ActivationStatus::FINISHED);
                }

                // std::dbg!(unsafe { a.read() });
            }
        }

        Ok(())
    }

    /// Handle read on custom token.
    #[tracing::instrument(skip(self))]
    pub fn handle_read(&mut self, token: Token) -> Result<()> {
        let Some(index) = self.read_to_client.get(&token) else {
            tracing::warn!(?token, "No client found for token");
            return Ok(());
        };

        let Some(client) = self.client_nodes.get_mut(*index) else {
            tracing::warn!(?index, "No client found for index");
            return Ok(());
        };

        tracing::info!(client.id);
        Ok(())
    }

    /// Take a file descriptor from the stored range.
    fn take_fd(&mut self, fd: Fd) -> Result<Option<OwnedFd>> {
        if fd.fd() < 0 {
            return Ok(None);
        }

        let Ok(index) = usize::try_from(fd.fd()) else {
            bail!("Received file descriptor with invalid index: {fd:?}");
        };

        if index >= self.header.n_fds() as usize {
            bail!(
                "Received file descriptor out of range 0-{}: {fd:?}",
                self.header.n_fds()
            );
        }

        let Some(received) = self.fds.get_mut(index) else {
            bail!(
                "Received file descriptor not in stored range 0-{}: {fd:?}",
                self.fds.len()
            );
        };

        let Some(fd) = received.fd.take() else {
            bail!("Received file descriptor already taken: {fd:?}");
        };

        Ok(Some(fd))
    }

    #[tracing::instrument(skip_all, ret(level = Level::DEBUG))]
    fn op_construct_node(&mut self) -> Result<()> {
        let Some(registry) = self
            .factories
            .get("client-node")
            .and_then(|&id| self.registries.get(id))
        else {
            bail!("No factory for client-node");
        };

        let Some(type_name) = registry.properties.get("factory.type.name") else {
            bail!("No factory type name for client-node");
        };

        let Some(version) = registry
            .properties
            .get("factory.type.version")
            .and_then(|version| str::parse::<u32>(version).ok())
        else {
            bail!("No factory type version for client-node");
        };

        let new_id = self.ids.alloc().context("ran out of identifiers")?;

        self.c
            .core_create_object("client-node", type_name, version, new_id)?;

        let mut ports = Ports::new();

        let port = ports.insert(consts::Direction::INPUT)?;
        port.name = String::from("input");

        // let port = ports.insert(consts::Direction::OUTPUT)?;
        // port.name = String::from("output");

        let write_token = Token::new(self.tokens.alloc().context("no more tokens")? as u64);
        let read_token = Token::new(self.tokens.alloc().context("no more tokens")? as u64);

        let index = self.client_nodes.insert(ClientNodeState::new(
            new_id,
            ports,
            write_token,
            read_token,
        )?);

        self.local_id_to_kind
            .insert(new_id, Kind::ClientNode(index));
        Ok(())
    }

    fn core(&mut self, pod: Pod<&[u64]>) -> Result<()> {
        match self.header.op() {
            op::CORE_INFO_EVENT => {
                self.core_info_event(pod).context("Core::Info")?;
            }
            op::CORE_DONE_EVENT => {
                self.core_done_event(pod).context("Core::Done")?;
            }
            op::CORE_PING_EVENT => {
                self.core_ping_event(pod).context("Core::Ping")?;
            }
            op::CORE_ERROR_EVENT => {
                self.core_error_event(pod).context("Core::Error")?;
            }
            op::CORE_BOUND_ID_EVENT => {
                self.core_bound_id_event(pod).context("Core::BoundId")?;
            }
            op::CORE_ADD_MEM_EVENT => {
                self.core_add_mem_event(pod).context("Core::AddMem")?;
            }
            op::CORE_DESTROY_EVENT => {
                self.core_destroy(pod).context("Core::Destroy")?;
            }
            op => {
                tracing::warn!(op, "Core unsupported op");
            }
        }

        Ok(())
    }

    fn client(&mut self, pod: Pod<&[u64]>) -> Result<()> {
        match self.header.op() {
            op::CLIENT_INFO_EVENT => {
                self.client_info(pod).context("Client::Info")?;
            }
            op::CLIENT_ERROR_EVENT => {
                self.client_error(pod).context("Client::Error")?;
            }
            op => {
                tracing::warn!(op, "Client unsupported op");
            }
        }

        Ok(())
    }

    fn dynamic(&mut self, pod: Pod<&[u64]>) -> Result<()> {
        let Some(kind) = self.local_id_to_kind.get(&self.header.id()) else {
            tracing::warn!(?self.header, "Unknown receiver");
            return Ok(());
        };

        match *kind {
            Kind::Registry => match self.header.op() {
                op::REGISTRY_GLOBAL_EVENT => {
                    self.registry_global(pod).context("Registry::Global")?;
                }
                op::REGISTRY_GLOBAL_REMOVE_EVENT => {
                    self.registry_global_remove(pod)
                        .context("Registry::GlobalRemove")?;
                }
                op => {
                    tracing::warn!(op, "Registry unsupported op");
                }
            },
            Kind::ClientNode(index) => match self.header.op() {
                op::CLIENT_NODE_TRANSPORT_EVENT => {
                    self.client_node_transport(index, pod)
                        .context("ClientNode::Transport")?;
                }
                op::CLIENT_NODE_SET_PARAM_EVENT => {
                    self.client_node_set_param(index, pod)
                        .context("ClientNode::SetParam")?;
                }
                op::CLIENT_NODE_SET_IO_EVENT => {
                    self.client_node_set_io(index, pod)
                        .context("ClientNode::SetIO")?;
                }
                op::CLIENT_NODE_COMMAND_EVENT => {
                    self.client_node_command(index, pod)
                        .context("ClientNode::Command")?;
                }
                op::CLIENT_NODE_PORT_SET_PARAM_EVENT => {
                    self.client_node_port_set_param(index, pod)
                        .context("ClientNode::PortSetParam")?;
                }
                op::CLIENT_NODE_USE_BUFFERS_EVENT => {
                    self.client_node_use_buffers(index, pod)
                        .context("ClientNode::UseBuffers")?;
                }
                op::CLIENT_NODE_PORT_SET_IO_EVENT => {
                    self.client_node_port_set_io(index, pod)
                        .context("ClientNode::PortSetIO")?;
                }
                op::CLIENT_NODE_SET_ACTIVATION_EVENT => {
                    self.client_node_set_activation(index, pod)
                        .context("ClientNode::SetActivation")?;
                }
                op::CLIENT_NODE_PORT_SET_MIX_INFO_EVENT => {
                    self.client_node_set_mix_info(index, pod)
                        .context("ClientNode::SetMixInfo")?;
                }
                op => {
                    tracing::warn!(op, "Client node unsupported op");
                }
            },
        }

        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn core_info_event(&mut self, pod: Pod<&[u64]>) -> Result<()> {
        let mut st = pod.next_struct()?;
        let id = st.field()?.next::<u32>()?;
        let cookie = st.field()?.next::<i32>()?;
        let user_name = st.field()?.next_unsized::<str, _>(str::to_owned)?;
        let host_name = st.field()?.next_unsized::<str, _>(str::to_owned)?;
        let version = st.field()?.next_unsized::<str, _>(str::to_owned)?;
        let name = st.field()?.next_unsized::<str, _>(str::to_owned)?;
        let change_mask = st.field()?.next::<u64>()?;

        let mut props = st.field()?.next_struct()?;

        if change_mask & 0x1 != 0 {
            let n_items = props.field()?.next::<i32>()?;

            for _ in 0..n_items {
                let key = props.field()?.next_unsized::<str, _>(str::to_owned)?;
                let value = props.field()?.next_unsized::<str, _>(str::to_owned)?;
                self.core.properties.insert(key, value);
            }
        }

        self.core.id = id;
        self.core.cookie = cookie;
        self.core.user_name = user_name;
        self.core.host_name = host_name;
        self.core.version = version;
        self.core.name = name;
        self.ops.push_back(Op::GetRegistry);
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn core_done_event(&mut self, pod: Pod<&[u64]>) -> Result<()> {
        let (id, seq) = pod.next_struct()?.decode::<(u32, u32)>()?;

        match id {
            GET_REGISTRY_SYNC => {
                self.ops.push_back(Op::ConstructNode);
                tracing::info!(id, seq, "Intitial registry sync done");
            }
            CREATE_CLIENT_NODE => {
                tracing::info!(id, seq, "Client node created");
            }
            id => {
                tracing::warn!(id, seq, "Unknown core done event id");
            }
        }

        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn core_ping_event(&mut self, pod: Pod<&[u64]>) -> Result<()> {
        let mut st = pod.next_struct()?;
        let id = st.field()?.next()?;
        let seq = st.field()?.next()?;

        tracing::debug!("Core ping {id} with seq {seq}");
        self.ops.push_back(Op::Pong { id, seq });
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn core_error_event(&mut self, pod: Pod<&[u64]>) -> Result<()> {
        let mut st = pod.next_struct()?;
        let id = st.field()?.next::<i32>()?;
        let seq = st.field()?.next::<i32>()?;
        let res = st.field()?.next::<i32>()?;
        let error = st.field()?.next_unsized::<str, _>(str::to_owned)?;

        tracing::error!(id, seq, res, error);
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn core_bound_id_event(&mut self, pod: Pod<&[u64]>) -> Result<()> {
        let mut st = pod.next_struct()?;
        let local_id = st.field()?.next::<u32>()?;
        let global_id = st.field()?.next::<u32>()?;
        self.globals.insert(local_id, global_id);

        tracing::debug!(local_id, global_id);
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn core_add_mem_event(&mut self, pod: Pod<&[u64]>) -> Result<()> {
        let (id, ty, fd, flags) = pod
            .next_struct()?
            .decode::<(u32, id::DataType, Fd, flags::MemBlock)>()?;

        let fd = self.take_fd(fd)?;

        let Some(fd) = fd else {
            self.memory.remove(id);
            return Ok(());
        };

        self.memory.insert(id, ty, fd, flags)?;
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn core_destroy(&mut self, pod: Pod<&[u64]>) -> Result<()> {
        let mut st = pod.next_struct()?;
        let id = st.field()?.next::<u32>()?;

        tracing::debug!(id);
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn client_info(&mut self, pod: Pod<&[u64]>) -> Result<()> {
        let mut st = pod.next_struct()?;
        let id = st.field()?.next::<u32>()?;
        let change_mask = st.field()?.next::<u64>()?;

        let mut props = st.field()?.next_struct()?;

        if change_mask & 0x1 != 0 {
            let n_items = props.field()?.next::<i32>()?;

            for _ in 0..n_items {
                let key = props.field()?.next_unsized::<str, _>(str::to_owned)?;
                let value = props.field()?.next_unsized::<str, _>(str::to_owned)?;
                self.client.properties.insert(key, value);
            }
        }

        self.client.id = id;
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn client_error(&mut self, pod: Pod<&[u64]>) -> Result<()> {
        let mut st = pod.next_struct()?;
        let id = st.field()?.next::<i32>()?;
        let res = st.field()?.next::<i32>()?;
        let error = st.field()?.next_unsized::<str, _>(str::to_owned)?;
        tracing::error!(id, res, error, "Client errored");
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn registry_global(&mut self, pod: Pod<&[u64]>) -> Result<()> {
        let (id, permissions, ty, version, mut props) =
            pod.decode::<Struct<_>>()?
                .decode::<(_, _, _, _, Struct<_>)>()?;

        let n_items = props.decode::<u32>()?;

        let index = self.registries.vacant_key();
        let mut registry = RegistryState::default();

        registry.id = id;
        registry.permissions = permissions;
        registry.ty = ty;
        registry.version = version;

        for _ in 0..n_items {
            let (key, value) = props.decode::<(&str, &str)>()?;
            registry.properties.insert(key.to_owned(), value.to_owned());
        }

        if registry.ty == consts::INTERFACE_FACTORY {
            if let Some(name) = registry.properties.get("factory.name") {
                self.factories.insert(name.clone(), index);
            }
        }

        tracing::trace!(id, ?registry, "Registry global event");

        self.id_to_registry.insert(id, index);
        self.registries.insert(registry);

        if let Some(kind) = self
            .globals
            .by_global(id)
            .and_then(|local_id| self.local_id_to_kind.get_mut(&local_id))
        {
            match *kind {
                Kind::Registry => {}
                Kind::ClientNode(index) => {
                    self.ops.push_back(Op::NodeUpdate { client: index });
                }
            }
        }

        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn registry_global_remove(&mut self, pod: Pod<&[u64]>) -> Result<()> {
        let mut st = pod.next_struct()?;
        let id = st.field()?.next::<u32>()?;

        let Some(registry_index) = self.id_to_registry.remove(&id) else {
            tracing::warn!(id, "Tried to remove unknown registry");
            return Ok(());
        };

        let Some(registry) = self.registries.try_remove(registry_index) else {
            tracing::warn!(registry_index, "Tried to remove unknown registry index");
            return Ok(());
        };

        tracing::debug!(?registry, "Removed registry");

        if let Some(local_id) = self.globals.remove_by_global(id) {
            self.ids.unset(local_id);

            if let Some(kind) = self.local_id_to_kind.remove(&local_id) {
                match kind {
                    Kind::Registry => {}
                    Kind::ClientNode(index) => {
                        let Some(..) = self.client_nodes.try_remove(index) else {
                            tracing::warn!(index, "Tried to remove unknown client node");
                            return Ok(());
                        };

                        tracing::info!(index, "Removed client node");
                    }
                }
            }
        }

        Ok(())
    }

    #[tracing::instrument(skip(self, pod))]
    fn client_node_transport(&mut self, index: usize, pod: Pod<&[u64]>) -> Result<()> {
        let mut st = pod.next_struct()?;
        let read_fd = self.take_fd(st.field()?.next::<Fd>()?)?;
        let write_fd = self.take_fd(st.field()?.next::<Fd>()?)?;
        let mem_id = st.field()?.next::<i32>()?;
        let offset = st.field()?.next::<isize>()?;
        let size = st.field()?.next::<usize>()?;

        let Some(node) = self.client_nodes.get_mut(index) else {
            bail!("Missing client node {index}");
        };

        if let Some(a) = node.activation.take() {
            self.memory.free(a);
        }

        let Ok(mem_id) = u32::try_from(mem_id) else {
            return Ok(());
        };

        let region = self.memory.map(mem_id, offset, size)?;

        if let Some(a) = node.activation.replace(region) {
            self.memory.free(a);
        }

        tracing::debug!(index, ?read_fd, ?write_fd, mem_id, offset, size,);

        let Some(node) = self.client_nodes.get_mut(index) else {
            bail!("Missing client node {index}");
        };

        node.read_fd = read_fd;
        node.write_fd = write_fd;

        if node.read_fd.is_some() {
            self.ops.push_back(Op::ReadInterest { client: index });
        }

        Ok(())
    }

    #[tracing::instrument(skip(self, pod))]
    fn client_node_set_param(&mut self, index: usize, pod: Pod<&[u64]>) -> Result<()> {
        let Some(node) = self.client_nodes.get_mut(index) else {
            bail!("Missing client node {index}");
        };

        let mut st = pod.next_struct()?;
        let id = st.field()?.next::<id::Param>()?;
        let _flags = st.field()?.next::<i32>()?;

        if let Some(obj) = st.field()?.next_option()? {
            tracing::trace!(?id, "set");
            node.set_param(id, obj.next_object()?.to_owned());
        } else {
            tracing::trace!(?id, "remove");
            node.remove_param(id);
        }

        self.ops.push_back(Op::NodeUpdate { client: index });
        Ok(())
    }

    #[tracing::instrument(skip(self, pod))]
    fn client_node_set_io(&mut self, index: usize, pod: Pod<&[u64]>) -> Result<()> {
        let Some(node) = self.client_nodes.get_mut(index) else {
            bail!("Missing client node {index}");
        };

        let mut st = pod.next_struct()?;
        let id = st.field()?.next::<id::IoType>()?;
        let mem_id = st.field()?.next::<i32>()?;
        let offset = st.field()?.next::<isize>()?;
        let size = st.field()?.next::<usize>()?;

        match id {
            id::IoType::CONTROL => {
                let Ok(mem_id) = u32::try_from(mem_id) else {
                    if let Some(region) = node.io_control.take() {
                        self.memory.free(region);
                    }

                    return Ok(());
                };

                let region = self.memory.map(mem_id, offset, size)?;

                if let Some(region) = node.io_control.replace(region) {
                    self.memory.free(region);
                }
            }
            id::IoType::CLOCK => {
                let Ok(mem_id) = u32::try_from(mem_id) else {
                    if let Some(region) = node.io_clock.take() {
                        self.memory.free(region);
                    }

                    return Ok(());
                };

                let region = self.memory.map(mem_id, offset, size)?;

                if let Some(region) = node.io_clock.replace(region) {
                    self.memory.free(region);
                }
            }
            id::IoType::POSITION => {
                let Ok(mem_id) = u32::try_from(mem_id) else {
                    if let Some(region) = node.io_position.take() {
                        self.memory.free(region);
                    }

                    return Ok(());
                };

                let region = self.memory.map(mem_id, offset, size)?;

                if let Some(region) = node.io_position.replace(region) {
                    self.memory.free(region);
                }
            }
            _ => {
                tracing::warn!(?id, "Unsupported IO type in set IO");
                return Ok(());
            }
        }

        Ok(())
    }

    #[tracing::instrument(skip(self, pod))]
    fn client_node_command(&mut self, index: usize, pod: Pod<&[u64]>) -> Result<()> {
        let Some(node) = self.client_nodes.get_mut(index) else {
            bail!("Missing client node {index}");
        };

        let mut st = pod.as_ref().next_struct()?;
        let obj = st.field()?.next_object()?;

        let object_type = id::CommandType::from_id(obj.object_type());
        let object_id = id::NodeCommand::from_id(obj.object_id());

        match object_id {
            id::NodeCommand::START => {
                for (_, activation) in &node.node_activations {
                    // activation.signal()?;
                }
            }
            _ => {
                tracing::info!(?object_type, ?object_id, ?pod);
            }
        }

        Ok(())
    }

    #[tracing::instrument(skip(self, pod))]
    fn client_node_port_set_param(&mut self, index: usize, pod: Pod<&[u64]>) -> Result<()> {
        let Some(node) = self.client_nodes.get_mut(index) else {
            bail!("Missing client node {index}");
        };

        let mut st = pod.next_struct()?;
        let direction = consts::Direction::from_raw(st.field()?.next::<u32>()?);
        let port_id = st.field()?.next::<u32>()?;
        let id = st.field()?.next::<id::Param>()?;
        let flags = st.field()?.next::<u32>()?;

        let port = node.ports.get_mut(direction, port_id)?;

        if let Some(param) = st.field()?.next_option()? {
            tracing::trace!(?id, "set");
            port.set_param(id, param.next_object()?.to_owned(), flags)?;
        } else {
            tracing::trace!(?id, "remove");
            port.remove_param(id)?;
        }

        self.ops.push_back(Op::NodeUpdate { client: index });
        Ok(())
    }

    #[tracing::instrument(skip(self, pod))]
    fn client_node_use_buffers(&mut self, index: usize, pod: Pod<&[u64]>) -> Result<()> {
        let Some(node) = self.client_nodes.get_mut(index) else {
            bail!("Missing client node {index}");
        };

        let mut st = pod.next_struct()?;

        let (direction, port_id, mix_id, flags, n_buffers) =
            st.decode::<(consts::Direction, u32, u32, u32, u32)>()?;

        let mut buffers = Vec::new();

        for _ in 0..n_buffers {
            let (mem_id, offset, size, n_metas) = st.decode::<(u32, i32, u32, u32)>()?;
            let region = self
                .memory
                .map(mem_id, offset as isize, size as usize)
                .context("mapping buffer")?;

            let mut metas = Vec::new();

            for _ in 0..n_metas {
                let (ty, size) = st.decode::<(id::MetaType, u32)>()?;
                metas.push(buffer::Meta { ty, size });
            }

            let mut datas = Vec::new();

            let n_datas = st.decode::<u32>()?;

            for _ in 0..n_datas {
                let (ty, data, flags, offset, max_size) =
                    st.decode::<(id::DataType, u32, flags::DataFlag, i32, u32)>()?;

                let Ok(max_size) = usize::try_from(max_size) else {
                    bail!("Invalid max size {max_size} for data type {ty:?}");
                };

                let region = match ty {
                    id::DataType::MEM_PTR => {
                        let Some(region) = region.offset(data as usize) else {
                            bail!("Invalid memory pointer {data} for region {region:?}");
                        };

                        assert!(region.size <= max_size);
                        assert!(offset == 0);

                        self.memory.track(&region);
                        region
                    }
                    id::DataType::MEM_FD => self.memory.map(data, offset as isize, max_size)?,
                    ty => {
                        bail!("Unsupported data type {ty:?} in use buffers");
                    }
                };

                datas.push(buffer::Data {
                    ty,
                    region,
                    flags,
                    max_size,
                });
            }

            buffers.push(Buffer {
                mem_id,
                offset,
                size,
                metas,
                datas,
            });
        }

        let buffers = Buffers {
            direction,
            mix_id,
            flags,
            buffers,
        };

        let replaced = node
            .ports
            .get_mut(direction, port_id)?
            .replace_buffers(buffers);

        if let Some(replaced) = replaced {
            for buffer in replaced.buffers {
                for data in buffer.datas {
                    self.memory.free(data.region);
                }
            }
        }

        Ok(())
    }

    fn client_node_port_set_io(&mut self, index: usize, pod: Pod<&[u64]>) -> Result<()> {
        let Some(node) = self.client_nodes.get_mut(index) else {
            bail!("Missing client node {index}");
        };

        let mut st = pod.next_struct()?;
        let direction = consts::Direction::from_raw(st.field()?.next::<u32>()?);
        let port_id = st.field()?.next::<u32>()?;
        let mix_id = st.field()?.next::<u32>()?;
        let id = st.field()?.next::<id::IoType>()?;
        let mem_id = st.field()?.next::<i32>()?;
        let offset = st.field()?.next::<isize>()?;
        let size = st.field()?.next::<usize>()?;
        let port = node.ports.get_mut(direction, port_id)?;

        let span = tracing::info_span!("client_node_port_set_io", ?direction, port_id, ?id,);
        let _span = span.enter();

        match id {
            id::IoType::CLOCK => {
                let Ok(mem_id) = u32::try_from(mem_id) else {
                    if let Some(region) = port.io_clock.take() {
                        self.memory.free(region);
                    };

                    return Ok(());
                };

                let region = self.memory.map(mem_id, offset, size)?;

                if let Some(region) = port.io_clock.replace(region) {
                    self.memory.free(region);
                }
            }
            id::IoType::POSITION => {
                let Ok(mem_id) = u32::try_from(mem_id) else {
                    if let Some(region) = port.io_position.take() {
                        self.memory.free(region);
                    };

                    return Ok(());
                };

                let region = self.memory.map(mem_id, offset, size)?;

                if let Some(region) = port.io_position.replace(region) {
                    self.memory.free(region);
                }
            }
            id::IoType::BUFFERS => {
                let Ok(mem_id) = u32::try_from(mem_id) else {
                    if let Some(region) = port.io_buffers.take() {
                        self.memory.free(region);
                    };

                    return Ok(());
                };

                let region = self.memory.map(mem_id, offset, size)?;

                if let Some(region) = port.io_buffers.replace(region) {
                    self.memory.free(region);
                }
            }
            id => {
                tracing::warn!(?id, "Unsupported IO type in port set IO");
                return Ok(());
            }
        }

        Ok(())
    }

    #[tracing::instrument(skip(self, pod))]
    fn client_node_set_activation(&mut self, index: usize, pod: Pod<&[u64]>) -> Result<()> {
        let mut st = pod.next_struct()?;

        let peer_id = st.field()?.next::<u32>()?;
        let fd = self.take_fd(st.field()?.next::<Fd>()?)?;
        let mem_id = st.field()?.next::<i32>()?;
        let offset = st.field()?.next::<i32>()? as isize;
        let size = st.field()?.next::<u32>()? as usize;

        let Some(node) = self.client_nodes.get_mut(index) else {
            bail!("Missing client node {index}");
        };

        if let Some(index) = node.peer_to_activation.remove(&peer_id) {
            if let Some(a) = node.node_activations.try_remove(index) {
                self.memory.free(a.region);
            }
        }

        let (Some(fd), Ok(mem_id)) = (fd, u32::try_from(mem_id)) else {
            return Ok(());
        };

        let region = self.memory.map(mem_id, offset, size)?;

        let index =
            node.node_activations
                .insert(Activation::new(peer_id, EventFd::from(fd), region));
        node.peer_to_activation.insert(peer_id, index);
        Ok(())
    }

    #[tracing::instrument(skip(self, pod))]
    fn client_node_set_mix_info(&mut self, index: usize, pod: Pod<&[u64]>) -> Result<()> {
        let Some(..) = self.client_nodes.get_mut(index) else {
            bail!("Missing client node {index}");
        };

        let st = pod.next_struct()?;
        tracing::info!(?st);
        Ok(())
    }
}
