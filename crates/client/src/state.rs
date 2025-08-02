use core::ffi::CStr;
use core::mem;
use core::slice;

use core::time::Duration;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::ffi::CString;
use std::fs::File;
use std::os::fd::{AsRawFd, OwnedFd, RawFd};
use std::time::SystemTime;

use alloc::borrow::ToOwned;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use anyhow::{Context, Result, anyhow, bail, ensure};
use pod::{ChoiceType, DynamicBuf, Fd, Object, Pod, Slice, Struct, Type};
use protocol::EventFd;
use protocol::Prop;
use protocol::buf::RecvBuf;
use protocol::consts;
use protocol::consts::ActivationStatus;
use protocol::ffi;
use protocol::flags;
use protocol::id::{self, AudioFormat, Format, MediaSubType, MediaType, ObjectType, Param};
use protocol::ids::Ids;
use protocol::op;
use protocol::op::{ClientEvent, ClientNodeEvent, CoreEvent, RegistryEvent};
use protocol::poll::{Interest, Token};
use protocol::types::Header;
use protocol::{Connection, Properties, prop};
use slab::Slab;
use tracing::Level;

use crate::activation::Activation;
use crate::buffer::{self, Buffer};
use crate::ptr::{atomic, volatile};
use crate::{Buffers, Client, Memory, Ports, Region};

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
#[allow(unused)]
struct ClientNodeState {
    id: u32,
    read_fd: Option<EventFd>,
    write_token: Token,
    write_fd: Option<EventFd>,
    read_token: Token,
    /// Activation record for this node.
    activation: Option<Region<ffi::NodeActivation>>,
    /// Activation records for dependent nodes.
    peer_activations: Slab<Activation>,
    params: BTreeMap<Param, Vec<Object<DynamicBuf>>>,
    ports: Ports,
    io_clock: Option<Region<ffi::IoClock>>,
    io_control: Option<Region<()>>,
    io_position: Option<Region<ffi::IoPosition>>,
    modified: bool,
    buf: Vec<f32>,
}

impl ClientNodeState {
    pub(crate) fn new(
        id: u32,
        ports: Ports,
        write_token: Token,
        read_token: Token,
    ) -> Result<Self> {
        let mut params = BTreeMap::new();
        let mut pod = pod::array();

        pod.as_mut()
            .write_object(ObjectType::FORMAT, Param::ENUM_FORMAT, |obj| {
                obj.property(Format::MEDIA_TYPE).write(MediaType::AUDIO)?;
                obj.property(Format::MEDIA_SUB_TYPE)
                    .write(MediaSubType::DSP)?;
                obj.property(Format::AUDIO_FORMAT)
                    .write(AudioFormat::F32P)?;
                obj.property(Format::AUDIO_CHANNELS).write(1u32)?;
                obj.property(Format::AUDIO_RATE).write(44100u32)?;
                Ok(())
            })?;

        params.insert(
            Param::ENUM_FORMAT,
            vec![pod.take().read_object()?.to_owned()?],
        );

        pod.as_mut()
            .write_object(ObjectType::FORMAT, Param::FORMAT, |obj| {
                obj.property(Format::MEDIA_TYPE).write(MediaType::AUDIO)?;
                obj.property(Format::MEDIA_SUB_TYPE)
                    .write(MediaSubType::DSP)?;
                obj.property(Format::AUDIO_FORMAT)
                    .write(AudioFormat::F32P)?;
                obj.property(Format::AUDIO_CHANNELS).write(1u32)?;
                obj.property(Format::AUDIO_RATE).write(44100u32)?;
                Ok(())
            })?;

        params.insert(Param::FORMAT, vec![pod.take().read_object()?.to_owned()?]);

        Ok(Self {
            id,
            ports,
            write_fd: None,
            read_fd: None,
            write_token,
            read_token,
            activation: None,
            peer_activations: Slab::new(),
            params,
            io_control: None,
            io_clock: None,
            io_position: None,
            modified: true,
            buf: Vec::with_capacity(1024 * 1024),
        })
    }

    /// Set a parameter for the node.
    #[inline]
    fn set_param(&mut self, param: Param, value: Object<DynamicBuf>) {
        self.params.insert(param, vec![value]);
        self.modified = true;
    }

    /// Remove a parameter for the node.
    #[inline]
    fn remove_param(&mut self, param: Param) {
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
    NodeActive { client: usize },
    NodeUpdate { client: usize },
    NodeStart { client: usize },
    NodeReadInterest { client: usize },
    Process,
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
pub struct State {
    tick: usize,
    writer: hound::WavWriter<File>,
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
    pub fn new(connection: Connection) -> Result<Self> {
        let mut ids = Ids::new();

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

        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 44100,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };

        Ok(Self {
            tick: 0,
            writer: hound::WavWriter::new(File::create("output.wav")?, spec)?,
            c: Client::new(connection),
            core: CoreState::default(),
            client,
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
        })
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
    pub fn run(&mut self, recv: &mut RecvBuf) -> Result<()> {
        'next: loop {
            while let Some(op) = self.ops.pop_front() {
                match op {
                    Op::CoreHello => {
                        self.c.core_hello()?;
                        self.c.client_update_properties(&self.client.properties)?;
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
                    Op::NodeActive { client } => {
                        if let Some(node) = self.client_nodes.get(client) {
                            self.c.client_node_set_active(node.id, true)?;
                        }
                    }
                    Op::NodeUpdate { client } => {
                        if let Some(node) = self.client_nodes.get_mut(client) {
                            if node.take_modified() {
                                self.c.client_node_update(node.id, 4, 4, &node.params)?;
                            }

                            for port in node.ports.inputs_mut() {
                                if !port.take_modified() {
                                    continue;
                                }

                                self.c.client_node_port_update(
                                    node.id,
                                    consts::Direction::INPUT,
                                    port.id(),
                                    &port.name,
                                    port.params(),
                                )?;
                            }

                            for port in node.ports.outputs_mut() {
                                if !port.take_modified() {
                                    continue;
                                }

                                self.c.client_node_port_update(
                                    node.id,
                                    consts::Direction::OUTPUT,
                                    port.id(),
                                    &port.name,
                                    port.params(),
                                )?;
                            }
                        }
                    }
                    Op::NodeStart { client } => {
                        if let Some(node) = self.client_nodes.get(client)
                            && let Some(a) = &node.activation
                        {
                            if atomic!(a, status).compare_exchange(
                                ActivationStatus::INACTIVE,
                                ActivationStatus::NOT_TRIGGERED,
                            ) {
                                tracing::info!("Starting node");
                            }
                        } else {
                            tracing::error!(
                                ?client,
                                "Cannot start node, missing activation for client"
                            );
                        }
                    }
                    Op::NodeReadInterest { client } => {
                        self.node_read_interest(client)?;
                    }
                    Op::Process => {
                        self.process()?;
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

                let Some(pod) = frame(recv, &self.header)? else {
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
        let mut samples = 0;
        let mut sum = 0.0;

        for (_, node) in self.client_nodes.iter_mut() {
            for sample in node.buf.drain(..) {
                self.writer.write_sample(sample)?;
                sum += sample;
                samples += 1;
            }
        }

        tracing::warn!(samples, sum, len = self.writer.len());
        self.writer.flush()?;

        for (_, node) in &mut self.client_nodes {
            if let Some(a) = &node.activation {
                tracing::info!(
                    "Activation status: {:?}",
                    crate::ptr::volatile!(a, status).read()
                );

                tracing::info!(
                    "Activation driver id: {:?}",
                    crate::ptr::volatile!(a, driver_id).read()
                );
            }

            for (_, a) in &node.peer_activations {
                tracing::info!(
                    "Target status: {:?}",
                    crate::ptr::volatile!(a.region, status).read()
                );

                tracing::info!(
                    "Target state: {:?}",
                    crate::ptr::volatile!(a.region, state).read()
                );

                tracing::info!(
                    "Target driver id: {:?}",
                    crate::ptr::volatile!(a.region, driver_id).read()
                );
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

        let Some(read_fd) = &client.read_fd else {
            tracing::warn!(?client, "No read file descriptor for client");
            return Ok(());
        };

        let Some(ev) = read_fd.read()? else {
            return Ok(());
        };

        if let Some(a) = &client.activation {
            self.ops.push_back(Op::Process);
        }

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

        let port = ports.insert(consts::Direction::OUTPUT)?;
        port.name = String::from("output");

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

    fn node_read_interest(&mut self, client: usize) -> Result<()> {
        let Some(node) = self.client_nodes.get(client) else {
            bail!("No client found for index {client}");
        };

        if let Some(read_fd) = &node.read_fd {
            self.read_to_client.insert(node.read_token, client);
            self.add_interest.push_back((
                read_fd.as_raw_fd(),
                node.read_token,
                Interest::READ | Interest::HUP | Interest::ERROR,
            ));
        }

        if let Some(write_fd) = &node.write_fd {
            self.write_to_client.insert(node.write_token, client);
            self.add_interest.push_back((
                write_fd.as_raw_fd(),
                node.write_token,
                Interest::HUP | Interest::ERROR,
            ));
        }

        Ok(())
    }

    #[tracing::instrument(skip(self))]
    fn process(&mut self) -> Result<()> {
        self.tick = self.tick.wrapping_add(1);
        let start = SystemTime::now();

        for (_, node) in &mut self.client_nodes {
            if let Some(activation) = &node.activation {
                let previous_status = atomic!(activation, status).swap(ActivationStatus::AWAKE);

                if previous_status != ActivationStatus::TRIGGERED {
                    tracing::warn!(?previous_status, "Expected TRIGGERED");
                }
            }

            let Some(activation) = &node.activation else {
                continue;
            };

            for port in node.ports.inputs_mut() {
                let (Some(buffers), Some(io_buffers)) = (&port.buffers, &port.io_buffers) else {
                    tracing::warn!("Input missing buffers");
                    continue;
                };

                let status = volatile!(io_buffers, status).read();

                if status != flags::Status::HAVE_DATA {
                    tracing::trace!("Input io status is not HAVE_DATA: {status:?}");
                    continue;
                };

                let id = volatile!(io_buffers, buffer_id).read();

                let Some(buffer) = buffers.buffers.get(id as usize) else {
                    bail!("Input no buffer with id {id} for port {}", port.id);
                };

                let meta = &buffer.metas[0];
                let data = &buffer.datas[0];

                let header;
                let samples;

                unsafe {
                    header = meta.region.cast::<ffi::MetaHeader>()?.read();

                    let chunk = data.chunk.as_ref();
                    let offset = chunk.offset as usize % data.max_size;
                    let size = (chunk.size as usize - offset).min(data.max_size);

                    samples = size / mem::size_of::<f32>();

                    node.buf.reserve(samples);
                    node.buf
                        .as_mut_ptr()
                        .add(samples)
                        .cast::<f32>()
                        .copy_from_nonoverlapping(
                            data.region.as_ptr().wrapping_add(offset).cast::<f32>(),
                            samples,
                        );
                    node.buf.set_len(node.buf.len() + samples);
                }

                tracing::warn!(samples, len = node.buf.len());

                let old_read = volatile!(io_buffers, status).replace(flags::Status::NEED_DATA);

                if self.tick % 100 == 0 {
                    tracing::warn!(?data.flags, ?header, ?id, ?old_read);
                }
            }

            for port in node.ports.outputs_mut() {
                let (Some(buffers), Some(io_buffers)) = (&mut port.buffers, &port.io_buffers)
                else {
                    tracing::trace!("Output missing buffers");
                    continue;
                };

                let status = volatile!(io_buffers, status).read();

                if status != flags::Status::NEED_DATA {
                    tracing::trace!("Output status is not NEED_DATA: {status:?}");
                    continue;
                };

                let id = volatile!(io_buffers, buffer_id).read();

                let Some(buffer) = buffers.buffers.get_mut(1) else {
                    bail!("Output no buffer with id {id} for port {}", port.id);
                };

                let meta = &buffer.metas[0];
                let data = &mut buffer.datas[0];

                let header;

                unsafe {
                    let chunk = data.chunk.as_mut();

                    header = meta.region.cast::<ffi::MetaHeader>()?.read();

                    let size = (chunk.size as usize).min(data.max_size);
                    let len = size.min(node.buf.len().wrapping_mul(mem::size_of::<f32>()));

                    data.region
                        .as_mut_ptr()
                        .cast::<u8>()
                        .copy_from_nonoverlapping(node.buf.as_ptr().cast::<u8>(), len);

                    chunk.offset = 0;
                    chunk.size = len as u32;
                    chunk.stride = 4;
                }

                volatile!(io_buffers, buffer_id).write(buffer.id);
                let old_write = volatile!(io_buffers, status).replace(flags::Status::HAVE_DATA);

                if self.tick % 100 == 0 {
                    tracing::warn!(?data.flags, ?header, ?id, ?old_write);
                }
            }

            if let Some(activation) = &node.activation {
                let previous_status =
                    atomic!(activation, status).swap(ActivationStatus::NOT_TRIGGERED);

                if previous_status != ActivationStatus::AWAKE {
                    tracing::warn!(?previous_status, "Expected AWAKE");
                }
            }

            for (_, a) in &node.peer_activations {
                a.signal()?;
            }
        }

        let elapsed = start.elapsed().context("Failed to get elapsed time")?;
        tracing::warn!(?elapsed);
        Ok(())
    }

    fn core(&mut self, pod: Pod<Slice<'_>>) -> Result<()> {
        let op = CoreEvent::from_raw(self.header.op());
        tracing::info!("Event: {op}");

        match op {
            CoreEvent::INFO => {
                self.core_info_event(pod).context(op)?;
            }
            CoreEvent::DONE => {
                self.core_done_event(pod).context(op)?;
            }
            CoreEvent::PING => {
                self.core_ping_event(pod).context(op)?;
            }
            CoreEvent::ERROR => {
                self.core_error_event(pod).context(op)?;
            }
            CoreEvent::BOUND_ID => {
                self.core_bound_id_event(pod).context(op)?;
            }
            CoreEvent::ADD_MEM => {
                self.core_add_mem_event(pod).context(op)?;
            }
            CoreEvent::DESTROY => {
                self.core_destroy(pod).context(op)?;
            }
            op => {
                tracing::warn!("Unsupported event: {op}");
            }
        }

        Ok(())
    }

    fn client(&mut self, pod: Pod<Slice<'_>>) -> Result<()> {
        let op = ClientEvent::from_raw(self.header.op());

        match op {
            ClientEvent::INFO => {
                self.client_info(pod).context(op)?;
            }
            ClientEvent::ERROR => {
                self.client_error(pod).context(op)?;
            }
            op => {
                tracing::warn!("Unsupported event: {op}");
            }
        }

        Ok(())
    }

    fn dynamic(&mut self, pod: Pod<Slice<'_>>) -> Result<()> {
        let Some(kind) = self.local_id_to_kind.get(&self.header.id()) else {
            tracing::warn!(?self.header, "Unknown receiver");
            return Ok(());
        };

        match *kind {
            Kind::Registry => {
                let op = RegistryEvent::from_raw(self.header.op());
                tracing::info!("Event: {op}");

                match op {
                    RegistryEvent::GLOBAL => {
                        self.registry_global(pod).context(op)?;
                    }
                    RegistryEvent::GLOBAL_REMOVE => {
                        self.registry_global_remove(pod).context(op)?;
                    }
                    op => {
                        tracing::warn!(?op, "Registry unsupported op");
                    }
                }
            }
            Kind::ClientNode(index) => {
                let op = ClientNodeEvent::from_raw(self.header.op());
                tracing::info!("Event: {op}");

                match op {
                    ClientNodeEvent::TRANSPORT => {
                        self.client_node_transport(index, pod).context(op)?;
                    }
                    ClientNodeEvent::SET_PARAM => {
                        self.client_node_set_param(index, pod).context(op)?;
                    }
                    ClientNodeEvent::SET_IO => {
                        self.client_node_set_io(index, pod).context(op)?;
                    }
                    ClientNodeEvent::COMMAND => {
                        self.client_node_command(index, pod).context(op)?;
                    }
                    ClientNodeEvent::PORT_SET_PARAM => {
                        self.client_node_port_set_param(index, pod).context(op)?;
                    }
                    ClientNodeEvent::USE_BUFFERS => {
                        self.client_node_use_buffers(index, pod).context(op)?;
                    }
                    ClientNodeEvent::PORT_SET_IO => {
                        self.client_node_port_set_io(index, pod).context(op)?;
                    }
                    ClientNodeEvent::SET_ACTIVATION => {
                        self.client_node_set_activation(index, pod).context(op)?;
                    }
                    ClientNodeEvent::PORT_SET_MIX_INFO => {
                        self.client_node_set_mix_info(index, pod).context(op)?;
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
    fn core_info_event(&mut self, pod: Pod<Slice<'_>>) -> Result<()> {
        let mut st = pod.read_struct()?;
        let id = st.field()?.read_sized::<u32>()?;
        let cookie = st.field()?.read_sized::<i32>()?;
        let user_name = st.field()?.read_unsized::<str>()?.to_owned();
        let host_name = st.field()?.read_unsized::<str>()?.to_owned();
        let version = st.field()?.read_unsized::<str>()?.to_owned();
        let name = st.field()?.read_unsized::<str>()?.to_owned();
        let change_mask = st.field()?.read_sized::<u64>()?;

        let mut props = st.field()?.read_struct()?;

        if change_mask & 0x1 != 0 {
            let n_items = props.field()?.read_sized::<i32>()?;

            for _ in 0..n_items {
                let key = props.field()?.read_unsized::<str>()?.to_owned();
                let value = props.field()?.read_unsized::<str>()?.to_owned();
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
    fn core_done_event(&mut self, pod: Pod<Slice<'_>>) -> Result<()> {
        let (id, seq) = pod.read_struct()?.read::<(i32, i32)>()?;

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
    fn core_ping_event(&mut self, pod: Pod<Slice<'_>>) -> Result<()> {
        let mut st = pod.read_struct()?;
        let id = st.field()?.read_sized()?;
        let seq = st.field()?.read_sized()?;

        tracing::debug!("Core ping {id} with seq {seq}");
        self.ops.push_back(Op::Pong { id, seq });
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn core_error_event(&mut self, pod: Pod<Slice<'_>>) -> Result<()> {
        let mut st = pod.read_struct()?;
        let id = st.field()?.read_sized::<i32>()?;
        let seq = st.field()?.read_sized::<i32>()?;
        let res = st.field()?.read_sized::<i32>()?;
        let error = st.field()?.read_unsized::<str>()?.to_owned();

        tracing::error!(id, seq, res, error);
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn core_bound_id_event(&mut self, pod: Pod<Slice<'_>>) -> Result<()> {
        let mut st = pod.read_struct()?;
        let local_id = st.field()?.read_sized::<u32>()?;
        let global_id = st.field()?.read_sized::<u32>()?;
        self.globals.insert(local_id, global_id);

        tracing::debug!(local_id, global_id);
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn core_add_mem_event(&mut self, pod: Pod<Slice<'_>>) -> Result<()> {
        let (id, ty, fd, flags) = pod
            .read_struct()?
            .read::<(u32, id::DataType, Fd, flags::MemBlock)>()?;

        let fd = self.take_fd(fd)?;

        let Some(fd) = fd else {
            self.memory.remove(id);
            return Ok(());
        };

        self.memory.insert(id, ty, fd, flags)?;
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn core_destroy(&mut self, pod: Pod<Slice<'_>>) -> Result<()> {
        let mut st = pod.read_struct()?;
        let id = st.field()?.read_sized::<u32>()?;

        tracing::debug!(id);
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn client_info(&mut self, pod: Pod<Slice<'_>>) -> Result<()> {
        let mut st = pod.read_struct()?;
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
    fn client_error(&mut self, pod: Pod<Slice<'_>>) -> Result<()> {
        let mut st = pod.read_struct()?;
        let id = st.field()?.read_sized::<i32>()?;
        let res = st.field()?.read_sized::<i32>()?;
        let error = st.field()?.read_unsized::<str>()?.to_owned();
        tracing::error!(id, res, error, "Client errored");
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn registry_global(&mut self, pod: Pod<Slice<'_>>) -> Result<()> {
        let (id, permissions, ty, version, mut props) =
            pod.read::<Struct<_>>()?.read::<(_, _, _, _, Struct<_>)>()?;

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
                    self.ops.push_back(Op::NodeActive { client: index });
                    self.ops.push_back(Op::NodeUpdate { client: index });
                }
            }
        }

        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn registry_global_remove(&mut self, pod: Pod<Slice<'_>>) -> Result<()> {
        let mut st = pod.read_struct()?;
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
    fn client_node_transport(&mut self, index: usize, pod: Pod<Slice<'_>>) -> Result<()> {
        let mut st = pod.read_struct()?;
        let read_fd = self.take_fd(st.field()?.read_sized::<Fd>()?)?;
        let write_fd = self.take_fd(st.field()?.read_sized::<Fd>()?)?;
        let mem_id = st.field()?.read_sized::<i32>()?;
        let offset = st.field()?.read_sized::<isize>()?;
        let size = st.field()?.read_sized::<usize>()?;

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

        // Set node as not triggered.
        if let Some(activation) = &node.activation {
            let old = atomic!(activation, status).swap(ActivationStatus::NOT_TRIGGERED);

            if old != ActivationStatus::INACTIVE {
                tracing::warn!("Expected node to be INACTIVE, but was {old:?}",);
            }
        }

        tracing::debug!(index, ?read_fd, ?write_fd, mem_id, offset, size,);

        let Some(node) = self.client_nodes.get_mut(index) else {
            bail!("Missing client node {index}");
        };

        node.read_fd = read_fd.map(EventFd::from);
        node.write_fd = write_fd.map(EventFd::from);

        if node.read_fd.is_some() {
            self.ops.push_back(Op::NodeReadInterest { client: index });
        }

        Ok(())
    }

    #[tracing::instrument(skip(self, pod))]
    fn client_node_set_param(&mut self, index: usize, pod: Pod<Slice<'_>>) -> Result<()> {
        let Some(node) = self.client_nodes.get_mut(index) else {
            bail!("Missing client node {index}");
        };

        let mut st = pod.read_struct()?;
        let id = st.field()?.read_sized::<Param>()?;
        let _flags = st.field()?.read_sized::<i32>()?;

        if let Some(obj) = st.field()?.read_option()? {
            tracing::trace!(?id, "set");
            node.set_param(id, obj.read_object()?.to_owned()?);
        } else {
            tracing::trace!(?id, "remove");
            node.remove_param(id);
        }

        self.ops.push_back(Op::NodeUpdate { client: index });
        Ok(())
    }

    #[tracing::instrument(skip(self, pod))]
    fn client_node_set_io(&mut self, index: usize, pod: Pod<Slice<'_>>) -> Result<()> {
        let Some(node) = self.client_nodes.get_mut(index) else {
            bail!("Missing client node {index}");
        };

        let mut st = pod.read_struct()?;
        let id = st.field()?.read_sized::<id::IoType>()?;
        let mem_id = st.field()?.read_sized::<i32>()?;
        let offset = st.field()?.read_sized::<isize>()?;
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
    fn client_node_command(&mut self, index: usize, pod: Pod<Slice<'_>>) -> Result<()> {
        let Some(..) = self.client_nodes.get_mut(index) else {
            bail!("Missing client node {index}");
        };

        let mut st = pod.as_ref().read_struct()?;
        let obj = st.field()?.read_object()?;

        let object_type = id::CommandType::from_id(obj.object_type());
        let object_id = id::NodeCommand::from_id(obj.object_id());

        match object_id {
            id::NodeCommand::START => {
                self.ops.push_back(Op::NodeStart { client: index });
                tracing::info!(?object_type, ?object_id, ?pod);
            }
            _ => {
                tracing::info!(?object_type, ?object_id, ?pod);
            }
        }

        Ok(())
    }

    #[tracing::instrument(skip(self, pod))]
    fn client_node_port_set_param(&mut self, index: usize, pod: Pod<Slice<'_>>) -> Result<()> {
        let Some(node) = self.client_nodes.get_mut(index) else {
            bail!("Missing client node {index}");
        };

        let mut st = pod.read_struct()?;
        let direction = consts::Direction::from_raw(st.field()?.read_sized::<u32>()?);
        let port_id = st.field()?.read_sized::<u32>()?;
        let id = st.field()?.read_sized::<Param>()?;
        let flags = st.field()?.read_sized::<u32>()?;

        let port = node.ports.get_mut(direction, port_id)?;

        if let Some(param) = st.field()?.read_option()? {
            tracing::trace!(?id, "set");
            port.set_param(id, param.read_object()?.to_owned()?, flags)?;
        } else {
            tracing::trace!(?id, "remove");
            port.remove_param(id)?;
        }

        self.ops.push_back(Op::NodeUpdate { client: index });
        Ok(())
    }

    #[tracing::instrument(skip(self, pod))]
    fn client_node_use_buffers(&mut self, index: usize, pod: Pod<Slice<'_>>) -> Result<()> {
        let Some(node) = self.client_nodes.get_mut(index) else {
            bail!("Missing client node {index}");
        };

        let mut st = pod.read_struct()?;

        tracing::warn!(?st);

        let (direction, port_id, mix_id, flags, n_buffers) = st
            .read::<(consts::Direction, u32, i32, u32, u32)>()
            .with_context(|| anyhow!("reading header"))?;

        let mix_id = u32::try_from(mix_id).ok();

        let mut buffers = Vec::new();

        for id in 0..n_buffers {
            let (mem_id, offset, size, n_metas) = st
                .read::<(u32, i32, u32, u32)>()
                .with_context(|| anyhow!("reading buffer {id}"))?;

            let mm = self
                .memory
                .map(mem_id, offset as isize, size as usize)
                .context("mapping buffer")?;

            let mut metas = Vec::new();

            let mut region = mm.clone();

            for _ in 0..n_metas {
                let (ty, size) = st.read::<(id::Meta, u32)>()?;
                self.memory.track(&region);

                metas.push(buffer::Meta {
                    ty,
                    size,
                    region: region.size(size as usize)?,
                });

                region = region.offset(size as usize, 8)?;
            }

            let mut datas = Vec::new();

            let n_datas = st.read::<usize>()?;

            for id in 0..n_datas {
                let chunk = region.clone().size(mem::size_of::<ffi::Chunk>())?.cast()?;
                region = region.offset(mem::size_of::<ffi::Chunk>(), 8)?;
                self.memory.track(&chunk);

                let (ty, data, flags, offset, max_size) = st
                    .read::<(id::DataType, u32, flags::DataFlag, i32, u32)>()
                    .with_context(|| anyhow!("reading data for buffer {id}"))?;

                let Ok(max_size) = usize::try_from(max_size) else {
                    bail!("Invalid max size {max_size} for data type {ty:?}");
                };

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
                    id::DataType::MEM_FD => {
                        let Ok(offset) = isize::try_from(offset) else {
                            bail!("Invalid offset {offset} for data type {ty:?}");
                        };

                        self.memory.map(data, offset, max_size)?
                    }
                    ty => {
                        bail!("Unsupported data type {ty:?} in use buffers");
                    }
                };

                datas.push(buffer::Data {
                    ty,
                    region: region.cast_array()?,
                    flags,
                    max_size,
                    chunk,
                });
            }

            self.memory.free(mm);

            buffers.push(Buffer {
                id,
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
                for meta in buffer.metas {
                    self.memory.free(meta.region);
                }

                for data in buffer.datas {
                    self.memory.free(data.region);
                    self.memory.free(data.chunk);
                }
            }
        }

        Ok(())
    }

    fn client_node_port_set_io(&mut self, index: usize, pod: Pod<Slice<'_>>) -> Result<()> {
        let Some(node) = self.client_nodes.get_mut(index) else {
            bail!("Missing client node {index}");
        };

        let mut st = pod.read_struct()?;
        let direction = consts::Direction::from_raw(st.field()?.read_sized::<u32>()?);
        let port_id = st.field()?.read_sized::<u32>()?;
        let _mix_id = st.field()?.read_sized::<u32>()?;
        let id = st.field()?.read_sized::<id::IoType>()?;
        let mem_id = st.field()?.read_sized::<i32>()?;
        let offset = st.field()?.read_sized::<isize>()?;
        let size = st.field()?.read_sized::<usize>()?;
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
    fn client_node_set_activation(&mut self, index: usize, pod: Pod<Slice<'_>>) -> Result<()> {
        let mut st = pod.read_struct()?;

        let peer_id = st.field()?.read_sized::<u32>()?;
        let fd = self.take_fd(st.field()?.read_sized::<Fd>()?)?;
        let mem_id = st.field()?.read_sized::<i32>()?;
        let offset = st.field()?.read_sized::<isize>()?;
        let size = st.field()?.read_sized::<usize>()?;

        let Some(node) = self.client_nodes.get_mut(index) else {
            bail!("Missing client node {index}");
        };

        let Ok(mem_id) = u32::try_from(mem_id) else {
            node.peer_activations.retain(|_, n| n.peer_id != peer_id);
            return Ok(());
        };

        let Some(fd) = fd else {
            bail!("Missing fd for peer {peer_id} in node {index}");
        };

        let region = self.memory.map(mem_id, offset, size)?;

        let index =
            node.peer_activations
                .insert(Activation::new(peer_id, EventFd::from(fd), region));

        Ok(())
    }

    #[tracing::instrument(skip(self, pod))]
    fn client_node_set_mix_info(&mut self, index: usize, pod: Pod<Slice<'_>>) -> Result<()> {
        let Some(..) = self.client_nodes.get_mut(index) else {
            bail!("Missing client node {index}");
        };

        let st = pod.read_struct()?;
        tracing::info!(?st);
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
