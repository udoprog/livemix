use std::collections::BTreeMap;
use std::collections::VecDeque;
use std::mem;
use std::os::fd::FromRawFd;
use std::os::fd::OwnedFd;

use anyhow::bail;
use pod::Array;
use pod::Fd;
use pod::Id;
use pod::Int;
use pod::Long;
use pod::Pod;
use protocol::consts;
use protocol::ids::Ids;
use protocol::op;
use protocol::poll::{ChangeInterest, Interest, PollEvent, Token};
use protocol::types::Header;
use protocol::{Buf, Connection, EventFd, Poll};

use anyhow::{Context, Result};

const CONNECTION: Token = Token::new(100);
const EVENT: Token = Token::new(200);

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
    id: i32,
    properties: BTreeMap<String, String>,
}

#[derive(Default, Debug)]
struct RegistryState {
    id: i32,
    permissions: i32,
    ty: String,
    version: i32,
    properties: BTreeMap<String, String>,
}

#[derive(Default, Debug)]
struct ClientNodeState {
    #[allow(unused)]
    id: u32,
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

#[derive(Default, Debug)]
struct ConnectionState {
    core: CoreState,
    client: ClientState,
    registries: BTreeMap<i32, RegistryState>,
    factories: BTreeMap<String, i32>,
    globals: BTreeMap<i32, i32>,
    state: State,
    has_header: bool,
    header: Header,
    initial_get_registry: Option<i32>,
    client_nodes: Vec<ClientNodeState>,
    id_to_kind: BTreeMap<u32, Kind>,
    ids: Ids,
    fds: VecDeque<ReceivedFd>,
    pings: VecDeque<(i32, i32)>,
}

impl ConnectionState {
    fn run(&mut self, c: &mut Connection, recv: &mut Buf) -> Result<()> {
        'next: loop {
            while let Some((id, seq)) = self.pings.pop_front() {
                c.core_pong(id, seq)?;
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
                    self.initial_get_registry = Some(c.core_sync(0)?);
                    self.state = State::Idle;
                }
                State::ConstructNode => {
                    tracing::info!("Constructing client node");

                    'done: {
                        let Some(registry) = self
                            .factories
                            .get("client-node")
                            .and_then(|id| self.registries.get(id))
                        else {
                            tracing::warn!("No factory for client-node");
                            break 'done;
                        };

                        let Some(type_name) = registry.properties.get("factory.type.name") else {
                            tracing::warn!("No factory type name for client-node");
                            break 'done;
                        };

                        let Some(version) = registry
                            .properties
                            .get("factory.type.version")
                            .and_then(|version| str::parse(version).ok())
                        else {
                            tracing::warn!("No factory type version for client-node");
                            break 'done;
                        };

                        let new_id = self.ids.alloc().context("ran out of identifiers")?;
                        c.core_create_object("client-node", type_name, version, new_id as i32)?;
                        let index = self.client_nodes.len();
                        self.id_to_kind.insert(new_id, Kind::ClientNode(index));
                        self.client_nodes.push(ClientNodeState { id: new_id });
                    };

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

    fn handle_client(&mut self, pod: Pod<&[u64]>) -> Result<()> {
        match self.header.op() {
            op::CLIENT_INFO => {
                self.handle_client_info(pod).context("Client::Info")?;
            }
            op::CLIENT_ERROR => {
                self.handle_client_error(pod).context("Client::Error")?;
            }
            op => {
                tracing::warn!(op, "Client unsupported op");
            }
        }

        Ok(())
    }

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
                op => {
                    tracing::warn!(op, "Registry unsupported op");
                }
            },
            Kind::ClientNode(index) => match self.header.op() {
                op::CLIENT_NODE_TRANSPORT_EVENT => {
                    self.handle_client_node_transport(index, pod)
                        .context("ClientNode::Transport")?;
                }
                op::CLIENT_NODE_SET_IO_EVENT => {
                    self.handle_client_node_set_io(index, pod)
                        .context("ClientNode::Update")?;
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
        let id = st.field()?.decode::<Int>()?;
        let seq = st.field()?.decode::<Int>()?;

        if Some(seq) == self.initial_get_registry {
            self.initial_get_registry = None;
            self.state = State::ConstructNode;
            tracing::info!(id, seq, "Intitial registry sync done");
        }

        tracing::info!(id, seq, "Core sync done");
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn handle_core_ping_event(&mut self, pod: Pod<&[u64]>) -> Result<()> {
        let mut st = pod.decode_struct()?;
        let id = st.field()?.decode::<Int>()?;
        let seq = st.field()?.decode::<Int>()?;

        tracing::debug!("Core ping {id} with seq {seq}");
        self.pings.push_back((id, seq));
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn handle_core_error_event(&mut self, pod: Pod<&[u64]>) -> Result<()> {
        let mut st = pod.decode_struct()?;
        let id = st.field()?.decode::<Int>()?;
        let _ = st.field()?.decode::<Int>()?;
        let res = st.field()?.decode::<Int>()?;
        let error = st.field()?.decode_unsized::<str, _>(str::to_owned)?;

        tracing::error!("Core resource {id} errored: {error} ({res})");
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn handle_core_bound_id_event(&mut self, pod: Pod<&[u64]>) -> Result<()> {
        let mut st = pod.decode_struct()?;
        let local_id = st.field()?.decode::<Int>()?;
        let global_id = st.field()?.decode::<Int>()?;
        self.globals.insert(local_id, global_id);

        tracing::debug!(local_id, global_id, "Core bound id");
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn handle_core_add_mem_event(&mut self, pod: Pod<&[u64]>) -> Result<()> {
        let mut st = pod.decode_struct()?;
        let id = st.field()?.decode::<Int>()?;
        let ty = st.field()?.decode::<Id<u32>>()?;
        let fd = self.take_fd(st.field()?.decode::<Fd>()?)?;
        let flags = st.field()?.decode::<Int>()?;

        tracing::info!(id, ?ty, ?fd, flags, "Core add memory");
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn handle_client_info(&mut self, pod: Pod<&[u64]>) -> Result<()> {
        let mut st = pod.decode_struct()?;
        let id = st.field()?.decode::<Int>()?;
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
        tracing::error!("Client {id} errored: {error} ({res})");
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn handle_registry_global(&mut self, pod: Pod<&[u64]>) -> Result<()> {
        let mut st = pod.decode_struct()?;

        let id = st.field()?.decode::<Int>()?;

        let registry = self.registries.entry(id).or_default();

        registry.id = id;
        registry.permissions = st.field()?.decode::<Int>()?;
        registry.ty = st.field()?.decode::<String>()?;
        registry.version = st.field()?.decode::<Int>()?;

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

    fn handle_client_node_transport(&mut self, index: usize, pod: Pod<&[u64]>) -> Result<()> {
        let Some(..) = self.client_nodes.get_mut(index) else {
            bail!("Missing client node {index}");
        };

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
        Ok(())
    }

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

    fn handle_client_node_set_activation(&mut self, index: usize, pod: Pod<&[u64]>) -> Result<()> {
        let Some(..) = self.client_nodes.get_mut(index) else {
            bail!("Missing client node {index}");
        };

        let mut st = pod.decode_struct()?;
        let node_id = st.field()?.decode::<Int>()?;
        let fd = self.take_fd(st.field()?.decode::<Fd>()?)?;
        let memid = st.field()?.decode::<Int>()?;
        let offset = st.field()?.decode::<Int>()?;
        let size = st.field()?.decode::<Int>()?;

        tracing::info!(
            index,
            node_id,
            ?fd,
            memid,
            offset,
            size,
            "Client node set activation"
        );
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
    ConstructNode,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::try_init().map_err(anyhow::Error::msg)?;

    let ev = EventFd::new(0)?;
    let mut poll = Poll::new()?;
    let mut c = Connection::open()?;

    c.set_nonblocking(true)?;

    let mut recv = Buf::new();

    poll.add(&c, CONNECTION, c.interest())?;
    poll.add(&ev, EVENT, Interest::READ)?;

    let mut events = Array::<PollEvent, 4>::new();
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
