use std::collections::{BTreeMap, HashMap, VecDeque};
use std::os::fd::OwnedFd;

use anyhow::bail;
use pod::{Fd, Id, Object, Pod};
use protocol::consts;
use protocol::id;
use protocol::ids::Ids;
use protocol::op;
use protocol::types::Header;
use protocol::{Connection, DynamicBuf};

use anyhow::{Context, Result};
use slab::Slab;

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
struct Activation {
    fd: OwnedFd,
    mem_id: u32,
    offset: i32,
    size: i32,
}

#[derive(Default, Debug)]
#[allow(unused)]
struct ClientNodeState {
    id: u32,
    port_id: u32,
    read_fd: Option<OwnedFd>,
    write_fd: Option<OwnedFd>,
    activation: Option<Activation>,
    params: BTreeMap<id::Param, Object<Box<[u64]>>>,
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
    Pong { id: u32, seq: u32 },
    ConstructNode,
    AddPort { client: usize },
}

#[derive(Debug)]
#[allow(unused)]
struct MemoryState {
    ty: Id<u32>,
    fd: OwnedFd,
    flags: i32,
}

#[derive(Debug)]
pub struct GlobalMap {
    global_to_local: BTreeMap<u32, u32>,
    local_to_global: BTreeMap<u32, u32>,
}

impl GlobalMap {
    #[inline]
    fn new() -> Self {
        Self {
            global_to_local: BTreeMap::new(),
            local_to_global: BTreeMap::new(),
        }
    }

    #[inline]
    fn insert(&mut self, local_id: u32, global_id: u32) {
        self.global_to_local.insert(global_id, local_id);
        self.local_to_global.insert(local_id, global_id);
    }

    /// Map a global to a local id.
    #[inline]
    fn by_global(&self, global_id: u32) -> Option<u32> {
        self.global_to_local.get(&global_id).copied()
    }

    /// Map a local to a global id.
    #[inline]
    fn by_local(&self, local_id: u32) -> Option<u32> {
        self.local_to_global.get(&local_id).copied()
    }

    #[inline]
    fn remove_by_local(&mut self, local_id: u32) -> Option<u32> {
        let global_id = self.local_to_global.remove(&local_id)?;
        self.global_to_local.remove(&global_id);
        Some(global_id)
    }

    #[inline]
    fn remove_by_global(&mut self, global_id: u32) -> Option<u32> {
        let local_id = self.global_to_local.remove(&global_id)?;
        self.local_to_global.remove(&local_id);
        Some(local_id)
    }
}

#[derive(Debug)]
pub struct ConnectionState {
    core: CoreState,
    client: ClientState,
    registries: Slab<RegistryState>,
    id_to_registry: BTreeMap<u32, usize>,
    factories: BTreeMap<String, usize>,
    globals: GlobalMap,
    client_nodes: Slab<ClientNodeState>,
    local_id_to_kind: BTreeMap<u32, Kind>,
    state: State,
    has_header: bool,
    header: Header,
    ids: Ids,
    fds: VecDeque<ReceivedFd>,
    ops: VecDeque<Op>,
    memory: HashMap<u32, MemoryState>,
}

impl ConnectionState {
    pub fn new() -> Self {
        let mut ids = Ids::new();

        // Well-known identifiers.
        ids.set(consts::CORE_ID);
        ids.set(consts::CLIENT_ID);

        Self {
            core: CoreState::default(),
            client: ClientState::default(),
            registries: Slab::new(),
            id_to_registry: BTreeMap::new(),
            factories: BTreeMap::new(),
            globals: GlobalMap::new(),
            client_nodes: Slab::new(),
            local_id_to_kind: BTreeMap::new(),
            state: State::ClientHello,
            has_header: false,
            header: Header::default(),
            ids,
            fds: VecDeque::with_capacity(16),
            ops: VecDeque::new(),
            memory: HashMap::new(),
        }
    }

    /// Add file descriptors.
    pub fn add_fds(&mut self, fds: impl IntoIterator<Item = OwnedFd>) {
        for fd in fds {
            self.fds.push_back(ReceivedFd { fd: Some(fd) });
        }
    }

    /// Process client.
    pub fn run(&mut self, c: &mut Connection, recv: &mut DynamicBuf) -> Result<()> {
        'next: loop {
            while let Some(op) = self.ops.pop_front() {
                match op {
                    Op::Pong { id, seq } => {
                        c.core_pong(id, seq)?;
                    }
                    Op::ConstructNode => {
                        if let Err(error) = self.op_construct_node(c) {
                            tracing_error!(error, "Failed to construct client node");
                        }
                    }
                    Op::AddPort { client } => {
                        tracing::info!("Adding port to client node");

                        if let Some(client) = self.client_nodes.get(client) {
                            c.client_node_update(client.id)?;
                            c.client_node_port_update(
                                client.id,
                                consts::Direction::Output,
                                client.port_id,
                            )?;
                            c.client_node_set_active(client.id, true)?;
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

                    let local_id = self.ids.alloc().context("ran out of identifiers")?;
                    c.core_get_registry(local_id)?;
                    self.local_id_to_kind.insert(local_id, Kind::Registry);
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

    fn op_construct_node(&mut self, c: &mut Connection) -> Result<()> {
        tracing::info!("Constructing client node");

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
        let port_id = self.ids.alloc().context("ran out of identifiers")?;

        c.core_create_object("client-node", type_name, version, new_id)?;

        let index = self.client_nodes.insert(ClientNodeState {
            id: new_id,
            port_id,
            write_fd: None,
            read_fd: None,
            activation: None,
            params: BTreeMap::new(),
        });

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
                op::CLIENT_NODE_SET_ACTIVATION_EVENT => {
                    self.client_node_set_activation(index, pod)
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
    fn core_info_event(&mut self, pod: Pod<&[u64]>) -> Result<()> {
        let mut st = pod.decode_struct()?;
        let id = st.field()?.decode::<u32>()?;
        let cookie = st.field()?.decode::<i32>()?;
        let user_name = st.field()?.decode_unsized::<str, _>(str::to_owned)?;
        let host_name = st.field()?.decode_unsized::<str, _>(str::to_owned)?;
        let version = st.field()?.decode_unsized::<str, _>(str::to_owned)?;
        let name = st.field()?.decode_unsized::<str, _>(str::to_owned)?;
        let change_mask = st.field()?.decode::<u64>()?;

        let mut props = st.field()?.decode_struct()?;

        if change_mask & 0x1 != 0 {
            let n_items = props.field()?.decode::<i32>()?;

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
    fn core_done_event(&mut self, pod: Pod<&[u64]>) -> Result<()> {
        let mut st = pod.decode_struct()?;
        let id = st.field()?.decode::<u32>()?;
        let seq = st.field()?.decode::<u32>()?;

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
        let mut st = pod.decode_struct()?;
        let id = st.field()?.decode()?;
        let seq = st.field()?.decode()?;

        tracing::debug!("Core ping {id} with seq {seq}");
        self.ops.push_back(Op::Pong { id, seq });
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn core_error_event(&mut self, pod: Pod<&[u64]>) -> Result<()> {
        let mut st = pod.decode_struct()?;
        let id = st.field()?.decode::<i32>()?;
        let seq = st.field()?.decode::<i32>()?;
        let res = st.field()?.decode::<i32>()?;
        let error = st.field()?.decode_unsized::<str, _>(str::to_owned)?;

        tracing::error!(id, seq, res, error, "Core resource errored");
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn core_bound_id_event(&mut self, pod: Pod<&[u64]>) -> Result<()> {
        let mut st = pod.decode_struct()?;
        let local_id = st.field()?.decode::<u32>()?;
        let global_id = st.field()?.decode::<u32>()?;

        self.globals.insert(local_id, global_id);
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn core_add_mem_event(&mut self, pod: Pod<&[u64]>) -> Result<()> {
        let mut st = pod.decode_struct()?;
        let id = st.field()?.decode::<u32>()?;
        let ty = st.field()?.decode::<Id<u32>>()?;
        let fd = self.take_fd(st.field()?.decode::<Fd>()?)?;
        let flags = st.field()?.decode::<i32>()?;

        tracing::info!(id, ?ty, ?fd, flags, "Core add memory");

        self.memory.insert(id, MemoryState { ty, fd, flags });
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn client_info(&mut self, pod: Pod<&[u64]>) -> Result<()> {
        let mut st = pod.decode_struct()?;
        let id = st.field()?.decode::<u32>()?;
        let change_mask = st.field()?.decode::<u64>()?;

        let mut props = st.field()?.decode_struct()?;

        if change_mask & 0x1 != 0 {
            let n_items = props.field()?.decode::<i32>()?;

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
    fn client_error(&mut self, pod: Pod<&[u64]>) -> Result<()> {
        let mut st = pod.decode_struct()?;
        let id = st.field()?.decode::<i32>()?;
        let res = st.field()?.decode::<i32>()?;
        let error = st.field()?.decode_unsized::<str, _>(str::to_owned)?;
        tracing::error!(id, res, error, "Client errored");
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn registry_global(&mut self, pod: Pod<&[u64]>) -> Result<()> {
        let mut st = pod.decode_struct()?;

        let id = st.field()?.decode::<u32>()?;

        let index = self.registries.vacant_key();

        let mut registry = RegistryState::default();

        registry.id = id;
        registry.permissions = st.field()?.decode::<i32>()?;
        registry.ty = st.field()?.decode::<String>()?;
        registry.version = st.field()?.decode::<u32>()?;

        let mut props = st.field()?.decode_struct()?;

        let n_items = props.field()?.decode::<i32>()?;

        for _ in 0..n_items {
            let key = props.field()?.decode_unsized::<str, _>(str::to_owned)?;
            let value = props.field()?.decode_unsized::<str, _>(str::to_owned)?;
            registry.properties.insert(key, value);
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
                    tracing::warn!(
                        index,
                        "Found interesting client node that was just registered"
                    );
                    self.ops.push_back(Op::AddPort { client: index });
                }
            }
        }

        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn registry_global_remove(&mut self, pod: Pod<&[u64]>) -> Result<()> {
        let mut st = pod.decode_struct()?;
        let id = st.field()?.decode::<u32>()?;

        let Some(registry_index) = self.id_to_registry.remove(&id) else {
            tracing::warn!(id, "Tried to remove unknown registry");
            return Ok(());
        };

        let Some(registry) = self.registries.try_remove(registry_index) else {
            tracing::warn!(registry_index, "Tried to remove unknown registry index");
            return Ok(());
        };

        tracing::info!(?registry, "Removed registry");

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
        let mut st = pod.decode_struct()?;
        let read_fd = self.take_fd(st.field()?.decode::<Fd>()?)?;
        let write_fd = self.take_fd(st.field()?.decode::<Fd>()?)?;
        let memfd = st.field()?.decode::<i32>()?;
        let offset = st.field()?.decode::<i32>()?;
        let size = st.field()?.decode::<i32>()?;

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

    #[tracing::instrument(skip(self, pod))]
    fn client_node_set_param(&mut self, index: usize, pod: Pod<&[u64]>) -> Result<()> {
        let mut st = pod.decode_struct()?;
        let param = st.field()?.decode::<id::Param>()?;
        let flags = st.field()?.decode::<i32>()?;
        let obj = st.field()?.decode_object()?;

        let object_type = id::ObjectType::from_id(obj.object_type());
        let object_id = id::Param::from_id(obj.object_id());

        let mut o = obj.as_ref();

        match object_id {
            id::Param::PROPS => {
                while !o.is_empty() {
                    let p = o.property()?;

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

        if let Some(client) = self.client_nodes.get_mut(index) {
            client.params.insert(param, obj.to_owned());
        }

        tracing::info!(?param, flags, ?object_type, ?object_id, ?obj, "Set param");
        Ok(())
    }

    #[tracing::instrument(skip(self, pod))]
    fn client_node_set_io(&mut self, index: usize, pod: Pod<&[u64]>) -> Result<()> {
        let Some(..) = self.client_nodes.get_mut(index) else {
            bail!("Missing client node {index}");
        };

        let mut st = pod.decode_struct()?;
        let id = st.field()?.decode::<id::Io>()?;
        let memid = st.field()?.decode::<i32>()?;
        let offset = st.field()?.decode::<i32>()?;
        let size = st.field()?.decode::<i32>()?;

        tracing::info!(index, ?id, memid, offset, size, "Client node set IO");
        Ok(())
    }

    #[tracing::instrument(skip(self, pod))]
    fn client_node_set_activation(&mut self, index: usize, pod: Pod<&[u64]>) -> Result<()> {
        let mut st = pod.decode_struct()?;
        let node_id = st.field()?.decode::<i32>()?;
        let fd = self.take_fd(st.field()?.decode::<Fd>()?)?;
        let mem_id = st.field()?.decode::<u32>()?;
        let offset = st.field()?.decode::<i32>()?;
        let size = st.field()?.decode::<i32>()?;

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
