use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::mem;
use std::os::fd::FromRawFd;
use std::os::fd::OwnedFd;
use std::sync::Arc;

use anyhow::bail;
use pod::Fd;
use pod::Id;
use pod::Int;
use pod::Long;
use pod::Pod;
use pod::id;
use protocol::consts;
use protocol::ids::Ids;
use protocol::op;
use protocol::poll::{ChangeInterest, Interest, PollEvent, Token};
use protocol::types::Header;
use protocol::{Connection, DynamicBuf, EventFd, Poll};

use anyhow::{Context, Result};

const CONNECTION: Token = Token::new(100);
const EVENT: Token = Token::new(200);
const CREATE_CLIENT_NODE: u32 = 0x2000;
const GET_REGISTRY_SYNC: u32 = 0x1000;

#[derive(Default, Debug)]
struct CoreState {
    id: i32,
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
struct Activation {
    fd: OwnedFd,
    mem_id: u32,
    offset: i32,
    size: i32,
}

#[derive(Default, Debug)]
struct ClientNodeState {
    #[allow(unused)]
    id: u32,
    port_id: u32,
    read_fd: Option<OwnedFd>,
    write_fd: Option<OwnedFd>,
    activation: Option<Activation>,
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
    Pong { id: i32, seq: i32 },
    ConstructNode,
    AddPort { client: usize },
}

#[derive(Debug)]
struct Memory {
    ty: Id<u32>,
    fd: OwnedFd,
    flags: i32,
}

#[derive(Default, Debug)]
struct ConnectionState {
    core: CoreState,
    client: ClientState,
    registries: BTreeMap<u32, RegistryState>,
    factories: BTreeMap<String, u32>,
    globals: BTreeMap<u32, u32>,
    state: State,
    has_header: bool,
    header: Header,
    client_nodes: Vec<ClientNodeState>,
    id_to_kind: BTreeMap<u32, Kind>,
    ids: Ids,
    fds: VecDeque<ReceivedFd>,
    ops: VecDeque<Op>,
    memory: HashMap<u32, Memory>,
}

impl ConnectionState {
    fn run(&mut self, c: &mut Connection, recv: &mut DynamicBuf) -> Result<()> {
        'next: loop {
            while let Some(op) = self.ops.pop_front() {
                match op {
                    Op::Pong { id, seq } => {
                        c.core_pong(id, seq)?;
                    }
                    Op::ConstructNode => {
                        tracing::info!("Constructing client node");

                        std::dbg!(&self.factories);

                        'done: {
                            let Some(registry) = self
                                .factories
                                .get("client-node")
                                .and_then(|id| self.registries.get(id))
                            else {
                                tracing::warn!("No factory for client-node");
                                break 'done;
                            };

                            let Some(type_name) = registry.properties.get("factory.type.name")
                            else {
                                tracing::warn!("No factory type name for client-node");
                                break 'done;
                            };

                            let Some(version) = registry
                                .properties
                                .get("factory.type.version")
                                .and_then(|version| str::parse::<u32>(version).ok())
                            else {
                                tracing::warn!("No factory type version for client-node");
                                break 'done;
                            };

                            let new_id = self.ids.alloc().context("ran out of identifiers")?;
                            let port_id = self.ids.alloc().context("ran out of identifiers")?;

                            std::dbg!(type_name);

                            c.core_create_object("client-node", type_name, version, new_id)?;

                            let index = self.client_nodes.len();
                            self.id_to_kind.insert(new_id, Kind::ClientNode(index));

                            self.client_nodes.push(ClientNodeState {
                                id: new_id,
                                port_id,
                                write_fd: None,
                                read_fd: None,
                                activation: None,
                            });
                        };
                    }
                    Op::AddPort { client } => {
                        tracing::info!("Adding port to client node");

                        if let Some(client) = self.client_nodes.get(client) {
                            // let new_id = self.ids.alloc().context("ran out of identifiers")?;
                            // c.client_node_get_node(client.id, 3, new_id)?;
                            // c.client_node_add_port(client.id, consts::Direction::Output, client.port_id)?;
                            c.client_node_set_active(client.id, true)?;
                            // c.client_node_update(client.id)?;
                        }
                    }
                }
            }

            match self.state {
                State::ClientHello => {
                    c.core_hello()?;
                    c.client_update_properties()?;
                    self.state = State::Connecting;
                }
                State::CoreBound => {
                    tracing::info!("Getting registry");

                    let new_id = self.ids.alloc().context("ran out of identifiers")?;
                    c.core_get_registry(new_id as i32)?;
                    self.id_to_kind.insert(new_id, Kind::Registry);
                    c.core_sync(GET_REGISTRY_SYNC)?;
                    self.state = State::Idle;
                }
                _ => {}
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

                tracing::trace!(self.header.id = ?self.header.id(), self.header.op = ?self.header.op(), "Received frame");

                let result = match self.header.id() {
                    consts::CORE_ID => self.handle_core(pod),
                    consts::CLIENT_ID => self.handle_client(pod),
                    _ => self.handle_dynamic(pod),
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

    /// Take a file descriptor from the stored range.
    fn take_fd(&mut self, fd: Fd) -> Result<OwnedFd> {
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

        Ok(fd)
    }

    #[tracing::instrument(skip_all)]
    fn handle_core(&mut self, pod: Pod<&[u64]>) -> Result<()> {
        match self.header.op() {
            op::CORE_INFO_EVENT => {
                self.handle_core_info_event(pod).context("Core::Info")?;
            }
            op::CORE_DONE_EVENT => {
                self.handle_core_done_event(pod).context("Core::Done")?;
            }
            op::CORE_PING_EVENT => {
                self.handle_core_ping_event(pod).context("Core::Ping")?;
            }
            op::CORE_ERROR_EVENT => {
                self.handle_core_error_event(pod).context("Core::Error")?;
            }
            op::CORE_BOUND_ID_EVENT => {
                self.handle_core_bound_id_event(pod)
                    .context("Core::BoundId")?;
            }
            op::CORE_ADD_MEM_EVENT => {
                self.handle_core_add_mem_event(pod)
                    .context("Core::AddMem")?;
            }
            op => {
                tracing::warn!(op, "Core unsupported op");
            }
        }

        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn handle_client(&mut self, pod: Pod<&[u64]>) -> Result<()> {
        match self.header.op() {
            op::CLIENT_INFO_EVENT => {
                self.handle_client_info(pod).context("Client::Info")?;
            }
            op::CLIENT_ERROR_EVENT => {
                self.handle_client_error(pod).context("Client::Error")?;
            }
            op => {
                tracing::warn!(op, "Client unsupported op");
            }
        }

        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn handle_dynamic(&mut self, pod: Pod<&[u64]>) -> Result<()> {
        let Some(kind) = self.id_to_kind.get(&self.header.id()) else {
            tracing::warn!(?self.header, "Unknown receiver");
            return Ok(());
        };

        match *kind {
            Kind::Registry => match self.header.op() {
                op::REGISTRY_GLOBAL_EVENT => {
                    self.handle_registry_global(pod)
                        .context("Registry::Global")?;
                }
                op::REGISTRY_GLOBAL_REMOVE_EVENT => {
                    self.handle_registry_global_remove(pod)
                        .context("Registry::GlobalRemove")?;
                }
                op => {
                    tracing::warn!(op, "Registry unsupported op");
                }
            },
            Kind::ClientNode(index) => match self.header.op() {
                op::CLIENT_NODE_TRANSPORT_EVENT => {
                    self.handle_client_node_transport(index, pod)
                        .context("ClientNode::Transport")?;
                }
                op::CLIENT_NODE_SET_PARAM_EVENT => {
                    self.handle_client_node_set_param(index, pod)
                        .context("ClientNode::SetParam")?;
                }
                op::CLIENT_NODE_SET_IO_EVENT => {
                    self.handle_client_node_set_io(index, pod)
                        .context("ClientNode::SetIO")?;
                }
                op::CLIENT_NODE_SET_ACTIVATION_EVENT => {
                    self.handle_client_node_set_activation(index, pod)
                        .context("ClientNode::SetActivation")?;
                }
                op => {
                    tracing::warn!(op, "Client node unsupported op");
                }
            },
        }

        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn handle_core_info_event(&mut self, pod: Pod<&[u64]>) -> Result<()> {
        let mut st = pod.decode_struct()?;
        let id = st.field()?.decode::<Int>()?;
        let cookie = st.field()?.decode::<Int>()?;
        let user_name = st.field()?.decode_unsized::<str, _>(str::to_owned)?;
        let host_name = st.field()?.decode_unsized::<str, _>(str::to_owned)?;
        let version = st.field()?.decode_unsized::<str, _>(str::to_owned)?;
        let name = st.field()?.decode_unsized::<str, _>(str::to_owned)?;
        let change_mask = st.field()?.decode::<Long>()?;

        let mut props = st.field()?.decode_struct()?;

        if change_mask & 0x1 != 0 {
            let n_items = props.field()?.decode::<Int>()?;

            for _ in 0..n_items {
                let key = props.field()?.decode_unsized::<str, _>(str::to_owned)?;
                let value = props.field()?.decode_unsized::<str, _>(str::to_owned)?;
                self.core.properties.insert(key, value);
            }
        }

        self.core.id = id;
        self.core.cookie = cookie;
        self.core.user_name = user_name;
        self.core.host_name = host_name;
        self.core.version = version;
        self.core.name = name;
        self.state = State::CoreBound;
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn handle_core_done_event(&mut self, pod: Pod<&[u64]>) -> Result<()> {
        let mut st = pod.decode_struct()?;
        let id = st.field()?.decode::<Int>()?.cast_unsigned();
        let seq = st.field()?.decode::<Int>()?;

        match id {
            GET_REGISTRY_SYNC => {
                self.ops.push_back(Op::ConstructNode);
                tracing::info!(id, seq, "Intitial registry sync done");
            }
            CREATE_CLIENT_NODE => {
                tracing::info!("Client node created");
            }
            _ => {}
        }

        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn handle_core_ping_event(&mut self, pod: Pod<&[u64]>) -> Result<()> {
        let mut st = pod.decode_struct()?;
        let id = st.field()?.decode::<Int>()?;
        let seq = st.field()?.decode::<Int>()?;

        tracing::debug!("Core ping {id} with seq {seq}");
        self.ops.push_back(Op::Pong { id, seq });
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn handle_core_error_event(&mut self, pod: Pod<&[u64]>) -> Result<()> {
        let mut st = pod.decode_struct()?;
        let id = st.field()?.decode::<Int>()?;
        let seq = st.field()?.decode::<Int>()?;
        let res = st.field()?.decode::<Int>()?;
        let error = st.field()?.decode_unsized::<str, _>(str::to_owned)?;

        tracing::error!(id, seq, res, error, "Core resource errored");
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn handle_core_bound_id_event(&mut self, pod: Pod<&[u64]>) -> Result<()> {
        let mut st = pod.decode_struct()?;
        let local_id = st.field()?.decode::<Int>()?.cast_unsigned();
        let global_id = st.field()?.decode::<Int>()?.cast_unsigned();
        self.globals.insert(local_id, global_id);

        tracing::info!(local_id, global_id, "Core bound id");

        if let Some(kind) = self.id_to_kind.get_mut(&local_id) {
            match *kind {
                Kind::Registry => {}
                Kind::ClientNode(index) => {
                    self.ops.push_back(Op::AddPort { client: index });
                }
            }
        }

        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn handle_core_add_mem_event(&mut self, pod: Pod<&[u64]>) -> Result<()> {
        let mut st = pod.decode_struct()?;
        let id = st.field()?.decode::<Int>()?.cast_unsigned();
        let ty = st.field()?.decode::<Id<u32>>()?;
        let fd = self.take_fd(st.field()?.decode::<Fd>()?)?;
        let flags = st.field()?.decode::<Int>()?;

        tracing::info!(id, ?ty, ?fd, flags, "Core add memory");

        self.memory.insert(id, Memory { ty, fd, flags });
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn handle_client_info(&mut self, pod: Pod<&[u64]>) -> Result<()> {
        let mut st = pod.decode_struct()?;
        let id = st.field()?.decode::<Int>()?.cast_unsigned();
        let change_mask = st.field()?.decode::<Long>()?;

        let mut props = st.field()?.decode_struct()?;

        if change_mask & 0x1 != 0 {
            let n_items = props.field()?.decode::<Int>()?;

            for _ in 0..n_items {
                let key = props.field()?.decode_unsized::<str, _>(str::to_owned)?;
                let value = props.field()?.decode_unsized::<str, _>(str::to_owned)?;

                self.client.properties.insert(key, value);
            }
        }

        self.client.id = id;
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn handle_client_error(&mut self, pod: Pod<&[u64]>) -> Result<()> {
        let mut st = pod.decode_struct()?;
        let id = st.field()?.decode::<Int>()?;
        let res = st.field()?.decode::<Int>()?;
        let error = st.field()?.decode_unsized::<str, _>(str::to_owned)?;
        tracing::error!(id, res, error, "Client errored");
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn handle_registry_global(&mut self, pod: Pod<&[u64]>) -> Result<()> {
        let mut st = pod.decode_struct()?;

        let id = st.field()?.decode::<Int>()?.cast_unsigned();

        let registry = self.registries.entry(id).or_default();

        registry.id = id;
        registry.permissions = st.field()?.decode::<Int>()?;
        registry.ty = st.field()?.decode::<String>()?;
        registry.version = st.field()?.decode::<Int>()?.cast_unsigned();

        let mut props = st.field()?.decode_struct()?;

        let n_items = props.field()?.decode::<Int>()?;

        for _ in 0..n_items {
            let key = props.field()?.decode_unsized::<str, _>(str::to_owned)?;
            let value = props.field()?.decode_unsized::<str, _>(str::to_owned)?;
            registry.properties.insert(key, value);
        }

        if registry.ty == consts::INTERFACE_FACTORY {
            if let Some(name) = registry.properties.get("factory.name") {
                self.factories.insert(name.clone(), id);
            }
        }

        tracing::trace!(id, ?registry, "Registry global event");
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn handle_registry_global_remove(&mut self, pod: Pod<&[u64]>) -> Result<()> {
        let mut st = pod.decode_struct()?;
        let id = st.field()?.decode::<Int>()?.cast_unsigned();

        tracing::info!(id, "Registry global remove event");

        if self.registries.remove(&id).is_none() {
            tracing::warn!("Tried to remove unknown registry {id}");
        }

        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn handle_client_node_transport(&mut self, index: usize, pod: Pod<&[u64]>) -> Result<()> {
        let mut st = pod.decode_struct()?;
        let read_fd = self.take_fd(st.field()?.decode::<Fd>()?)?;
        let write_fd = self.take_fd(st.field()?.decode::<Fd>()?)?;
        let memfd = st.field()?.decode::<Int>()?;
        let offset = st.field()?.decode::<Int>()?;
        let size = st.field()?.decode::<Int>()?;

        tracing::info!(
            index,
            ?read_fd,
            ?write_fd,
            memfd,
            offset,
            size,
            "Client node transport"
        );

        let Some(node) = self.client_nodes.get_mut(index) else {
            bail!("Missing client node {index}");
        };

        node.read_fd = Some(read_fd);
        node.write_fd = Some(write_fd);
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn handle_client_node_set_param(&mut self, index: usize, pod: Pod<&[u64]>) -> Result<()> {
        tracing::info!(index, ?pod, "set param");

        let mut st = pod.decode_struct()?;
        let param = st.field()?.decode::<id::Param>()?;
        let flags = st.field()?.decode::<Int>()?;
        let mut obj = st.field()?.decode_object()?;

        let object_type = id::ObjectType::from_id(obj.object_type());
        let object_id = id::Param::from_id(obj.object_id());

        match object_id {
            id::Param::PROPS => {
                while !obj.is_empty() {
                    let p = obj.property()?;

                    match id::Prop::from_id(p.key()) {
                        id::Prop::CHANNEL_VOLUMES => {
                            let value = p.value().decode_array()?;
                            tracing::info!(?value, "Set channel volumes");
                        }
                        prop => {
                            tracing::warn!(?prop, "Unsupported property in set param");
                        }
                    }
                }
            }
            id => {
                tracing::warn!(?id, "Unsupported param in set param");
            }
        }

        tracing::info!(?param, flags, ?object_type, ?object_id, "set param");
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn handle_client_node_set_io(&mut self, index: usize, pod: Pod<&[u64]>) -> Result<()> {
        let Some(..) = self.client_nodes.get_mut(index) else {
            bail!("Missing client node {index}");
        };

        let mut st = pod.decode_struct()?;
        let Id(id) = st.field()?.decode::<Id<u32>>()?;
        let memid = st.field()?.decode::<Int>()?;
        let offset = st.field()?.decode::<Int>()?;
        let size = st.field()?.decode::<Int>()?;

        tracing::info!(index, id, memid, offset, size, "Client node set IO");
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn handle_client_node_set_activation(&mut self, index: usize, pod: Pod<&[u64]>) -> Result<()> {
        let mut st = pod.decode_struct()?;
        let node_id = st.field()?.decode::<Int>()?;
        let fd = self.take_fd(st.field()?.decode::<Fd>()?)?;
        let mem_id = st.field()?.decode::<Int>()?.cast_unsigned();
        let offset = st.field()?.decode::<Int>()?;
        let size = st.field()?.decode::<Int>()?;

        tracing::info!(
            index,
            node_id,
            ?fd,
            mem_id,
            offset,
            size,
            "Client node set activation"
        );

        let Some(node) = self.client_nodes.get_mut(index) else {
            bail!("Missing client node {index}");
        };

        node.activation = Some(Activation {
            fd,
            mem_id,
            offset,
            size,
        });
        Ok(())
    }
}

#[derive(Default, Debug)]
enum State {
    #[default]
    ClientHello,
    Connecting,
    CoreBound,
    Idle,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::try_init().map_err(anyhow::Error::msg)?;

    let ev = Arc::new(EventFd::new(0)?);
    let mut poll = Poll::new()?;
    let mut c = Connection::open()?;

    c.set_nonblocking(true)?;

    let mut recv = DynamicBuf::new();

    poll.add(&c, CONNECTION, c.interest())?;
    poll.add(&ev, EVENT, Interest::READ)?;

    let mut events = pod::Buf::<PollEvent, 4>::new();
    let mut state = ConnectionState::default();

    // Well-known identifiers.
    state.ids.set(consts::CORE_ID as u32);
    state.ids.set(consts::CLIENT_ID as u32);

    let mut fds = [0; 16];

    loop {
        state.run(&mut c, &mut recv)?;

        if let ChangeInterest::Changed(interest) = c.modified() {
            poll.modify(&c, CONNECTION, interest)?;
        }

        poll.poll(&mut events)?;

        while let Some(e) = events.pop_front() {
            match e.token {
                CONNECTION => {
                    if e.interest.is_read() {
                        let n_fds = c
                            .recv_with_fds(&mut recv, &mut fds[..])
                            .context("Failed to receive file descriptors")?;

                        for fd in &mut fds[..n_fds] {
                            // SAFETY: We must trust the file descriptor we have
                            // just received.
                            state.fds.push_back(ReceivedFd {
                                fd: Some(unsafe { OwnedFd::from_raw_fd(mem::take(fd)) }),
                            });
                        }
                    }

                    if e.interest.is_write() {
                        c.send()?;
                    }
                }
                EVENT => {
                    if let Some(value) = ev.read()? {
                        println!("Event: {value}");
                    }
                }
                other => {
                    println!("Unknown token: {other:?}");
                }
            }
        }
    }
}
