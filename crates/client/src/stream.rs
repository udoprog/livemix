use core::ffi::CStr;
use core::fmt;
use core::mem;
use core::mem::MaybeUninit;
use core::slice;

use core::time::Duration;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::ffi::CString;
use std::fs::File;
use std::os::fd::FromRawFd;
use std::os::fd::{AsRawFd, OwnedFd, RawFd};
use std::time::SystemTime;

use alloc::borrow::ToOwned;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use anyhow::{Context, Result, anyhow, bail, ensure};
use pod::AsSlice;
use pod::{ChoiceType, DynamicBuf, Fd, Object, Pod, Slice, Struct, Type};
use protocol::EventFd;
use protocol::Poll;
use protocol::Prop;
use protocol::buf::RecvBuf;
use protocol::consts::{self, Activation, Direction};
use protocol::ffi;
use protocol::flags;
use protocol::id;
use protocol::ids::IdSet;
use protocol::op::{self, ClientEvent, ClientNodeEvent, CoreEvent, RegistryEvent};
use protocol::poll::{ChangeInterest, Interest, PollEvent, Token};
use protocol::types::Header;
use protocol::{Connection, Properties, prop};
use slab::Slab;
use tracing::Level;

use crate::activation::PeerActivation;
use crate::buffer::{self, Buffer};
use crate::events::RemoveNodeParamEvent;
use crate::events::{RemovePortParamEvent, SetNodeParamEvent, SetPortParamEvent, StreamEvent};
use crate::ports::PortMix;
use crate::ports::PortParam;
use crate::ptr::{atomic, volatile};
use crate::{
    Buffers, Client, ClientNode, ClientNodeId, ClientNodes, Memory, MixId, PortId, Ports, Region,
};

const CREATE_CLIENT_NODE: i32 = 0x2000;
const GET_REGISTRY_SYNC: i32 = 0x1000;

macro_rules! tracing_error {
    ($error:expr, $($tt:tt)*) => {{
        tracing::error!(error = ?$error, $($tt)*);

        for error in $error.chain().skip(1) {
            tracing::error!(?error, "Caused by");
        }
    }};
}

/// The local connection state.
pub struct Stream {
    tick: usize,
    c: Client,
    connection_added: bool,
    connection_token: Token,
    core: CoreState,
    client: ClientState,
    registries: Slab<RegistryState>,
    id_to_registry: BTreeMap<u32, usize>,
    factories: BTreeMap<String, usize>,
    globals: GlobalMap,
    client_nodes: ClientNodes,
    local_id_to_kind: BTreeMap<u32, Kind>,
    has_header: bool,
    header: Header,
    ids: IdSet,
    tokens: IdSet,
    process_set: IdSet,
    read_to_client: HashMap<Token, ClientNodeId>,
    write_to_client: HashMap<Token, ClientNodeId>,
    fds: Vec<ReceivedFd>,
    ops: VecDeque<Op>,
    memory: Memory,
    add_interest: VecDeque<(RawFd, Token, Interest)>,
    modify_interest: VecDeque<(RawFd, Token, Interest)>,
}

impl Stream {
    pub fn new(connection: Connection) -> Result<Self> {
        let mut ids = IdSet::new();

        // Well-known identifiers.
        ids.set(consts::CORE_ID);
        ids.set(consts::CLIENT_ID);

        let mut client = ClientState::default();

        client
            .properties
            .insert(prop::APPLICATION_NAME, String::from("livemix"));

        client
            .properties
            .insert(prop::NODE_NAME, String::from("livemix_node"));

        let mut tokens = IdSet::new();
        let connection_token = Token::new(tokens.alloc().context("no more tokens")? as u64);

        Ok(Self {
            tick: 0,
            c: Client::new(connection),
            connection_added: false,
            connection_token,
            core: CoreState::default(),
            client,
            registries: Slab::new(),
            id_to_registry: BTreeMap::new(),
            factories: BTreeMap::new(),
            globals: GlobalMap::new(),
            client_nodes: ClientNodes::new(),
            local_id_to_kind: BTreeMap::new(),
            has_header: false,
            header: Header::default(),
            ids,
            tokens,
            process_set: IdSet::new(),
            read_to_client: HashMap::new(),
            write_to_client: HashMap::new(),
            fds: Vec::with_capacity(16),
            ops: VecDeque::from([Op::CoreHello]),
            memory: Memory::new(),
            add_interest: VecDeque::new(),
            modify_interest: VecDeque::new(),
        })
    }

    /// Get a node.
    pub fn node(&self, node_id: ClientNodeId) -> Result<&ClientNode> {
        self.client_nodes.get(node_id)
    }

    /// Get a mutable node.
    pub fn node_mut(&mut self, node_id: ClientNodeId) -> Result<&mut ClientNode> {
        self.client_nodes.get_mut(node_id)
    }

    /// Iterate over nodes.
    pub fn nodes(&mut self) -> impl Iterator<Item = &ClientNode> {
        self.client_nodes.iter()
    }

    /// Iterate over nodes mutably.
    pub fn nodes_mut(&mut self) -> impl Iterator<Item = &mut ClientNode> {
        self.client_nodes.iter_mut()
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
        if !self.connection_added {
            self.connection_added = true;
            return Some((self.c.as_raw_fd(), self.connection_token, self.c.interest()));
        }

        self.add_interest.pop_front()
    }

    #[inline]
    pub fn modify_interest(&mut self) -> Option<(RawFd, Token, Interest)> {
        if let ChangeInterest::Changed(interest) = self.c.modify_interest() {
            return Some((self.c.as_raw_fd(), self.connection_token, interest));
        }

        if let Some((fd, token, interest)) = self.modify_interest.pop_front() {
            return Some((fd, token, interest));
        }

        None
    }

    /// Add file descriptors.
    #[tracing::instrument(skip(self, fds))]
    pub fn add_fds(&mut self, fds: impl IntoIterator<Item = OwnedFd>) {
        let mut added = 0usize;

        for fd in fds {
            self.fds.push(ReceivedFd { fd: Some(fd) });
            added += 1;
        }

        if added > 0 {
            tracing::trace!(added, fds = ?self.fds);
        }
    }

    #[tracing::instrument(skip(self))]
    fn process_operations(&mut self) -> Result<Option<StreamEvent>> {
        while let Some(op) = self.ops.pop_front() {
            tracing::trace!(?op);

            match op {
                Op::CoreHello => {
                    self.c.core_hello()?;
                    self.c.client_update_properties(&self.client.properties)?;
                }
                Op::GetRegistry => {
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
                Op::NodeCreated { node_id } => {
                    let node = self.client_nodes.get(node_id)?;
                    self.c.client_node_set_active(node.id, true)?;
                    return Ok(Some(StreamEvent::NodeCreated(node_id)));
                }
                Op::NodeUpdate { node_id, what } => {
                    let node = self.client_nodes.get_mut(node_id)?;

                    if node.take_modified() {
                        self.c.client_node_update(node.id, 4, 4, &node.params)?;
                    }

                    for port in node.ports.inputs_mut() {
                        if !port.take_modified() {
                            continue;
                        }

                        self.c.client_node_port_update(
                            node.id,
                            Direction::INPUT,
                            port.id,
                            &port.name,
                            port.param_values(),
                            port.param_flags(),
                        )?;
                    }

                    for port in node.ports.outputs_mut() {
                        if !port.take_modified() {
                            continue;
                        }

                        self.c.client_node_port_update(
                            node.id,
                            Direction::OUTPUT,
                            port.id,
                            &port.name,
                            port.param_values(),
                            port.param_flags(),
                        )?;
                    }

                    if let Some(what) = what {
                        let ev = match what {
                            NodeUpdateWhat::SetNodeParam(param) => {
                                StreamEvent::SetNodeParam(SetNodeParamEvent { node_id, param })
                            }
                            NodeUpdateWhat::RemoveNodeParam(param) => {
                                StreamEvent::RemoveNodeParam(RemoveNodeParamEvent {
                                    node_id,
                                    param,
                                })
                            }
                            NodeUpdateWhat::SetPortParam(direction, port_id, param) => {
                                StreamEvent::SetPortParam(SetPortParamEvent {
                                    node_id,
                                    direction,
                                    port_id,
                                    param,
                                })
                            }
                            NodeUpdateWhat::RemovePortParam(direction, port_id, param) => {
                                StreamEvent::RemovePortParam(RemovePortParamEvent {
                                    node_id,
                                    direction,
                                    port_id,
                                    param,
                                })
                            }
                        };

                        return Ok(Some(ev));
                    }
                }
                Op::NodeStart { node_id } => {
                    let node = self.client_nodes.get_mut(node_id)?;

                    let Some(a) = &mut node.activation else {
                        continue;
                    };

                    let was_inactive = unsafe {
                        atomic!(a, status)
                            .compare_exchange(Activation::INACTIVE, Activation::FINISHED)
                    };

                    if was_inactive {
                        let state = unsafe { volatile!(a, state[0]).read() };
                        let client_version = unsafe { volatile!(a, client_version).read() };
                        tracing::info!(?state, ?client_version, "Starting node");
                    }
                }
                Op::NodePause { node_id } => {
                    let node = self.client_nodes.get_mut(node_id)?;

                    if let Some(a) = &mut node.activation {
                        unsafe { atomic!(a, status).store(Activation::INACTIVE) };
                    } else {
                        tracing::error!(
                            ?node_id,
                            "Cannot pause node, missing activation for client"
                        );
                    }
                }
                Op::NodeReadInterest { node_id } => {
                    self.node_read_interest(node_id)?;
                }
            }
        }

        Ok(None)
    }

    #[tracing::instrument(skip(self, recv))]
    fn process_messages(&mut self, recv: &mut RecvBuf) -> Result<bool> {
        if !self.has_header {
            if let Some(h) = recv.read::<Header>() {
                self.header = h;
                self.has_header = true;
            }
        }

        if !self.has_header {
            return Ok(false);
        }

        if (self.header.n_fds() as usize) > self.fds.len() {
            return Ok(false);
        }

        let Some(pod) = frame(recv, &self.header)? else {
            return Ok(false);
        };

        let st = pod.read_struct()?;

        let result = match self.header.id() {
            consts::CORE_ID => self.core(st),
            consts::CLIENT_ID => self.client(st),
            _ => self.dynamic(st),
        };

        if self.header.n_fds() > 0 {
            let n_fds = self.header.n_fds() as usize;

            ensure!(
                n_fds <= self.fds.len(),
                "Header specifies more file descriptors ({n_fds}) than is stored ({})",
                self.fds.len()
            );

            if n_fds > 0 {
                for fd in self.fds.drain(..n_fds) {
                    if let Some(fd) = fd.fd {
                        tracing::warn!("Unused file descriptor dropped: {fd:?}");
                    }
                }

                tracing::trace!(n_fds, fds_after = ?self.fds, "Freed file descriptors");
            }
        }

        self.has_header = false;
        result?;
        Ok(true)
    }

    /// Process client.
    #[tracing::instrument(skip(self, poll, recv))]
    pub fn run(&mut self, poll: &mut Poll, recv: &mut RecvBuf) -> Result<Option<StreamEvent>> {
        loop {
            if let Some(ev) = self.process_operations()? {
                return Ok(Some(ev));
            }

            if !self.process_messages(recv)? {
                break;
            }
        }

        if let Some(raw_id) = self.process_set.take_next() {
            return Ok(Some(StreamEvent::Process(ClientNodeId::new(raw_id))));
        }

        while let Some((fd, token, interest)) = self.add_interest() {
            tracing::trace!(?fd, ?token, ?interest, "Adding interest");
            poll.add(fd, token, interest)?;
        }

        while let Some((fd, token, interest)) = self.modify_interest() {
            tracing::trace!(?fd, ?token, ?interest, "Modifying interest");
            poll.modify(fd, token, interest)?;
        }

        Ok(None)
    }

    #[tracing::instrument(skip(self))]
    pub fn drive(&mut self, recv: &mut RecvBuf, e: PollEvent) -> Result<()> {
        if e.token == self.connection_token {
            tracing::trace!(?e.interest, "connection");

            if e.interest.is_read() {
                let mut fds = [0; 32];

                let n_fds = self
                    .c
                    .recv_with_fds(recv, &mut fds[..])
                    .context("Receive errored")?;

                // SAFETY: We must trust the file descriptor we have
                // just received.
                let iter = fds[..n_fds]
                    .iter_mut()
                    .map(|fd| unsafe { OwnedFd::from_raw_fd(mem::take(fd)) });

                self.add_fds(iter);
            }

            if e.interest.is_write() {
                self.c.send()?;
            }

            return Ok(());
        }

        if e.interest.is_read() {
            self.handle_read(e.token)?;
            return Ok(());
        }

        Ok(())
    }

    /// Handle read on custom token.
    #[tracing::instrument(skip(self, token))]
    pub fn handle_read(&mut self, token: Token) -> Result<()> {
        let Some(node_id) = self.read_to_client.get(&token) else {
            tracing::warn!(?token, "Got read for unknown token");
            return Ok(());
        };

        let node = self.client_nodes.get_mut(*node_id)?;

        let Some(read_fd) = &node.read_fd else {
            bail!("No read file descriptor for client");
        };

        let Some(ev) = read_fd.read()? else {
            return Ok(());
        };

        self.process_set.set(node_id.into_u32());
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

    #[tracing::instrument(skip_all, ret(level = Level::TRACE))]
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

        let write_token = Token::new(self.tokens.alloc().context("no more tokens")? as u64);
        let read_token = Token::new(self.tokens.alloc().context("no more tokens")? as u64);

        let node_id =
            self.client_nodes
                .insert(ClientNode::new(new_id, ports, write_token, read_token)?);

        self.local_id_to_kind
            .insert(new_id, Kind::ClientNode(node_id));

        self.ops.push_back(Op::NodeCreated { node_id: node_id });
        Ok(())
    }

    fn node_read_interest(&mut self, node_id: ClientNodeId) -> Result<()> {
        let node = self.client_nodes.get(node_id)?;

        if let Some(read_fd) = &node.read_fd {
            self.read_to_client.insert(node.read_token, node_id);
            self.add_interest.push_back((
                read_fd.as_raw_fd(),
                node.read_token,
                Interest::READ | Interest::HUP | Interest::ERROR,
            ));
        }

        if let Some(write_fd) = &node.write_fd {
            self.write_to_client.insert(node.write_token, node_id);
            self.add_interest.push_back((
                write_fd.as_raw_fd(),
                node.write_token,
                Interest::HUP | Interest::ERROR,
            ));
        }

        Ok(())
    }

    fn core(&mut self, mut st: Struct<Slice<'_>>) -> Result<()> {
        let op = CoreEvent::from_raw(self.header.op());
        tracing::trace!("Event: {op}");

        match op {
            CoreEvent::INFO => {
                self.core_info_event(st).context(op)?;
            }
            CoreEvent::DONE => {
                self.core_done_event(st).context(op)?;
            }
            CoreEvent::PING => {
                self.core_ping_event(st).context(op)?;
            }
            CoreEvent::ERROR => {
                self.core_error_event(st).context(op)?;
            }
            CoreEvent::BOUND_ID => {
                self.core_bound_id_event(st).context(op)?;
            }
            CoreEvent::ADD_MEM => {
                self.core_add_mem_event(st).context(op)?;
            }
            CoreEvent::DESTROY => {
                self.core_destroy(st).context(op)?;
            }
            op => {
                tracing::warn!("Unsupported event: {op}");
            }
        }

        Ok(())
    }

    fn client(&mut self, mut st: Struct<Slice<'_>>) -> Result<()> {
        let op = ClientEvent::from_raw(self.header.op());

        match op {
            ClientEvent::INFO => {
                self.client_info(st).context(op)?;
            }
            ClientEvent::ERROR => {
                self.client_error(st).context(op)?;
            }
            op => {
                tracing::warn!("Unsupported event: {op}");
            }
        }

        Ok(())
    }

    fn dynamic(&mut self, st: Struct<Slice<'_>>) -> Result<()> {
        let Some(kind) = self.local_id_to_kind.get(&self.header.id()) else {
            tracing::warn!(?self.header, "Unknown receiver");
            return Ok(());
        };

        match *kind {
            Kind::Registry => {
                let op = RegistryEvent::from_raw(self.header.op());
                tracing::trace!("Event: {op}");

                match op {
                    RegistryEvent::GLOBAL => {
                        self.registry_global(st).context(op)?;
                    }
                    RegistryEvent::GLOBAL_REMOVE => {
                        self.registry_global_remove(st).context(op)?;
                    }
                    op => {
                        tracing::warn!(?op, "Registry unsupported op");
                    }
                }
            }
            Kind::ClientNode(node_id) => {
                let op = ClientNodeEvent::from_raw(self.header.op());
                tracing::trace!("Event: {op}");

                match op {
                    ClientNodeEvent::TRANSPORT => {
                        self.client_node_transport(node_id, st).context(op)?;
                    }
                    ClientNodeEvent::SET_PARAM => {
                        self.client_node_set_param(node_id, st).context(op)?;
                    }
                    ClientNodeEvent::SET_IO => {
                        self.client_node_set_io(node_id, st).context(op)?;
                    }
                    ClientNodeEvent::COMMAND => {
                        self.client_node_command(node_id, st).context(op)?;
                    }
                    ClientNodeEvent::PORT_SET_PARAM => {
                        self.client_node_port_set_param(node_id, st).context(op)?;
                    }
                    ClientNodeEvent::USE_BUFFERS => {
                        self.client_node_use_buffers(node_id, st).context(op)?;
                    }
                    ClientNodeEvent::PORT_SET_IO => {
                        self.client_node_port_set_io(node_id, st).context(op)?;
                    }
                    ClientNodeEvent::SET_ACTIVATION => {
                        self.client_node_set_activation(node_id, st).context(op)?;
                    }
                    ClientNodeEvent::PORT_SET_MIX_INFO => {
                        self.client_node_set_mix_info(node_id, st).context(op)?;
                    }
                    op => {
                        tracing::warn!("Unsupported event: {op}");
                    }
                }
            }
        }

        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn core_info_event(&mut self, mut st: Struct<Slice<'_>>) -> Result<()> {
        let (id, cookie) = st.read::<(u32, i32)>()?;
        let (user_name, host_name, version, name) =
            st.read::<(String, String, String, String)>()?;
        let change_mask = st.read::<flags::CoreInfoChangeFlags>()?;

        let mut props = st.read::<Struct<_>>()?;

        if change_mask & flags::CoreInfoChangeFlags::PROPS {
            let n_items = props.read::<u32>()?;

            for _ in 0..n_items {
                let (key, value) = props.read::<(String, String)>()?;
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
    fn core_done_event(&mut self, mut st: Struct<Slice<'_>>) -> Result<()> {
        let (id, seq) = st.read::<(i32, i32)>()?;

        match id {
            GET_REGISTRY_SYNC => {
                self.ops.push_back(Op::ConstructNode);
                tracing::trace!(id, seq, "Intitial registry sync done");
            }
            CREATE_CLIENT_NODE => {
                tracing::trace!(id, seq, "Client node created");
            }
            id => {
                tracing::warn!(id, seq, "Unknown core done event id");
            }
        }

        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn core_ping_event(&mut self, mut st: Struct<Slice<'_>>) -> Result<()> {
        let id = st.field()?.read_sized()?;
        let seq = st.field()?.read_sized()?;

        tracing::debug!("Core ping {id} with seq {seq}");
        self.ops.push_back(Op::Pong { id, seq });
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn core_error_event(&mut self, mut st: Struct<Slice<'_>>) -> Result<()> {
        let id = st.field()?.read_sized::<i32>()?;
        let seq = st.field()?.read_sized::<i32>()?;
        let res = st.field()?.read_sized::<i32>()?;
        let error = st.field()?.read_unsized::<str>()?.to_owned();

        tracing::error!(id, seq, res, error);
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn core_bound_id_event(&mut self, mut st: Struct<Slice<'_>>) -> Result<()> {
        let local_id = st.field()?.read_sized::<u32>()?;
        let global_id = st.field()?.read_sized::<u32>()?;
        self.globals.insert(local_id, global_id);

        tracing::debug!(local_id, global_id);
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn core_add_mem_event(&mut self, mut st: Struct<Slice<'_>>) -> Result<()> {
        let (id, ty, fd, flags) = st.read::<(u32, id::DataType, Fd, flags::MemBlock)>()?;

        let fd = self.take_fd(fd)?;

        let Some(fd) = fd else {
            self.memory.remove(id);
            return Ok(());
        };

        self.memory.insert(id, ty, fd, flags)?;
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn core_destroy(&mut self, mut st: Struct<Slice<'_>>) -> Result<()> {
        let id = st.field()?.read_sized::<u32>()?;

        tracing::debug!(id);
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn client_info(&mut self, mut st: Struct<Slice<'_>>) -> Result<()> {
        let id = st.field()?.read_sized::<u32>()?;
        let change_mask = st.field()?.read_sized::<u64>()?;

        let mut props = st.field()?.read_struct()?;

        if change_mask & 0x1 != 0 {
            let n_items = props.field()?.read_sized::<i32>()?;

            for _ in 0..n_items {
                let key = props.field()?.read_unsized::<CStr>()?;
                let value = props.field()?.read_unsized::<CStr>()?;

                self.client
                    .server_properties
                    .insert(key.to_owned(), value.to_owned());
            }
        }

        self.client.id = id;
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn client_error(&mut self, mut st: Struct<Slice<'_>>) -> Result<()> {
        let id = st.field()?.read_sized::<i32>()?;
        let res = st.field()?.read_sized::<i32>()?;
        let error = st.field()?.read_unsized::<str>()?.to_owned();
        tracing::error!(id, res, error, "Client errored");
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn registry_global(&mut self, mut st: Struct<Slice<'_>>) -> Result<()> {
        let (id, permissions, ty, version, mut props) = st.read::<(_, _, _, _, Struct<_>)>()?;

        let n_items = props.read::<u32>()?;

        let index = self.registries.vacant_key();
        let mut registry = RegistryState::default();

        registry.id = id;
        registry.permissions = permissions;
        registry.ty = ty;
        registry.version = version;

        for _ in 0..n_items {
            let (key, value) = props.read::<(&str, &str)>()?;
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
                    self.ops.push_back(Op::NodeUpdate {
                        node_id: index,
                        what: None,
                    });
                }
            }
        }

        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn registry_global_remove(&mut self, mut st: Struct<Slice<'_>>) -> Result<()> {
        let id = st.field()?.read_sized::<u32>()?;

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
                    Kind::ClientNode(node_id) => {
                        if self.client_nodes.remove(node_id).is_none() {
                            tracing::warn!(?node_id, "Tried to remove unknown client node");
                        } else {
                            tracing::info!(?node_id, "Removed client node");
                        }
                    }
                }
            }
        }

        Ok(())
    }

    #[tracing::instrument(skip(self, st))]
    fn client_node_transport(
        &mut self,
        node_id: ClientNodeId,
        mut st: Struct<Slice<'_>>,
    ) -> Result<()> {
        let read_fd = st.field()?.read::<Fd>()?;
        let write_fd = st.field()?.read::<Fd>()?;
        let mem_id = st.field()?.read::<i32>()?;
        let offset = st.field()?.read::<usize>()?;
        let size = st.field()?.read::<usize>()?;

        let read_fd = self.take_fd(read_fd)?;
        let write_fd = self.take_fd(write_fd)?;

        let node = self.client_nodes.get_mut(node_id)?;

        if let Some(a) = node.take_activation() {
            self.memory.free(a);
        }

        let Ok(mem_id) = u32::try_from(mem_id) else {
            return Ok(());
        };

        let region = self
            .memory
            .map(mem_id, offset, size)?
            .cast::<ffi::NodeActivation>()?;

        if let Some(a) = node.replace_activation(region) {
            self.memory.free(a);
        }

        tracing::debug!(?node_id, ?read_fd, ?write_fd, mem_id, offset, size);

        node.read_fd = read_fd.map(EventFd::from);
        node.write_fd = write_fd.map(EventFd::from);

        if node.read_fd.is_some() {
            self.ops.push_back(Op::NodeReadInterest { node_id });
        }

        Ok(())
    }

    #[tracing::instrument(skip(self, st))]
    fn client_node_set_param(
        &mut self,
        node_id: ClientNodeId,
        mut st: Struct<Slice<'_>>,
    ) -> Result<()> {
        let node = self.client_nodes.get_mut(node_id)?;

        let id = st.field()?.read_sized::<id::Param>()?;
        let _flags = st.field()?.read_sized::<i32>()?;

        let what = if let Some(obj) = st.field()?.read_option()? {
            tracing::trace!(?id, "set");
            node.set_param(id, [obj.read_object()?.to_owned()?]);
            NodeUpdateWhat::SetNodeParam(id)
        } else {
            tracing::trace!(?id, "remove");
            node.remove_param(id);
            NodeUpdateWhat::RemoveNodeParam(id)
        };

        self.ops.push_back(Op::NodeUpdate {
            node_id,
            what: Some(what),
        });
        Ok(())
    }

    #[tracing::instrument(skip(self, st))]
    fn client_node_set_io(
        &mut self,
        node_id: ClientNodeId,
        mut st: Struct<Slice<'_>>,
    ) -> Result<()> {
        let node = self.client_nodes.get_mut(node_id)?;

        let id = st.field()?.read_sized::<id::IoType>()?;
        let mem_id = st.field()?.read_sized::<i32>()?;
        let offset = st.field()?.read_sized::<usize>()?;
        let size = st.field()?.read_sized::<usize>()?;

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

                let region = self.memory.map(mem_id, offset, size)?.cast()?;

                if let Some(region) = node.io_clock.replace(region) {
                    self.memory.free(region);
                }
            }
            id::IoType::POSITION => {
                if let Some(region) = node.take_io_position() {
                    self.memory.free(region);
                }

                let Ok(mem_id) = u32::try_from(mem_id) else {
                    return Ok(());
                };

                let region = self
                    .memory
                    .map(mem_id, offset, size)?
                    .cast::<ffi::IoPosition>()?;

                if let Some(region) = node.replace_io_position(region) {
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

    #[tracing::instrument(skip(self, st))]
    fn client_node_command(
        &mut self,
        node_id: ClientNodeId,
        mut st: Struct<Slice<'_>>,
    ) -> Result<()> {
        let node = self.client_nodes.get_mut(node_id)?;

        let obj = st.field()?.read_object()?;

        let object_type = id::CommandType::from_id(obj.object_type());
        let object_id = id::NodeCommand::from_id(obj.object_id());

        tracing::trace!(?object_id);

        match object_id {
            id::NodeCommand::START => {
                self.ops.push_back(Op::NodeStart { node_id });
            }
            id::NodeCommand::PAUSE => {
                self.ops.push_back(Op::NodePause { node_id });
            }
            _ => {
                tracing::warn!(?object_id, "Unsupported command");
            }
        }

        Ok(())
    }

    #[tracing::instrument(skip(self, st))]
    fn client_node_port_set_param(
        &mut self,
        node_id: ClientNodeId,
        mut st: Struct<Slice<'_>>,
    ) -> Result<()> {
        let node = self.client_nodes.get_mut(node_id)?;

        let direction = st.field()?.read::<Direction>()?;
        let port_id = st.field()?.read::<PortId>()?;
        let id = st.field()?.read_sized::<id::Param>()?;
        let flags = st.field()?.read_sized::<u32>()?;

        let port = node.ports.get_mut(direction, port_id)?;

        let what = if let Some(param) = st.field()?.read_option()? {
            tracing::trace!(?id, flags, object = ?param.as_ref().read_object()?, "set");
            port.set_param(id, [PortParam::with_flags(param.read_object()?, flags)])?;
            NodeUpdateWhat::SetPortParam(direction, port_id, id)
        } else {
            tracing::trace!(?id, flags, "remove");
            _ = port.remove_param(id);
            NodeUpdateWhat::RemovePortParam(direction, port_id, id)
        };

        self.ops.push_back(Op::NodeUpdate {
            node_id: node_id,
            what: Some(what),
        });
        Ok(())
    }

    #[tracing::instrument(skip(self, st))]
    fn client_node_use_buffers(
        &mut self,
        node_id: ClientNodeId,
        mut st: Struct<Slice<'_>>,
    ) -> Result<()> {
        let node = self.client_nodes.get_mut(node_id)?;

        let (direction, port_id, mix_id, flags, n_buffers) = st
            .read::<(Direction, PortId, MixId, u32, u32)>()
            .context("reading header")?;

        let mut buffers = Vec::new();

        for id in 0..n_buffers {
            let (mem_id, offset, size, n_metas) = st
                .read::<(u32, usize, usize, u32)>()
                .with_context(|| anyhow!("reading buffer {id}"))?;

            let mm = self
                .memory
                .map(mem_id, offset, size)
                .context("mapping buffer")?;

            let mut metas = Vec::new();

            let mut region = mm.clone();

            for _ in 0..n_metas {
                let (ty, size) = st.read::<(id::Meta, usize)>()?;
                self.memory.track(&region);

                metas.push(buffer::Meta {
                    ty,
                    region: region.clone(),
                });

                region = region.offset(size, 8)?;
            }

            let mut datas = Vec::new();

            let n_datas = st.read::<usize>()?;

            for id in 0..n_datas {
                let chunk = region.clone().size(mem::size_of::<ffi::Chunk>())?.cast()?;
                region = region.offset(mem::size_of::<ffi::Chunk>(), 8)?;
                self.memory.track(&chunk);

                let (ty, data, flags, offset, max_size) = st
                    .read::<(id::DataType, u32, flags::DataFlag, usize, usize)>()
                    .with_context(|| anyhow!("reading data for buffer {id}"))?;

                let region = match ty {
                    id::DataType::MEM_PTR => {
                        let Ok(data) = usize::try_from(data) else {
                            bail!("Invalid data offset {data} for data type {ty:?}");
                        };

                        let region = mm.offset(data, 1)?.size(max_size)?;

                        ensure!(offset == 0);

                        self.memory.track(&region);
                        region
                    }
                    id::DataType::MEM_FD => self.memory.map(data, offset, max_size)?,
                    ty => {
                        bail!("Unsupported data type {ty:?} in use buffers");
                    }
                };

                datas.push(buffer::Data {
                    ty,
                    region,
                    flags,
                    chunk,
                });
            }

            self.memory.free(mm);

            buffers.push(Buffer {
                id,
                offset,
                size,
                metas,
                datas,
            });
        }

        tracing::warn!(
            target: "io",
            ?direction,
            ?port_id,
            ?mix_id,
            buffers = buffers.len(),
            "UseBuffers"
        );

        let buffers = Buffers {
            direction,
            port_id,
            mix_id,
            flags,
            buffers,
            available: 0,
        };

        node.ports
            .get_mut(direction, port_id)?
            .replace_buffers(buffers, |b| {
                for buffer in b.buffers {
                    for meta in buffer.metas {
                        self.memory.free(meta.region);
                    }

                    for data in buffer.datas {
                        self.memory.free(data.region);
                        self.memory.free(data.chunk);
                    }
                }
            });

        Ok(())
    }

    #[tracing::instrument(skip(self, st))]
    fn client_node_port_set_io(
        &mut self,
        node_id: ClientNodeId,
        mut st: Struct<Slice<'_>>,
    ) -> Result<()> {
        let node = self.client_nodes.get_mut(node_id)?;

        let (direction, port_id, mix_id, id, mem_id, offset, size) =
            st.read::<(Direction, PortId, MixId, id::IoType, i32, usize, usize)>()?;

        let port = node.ports.get_mut(direction, port_id)?;

        let mem_id = u32::try_from(mem_id).ok();

        tracing::warn!(
            target: "io",
            ?direction,
            ?port_id,
            ?mix_id,
            ?id,
            ?mem_id,
            "SetIO"
        );

        let span = tracing::info_span!(
            "client_node_port_set_io",
            ?direction,
            ?port_id,
            ?mix_id,
            ?id,
            ?mem_id
        );
        let _span = span.enter();

        match id {
            id::IoType::CLOCK => {
                ensure!(mix_id == MixId::ZERO, "Mix ID must be 0 for CLOCK IO type");

                let Some(mem_id) = mem_id else {
                    if let Some(region) = port.io_clock.take() {
                        self.memory.free(region);
                    };

                    return Ok(());
                };

                let region = self.memory.map(mem_id, offset, size)?.cast()?;

                if let Some(region) = port.io_clock.replace(region) {
                    self.memory.free(region);
                }
            }
            id::IoType::POSITION => {
                ensure!(
                    mix_id == MixId::ZERO,
                    "Mix ID must be 0 for POSITION IO type"
                );

                let Some(mem_id) = mem_id else {
                    if let Some(region) = port.io_position.take() {
                        self.memory.free(region);
                    };

                    return Ok(());
                };

                let region = self.memory.map(mem_id, offset, size)?.cast()?;

                if let Some(region) = port.io_position.replace(region) {
                    self.memory.free(region);
                }
            }
            id::IoType::BUFFERS => {
                /// Free everything on the specified mix since the I/O area has
                /// changed and there are no other recourses for freeing
                /// reserved buffers.
                port.port_buffers.free_all(mix_id);

                if let Some(mem_id) = mem_id {
                    let region = self.memory.map(mem_id, offset, size)?.cast()?;
                    port.mixes.buffers.push(PortMix { mix_id, region });
                } else {
                    for buf in port.mixes.buffers.extract_if(.., |b| b.mix_id == mix_id) {
                        self.memory.free(buf.region);
                    }
                }
            }
            id => {
                tracing::warn!(?id, "Unsupported IO type in port set IO");
                return Ok(());
            }
        }

        Ok(())
    }

    #[tracing::instrument(skip(self, st))]
    fn client_node_set_activation(
        &mut self,
        node_id: ClientNodeId,
        mut st: Struct<Slice<'_>>,
    ) -> Result<()> {
        let peer_id = st.field()?.read_sized::<u32>()?;
        let signal_fd = st.field()?.read_sized::<Fd>()?;
        let mem_id = st.field()?.read_sized::<i32>()?;
        let offset = st.field()?.read_sized::<usize>()?;
        let size = st.field()?.read_sized::<usize>()?;

        let signal_fd = self.take_fd(signal_fd)?;

        let node = self.client_nodes.get_mut(node_id)?;

        for a in node
            .peer_activations
            .extract_if(.., |a| a.peer_id == peer_id)
        {
            self.memory.free(a.region);
        }

        let (Ok(mem_id), Some(signal_fd)) = (u32::try_from(mem_id), signal_fd) else {
            return Ok(());
        };

        let signal_fd = EventFd::from(signal_fd);
        let region = self.memory.map(mem_id, offset, size)?.cast()?;

        let peer = unsafe { PeerActivation::new(peer_id, signal_fd, region) };
        node.peer_activations.push(peer);
        Ok(())
    }

    #[tracing::instrument(skip(self, st))]
    fn client_node_set_mix_info(
        &mut self,
        node_id: ClientNodeId,
        mut st: Struct<Slice<'_>>,
    ) -> Result<()> {
        let direction = st.read::<Direction>()?;
        let port_id = st.read::<PortId>()?;
        let mix_id = st.read::<MixId>()?;
        let peer_id = st.read::<i32>()?;
        let peer_id = u32::try_from(peer_id).ok().map(PortId::new);

        tracing::warn!(target: "io", ?direction, ?port_id, ?mix_id, ?peer_id, "SetMixInfo");

        let mut props = st.read::<Struct<_>>()?;
        let n_items = props.read::<u32>()?;

        let mut values = BTreeMap::new();

        for _ in 0..n_items {
            let (key, value) = props.read::<(String, String)>()?;
            values.insert(key, value);
        }

        let node = self.client_nodes.get_mut(node_id)?;
        let port = node.ports.get_mut(direction, port_id)?;

        if let Some(peer_id) = peer_id {
            port.mix_info.insert(mix_id, peer_id, values);
        } else {
            port.mix_info.remove(mix_id);
        }

        Ok(())
    }
}

/// Read a frame from the current buffer.
fn frame<'buf>(buf: &'buf mut RecvBuf, header: &Header) -> Result<Option<Pod<Slice<'buf>>>> {
    let size = header.size() as usize;

    let Some(bytes) = buf.read_bytes(size) else {
        return Ok(None);
    };

    Ok(Some(Pod::new(pod::buf::slice(bytes))))
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
    properties: Properties,
    server_properties: BTreeMap<CString, CString>,
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
enum Kind {
    Registry,
    ClientNode(ClientNodeId),
}

#[derive(Debug)]
struct ReceivedFd {
    fd: Option<OwnedFd>,
}

#[derive(Debug)]
enum NodeUpdateWhat {
    SetNodeParam(id::Param),
    RemoveNodeParam(id::Param),
    SetPortParam(Direction, PortId, id::Param),
    RemovePortParam(Direction, PortId, id::Param),
}

#[derive(Debug)]
enum Op {
    CoreHello,
    GetRegistry,
    Pong {
        id: u32,
        seq: u32,
    },
    ConstructNode,
    NodeCreated {
        node_id: ClientNodeId,
    },
    NodeUpdate {
        node_id: ClientNodeId,
        what: Option<NodeUpdateWhat>,
    },
    NodeStart {
        node_id: ClientNodeId,
    },
    NodePause {
        node_id: ClientNodeId,
    },
    NodeReadInterest {
        node_id: ClientNodeId,
    },
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
