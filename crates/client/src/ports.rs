use core::fmt;
use core::marker::PhantomData;
use core::mem;
use core::ptr::NonNull;

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::VecDeque;
use std::collections::btree_map::Entry;

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use anyhow::{Result, bail};
use bittle::Bits;
use bittle::BitsMut;
use pod::AsSlice;
use pod::PodItem;
use pod::PodSink;
use pod::PodStream;
use pod::Readable;
use pod::Writable;
use pod::{ChoiceType, DynamicBuf, Object, Type};
use protocol::consts::{self, Direction};
use protocol::flags::{ParamFlag, Status};
use protocol::id::{
    self, AudioFormat, FormatKey, MediaSubType, MediaType, ObjectType, Param, ParamBuffersKey,
    ParamIoKey, ParamMetaKey,
};
use protocol::{ffi, flags, object};
use tracing::Level;

use crate::buffer::Buffer;
use crate::ptr::volatile;
use crate::{Buffers, Region};

/// The identifier of a port.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct PortId(u32);

impl PortId {
    /// Construct a new port identifier.
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    /// Get the index of the port.
    ///
    /// Since it was constructed from a `usize`, it can always be safely coerced
    /// into one.
    fn index(self) -> usize {
        self.0 as usize
    }
}

impl fmt::Display for PortId {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Writable for PortId {
    #[inline]
    fn write_into(&self, pod: &mut impl PodSink) -> Result<(), pod::Error> {
        pod.next()?.write(self.0)
    }
}

impl<'de> Readable<'de> for PortId {
    #[inline]
    fn read_from(pod: &mut impl PodStream<'de>) -> Result<Self, pod::Error> {
        let pod = pod.next()?;
        Ok(PortId(pod.read_sized::<u32>()?))
    }
}

/// The identifier of a mix.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct MixId(u32);

impl MixId {
    /// The zero mix identifier.
    pub const ZERO: Self = Self(0);

    /// An invalid mix identifier.
    pub const INVALID: Self = Self(u32::MAX);

    /// Construct a new mix identifier.
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    /// Convert the `MixId` into a `usize`.
    pub(crate) fn index(self) -> usize {
        self.0 as usize
    }
}

impl fmt::Display for MixId {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl fmt::Debug for MixId {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0 == u32::MAX {
            write!(f, "MixId::INVALID")
        } else {
            f.write_fmt(format_args!("MixId({})", self.0))
        }
    }
}

impl Writable for MixId {
    #[inline]
    fn write_into(&self, pod: &mut impl PodSink) -> Result<(), pod::Error> {
        pod.next()?.write(self.0)
    }
}

impl<'de> Readable<'de> for MixId {
    #[inline]
    fn read_from(pod: &mut impl PodStream<'de>) -> Result<Self, pod::Error> {
        Ok(MixId(pod.next()?.read::<i32>()?.cast_unsigned()))
    }
}

/// A port parameter with associated flags.
#[derive(Debug)]
#[non_exhaustive]
pub struct PortParam<B = DynamicBuf>
where
    B: AsSlice,
{
    pub value: Object<B>,
    pub flags: u32,
}

impl<B> PortParam<B>
where
    B: AsSlice,
{
    /// Construct a port parameter with empty flags.
    #[inline]
    pub fn new(value: Object<B>) -> Self {
        Self { value, flags: 0 }
    }

    /// Construct a port parameter with associated flags.
    #[inline]
    pub fn with_flags(value: Object<B>, flags: u32) -> Self {
        Self { value, flags }
    }
}

/// A set of allocated buffers for a port.
pub struct PortBuffers {
    /// The buffers associated with the port.
    buffers: Vec<Buffers>,
    /// Bit sets, one per mix, indicating whether a buffer is currently in use
    /// with a particular "mix" or peer.
    mixes: Vec<u128>,
}

impl PortBuffers {
    /// Construct a new set of buffers with a pre-determined size of mixes.
    pub fn new(direction: Direction) -> Self {
        let mixes_len = if direction == Direction::OUTPUT {
            64
        } else {
            0
        };

        Self {
            buffers: Vec::new(),
            mixes: vec![0; mixes_len],
        }
    }
}

impl PortBuffers {
    /// Get the next input buffer.
    pub fn next_input<'io>(&mut self, mix: &'io mut PortMix) -> Option<PortInputBuffer<'io, '_>> {
        let status = unsafe { volatile!(mix.region, status).read() };

        if !(status & Status::HAVE_DATA) {
            return None;
        }

        let id = unsafe { volatile!(mix.region, buffer_id).read() };

        let buffer = self.get_mut(mix.mix_id, id as u32)?;

        Some(PortInputBuffer { io: mix, buffer })
    }

    /// Just get the specified buffer by id.
    pub(crate) fn get_mut(&mut self, mix_id: MixId, buffer_id: u32) -> Option<&mut Buffer> {
        let index = usize::try_from(buffer_id).ok()?;

        self.buffers
            .iter_mut()
            .find(|b| b.mix_id == mix_id)?
            .buffers
            .get_mut(index)
    }

    /// The given mix id has been removed, so clear any reservations that are present on it.
    pub(crate) fn free_all(&mut self, mix_id: MixId) {
        debug_assert_ne!(mix_id, MixId::INVALID);

        let Some(mix) = self.mixes.get_mut(mix_id.index()) else {
            return;
        };

        let mix = mem::take(mix);

        let Some(buf) = self.buffers.first_mut() else {
            return;
        };

        debug_assert_eq!(buf.mix_id, MixId::INVALID);

        for buffer_id in mix.iter_ones() {
            if self.mixes.iter().all(|m| !m.test_bit(buffer_id)) {
                buf.available.clear_bit(buffer_id);
            }
        }
    }

    /// Free the given buffer by id.
    fn free(&mut self, mix_id: MixId, buffer_id: u32) {
        if let Some(mix) = self.mixes.get_mut(mix_id.index()) {
            mix.clear_bit(buffer_id);
        }

        let Some(buf) = self.buffers.first_mut() else {
            return;
        };

        debug_assert_eq!(buf.mix_id, MixId::INVALID);

        if self.mixes.iter().all(|m| !m.test_bit(buffer_id)) {
            buf.available.clear_bit(buffer_id);
        }
    }

    /// Get the next free buffer in the set.
    pub fn next_output<'mix>(
        &mut self,
        mixes: &'mix mut PortMixes,
    ) -> Option<PortOutputBuffer<'mix, '_>> {
        // Recycle buffers before we try and acquire a new one.
        for buf in &mut mixes.buffers {
            let status = unsafe { volatile!(buf.region, status).read() };
            let target_id = unsafe { volatile!(buf.region, buffer_id).read() };

            if status & Status::NEED_DATA && target_id >= 0 {
                self.free(buf.mix_id, target_id as u32);
            }
        }

        let buf = self.buffers.first_mut()?;
        debug_assert_eq!(buf.mix_id, MixId::INVALID);

        let id = buf.available.iter_zeros().next()?;
        let b = buf.buffers.get_mut(id as usize)?;

        buf.available.set_bit(id);

        for io_buffer in &mixes.buffers {
            if let Some(mix) = self.mixes.get_mut(io_buffer.mix_id.index()) {
                mix.set_bit(id);
            }
        }

        let b = NonNull::from(b);
        let port_buffers = NonNull::from(self);

        Some(PortOutputBuffer {
            io: mixes,
            port_buffers,
            buf: b,
            _marker: PhantomData,
        })
    }
}

/// An allocated input buffer.
#[must_use = "In order for the input buffer to be used again, `need_data` must be called"]
pub struct PortInputBuffer<'io, 'buf> {
    /// The IO buffers for the port.
    io: &'io mut PortMix,
    /// The buffer that is being read.
    buffer: &'buf mut Buffer,
}

impl PortInputBuffer<'_, '_> {
    /// Access the underlying buffer mutably.
    pub fn buffer_mut(&mut self) -> &mut Buffer {
        &mut self.buffer
    }

    /// The mix the input buffer is associated with.
    pub fn mix_id(&self) -> MixId {
        self.io.mix_id
    }

    /// Mark the input buffer as needing more data.
    pub fn need_data(self) -> Result<()> {
        unsafe { volatile!(self.io.region, status).replace(flags::Status::NEED_DATA) };
        Ok(())
    }
}

/// An output buffer related to a port.
#[must_use = "In order for the output buffer to be used, `have_data` must be called"]
pub struct PortOutputBuffer<'io, 'buf> {
    io: &'io mut PortMixes,
    port_buffers: NonNull<PortBuffers>,
    pub buf: NonNull<Buffer>,
    _marker: PhantomData<&'buf mut PortBuffers>,
}

impl PortOutputBuffer<'_, '_> {
    /// Access the underlying buffer.
    pub fn buffer_mut(&mut self) -> &mut Buffer {
        // SAFETY: Returning this mutable reference prevents calling `have_data`
        // which requires mixed access again.
        unsafe { self.buf.as_mut() }
    }

    /// Mark the output buffer as having data.
    pub fn have_data(mut self) -> Result<()> {
        let id = unsafe { self.buf.as_ref().id };
        let port_buffers = unsafe { self.port_buffers.as_mut() };

        // Recycle buffers.
        for buf in &mut self.io.buffers {
            let status = unsafe { volatile!(buf.region, status).read() };

            if !(status & Status::NEED_DATA) && !(status & Status::OK) {
                port_buffers.free(buf.mix_id, id);
                continue;
            }

            unsafe {
                volatile!(buf.region, buffer_id).replace(id as i32);
                volatile!(buf.region, status).replace(flags::Status::HAVE_DATA);
            };
        }

        Ok(())
    }
}

/// The IO area for a port.
///
/// This is keyed by mix, since it might refer to multiple links.
pub struct PortMix {
    /// The mix identifier.
    pub(crate) mix_id: MixId,
    /// The memory region.
    pub(crate) region: Region<ffi::IoBuffers>,
}

/// The IO buffers for a port.
#[derive(Default)]
pub struct PortMixes {
    pub buffers: Vec<PortMix>,
}

impl PortMixes {
    /// Iterate over port mixes.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut PortMix> {
        self.buffers.iter_mut()
    }
}

/// The definition of a port.
#[non_exhaustive]
pub struct Port {
    /// The direction of the port.
    pub direction: Direction,
    /// Identifier of the port, unique per direction.
    pub id: PortId,
    /// The name of the port.
    pub name: String,
    /// List of available buffers for the port.
    pub port_buffers: PortBuffers,
    /// The IO clock region for the port.
    pub io_clock: Option<Region<ffi::IoClock>>,
    /// The IO position region for the port.
    pub io_position: Option<Region<ffi::IoPosition>>,
    /// The IO buffers region for the port.
    pub mixes: PortMixes,
    /// The audio format of the port.
    pub format: Option<object::AudioFormat>,
    /// The mix information for the port.
    ///
    /// This tells you the peers are connected to the port.
    pub mix_info: PortMixInfo,
    modified: bool,
    param_values: BTreeMap<Param, Vec<PortParam<DynamicBuf>>>,
    param_flags: BTreeMap<Param, ParamFlag>,
}

impl Port {
    /// Take the modified state of the port.
    #[inline]
    pub(crate) fn take_modified(&mut self) -> bool {
        mem::take(&mut self.modified)
    }

    /// Set a parameter flag.
    fn set_flag(&mut self, id: Param, flag: flags::ParamFlag) {
        match self.param_flags.entry(id) {
            Entry::Vacant(e) => {
                e.insert(flag);
            }
            Entry::Occupied(e) => {
                if e.get().contains(flag) {
                    return;
                }

                *e.into_mut() |= flag;
            }
        }

        self.modified = true;
    }

    /// Set a parameter flag.
    pub fn set_read(&mut self, id: Param) {
        self.set_flag(id, flags::ParamFlag::READ);
    }

    /// Set that a parameter is writable.
    pub fn set_write(&mut self, id: Param) {
        self.set_flag(id, flags::ParamFlag::WRITE);
    }

    /// Set a parameter on the port to the given values.
    #[inline]
    pub fn set_param(
        &mut self,
        id: Param,
        values: impl IntoIterator<Item = PortParam<impl AsSlice>, IntoIter: ExactSizeIterator>,
    ) -> Result<()> {
        let mut iter = values.into_iter();
        let mut params = Vec::with_capacity(iter.len());

        for param in iter {
            params.push(PortParam::with_flags(
                param.value.as_ref().to_owned()?,
                param.flags,
            ));
        }

        self.param_values.insert(id, params);
        self.set_flag(id, flags::ParamFlag::READ);
        self.modified = true;
        Ok(())
    }

    /// Push a parameter.
    ///
    /// This will append the value to the existing set of parameters of the
    /// given type.
    #[inline]
    pub fn push_param(&mut self, id: Param, value: PortParam<impl AsSlice>) -> Result<()> {
        self.param_values
            .entry(id)
            .or_default()
            .push(PortParam::with_flags(
                value.value.as_ref().to_owned()?,
                value.flags,
            ));

        self.set_flag(id, flags::ParamFlag::READ);
        self.modified = true;
        Ok(())
    }

    /// Remove a parameter from the port and return the values of the removed
    /// parameter if it exists.
    #[inline]
    pub fn remove_param(&mut self, id: Param) -> Option<Vec<PortParam>> {
        let param = self.param_values.remove(&id)?;

        // If we remove a parameter it is no longer readable.
        let flag = self.param_flags.entry(id).or_default();
        *flag ^= flags::ParamFlag::READ;

        self.modified = true;
        Some(param)
    }

    /// Get the values of a parameter.
    pub fn get_param(&self, id: Param) -> &[PortParam<DynamicBuf>] {
        self.param_values
            .get(&id)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    /// Get parameters from the port.
    pub(crate) fn param_values(&self) -> &BTreeMap<Param, Vec<PortParam<impl AsSlice>>> {
        &self.param_values
    }

    /// Get parameters from the port.
    pub(crate) fn param_flags(&self) -> &BTreeMap<Param, flags::ParamFlag> {
        &self.param_flags
    }

    /// Replace the current set of buffers for this port.
    #[inline]
    #[tracing::instrument(skip(self, f, buffers), fields(port_id = ?self.id, mix_id = ?buffers.mix_id), ret(level = Level::TRACE))]
    pub(crate) fn replace_buffers(&mut self, mut buffers: Buffers, mut f: impl FnMut(Buffers)) {
        // Fox INVALID mix id, the provided buffer applies to all mixes.
        if buffers.mix_id == MixId::INVALID {
            for buf in self.port_buffers.buffers.drain(..) {
                f(buf);
            }

            self.port_buffers.buffers.push(buffers);
        } else {
            for buf in self
                .port_buffers
                .buffers
                .extract_if(.., |b| b.mix_id == buffers.mix_id)
            {
                f(buf);
            }

            self.port_buffers.buffers.push(buffers);
        }
    }
}

pub struct PortMixInfoPeer {
    /// The identifier of the mix.
    pub mix_id: MixId,
    /// The connected peer.
    pub peer_id: PortId,
    /// The properties of the peer.
    pub properties: BTreeMap<String, String>,
}

#[derive(Default)]
pub struct PortMixInfo {
    peers: Vec<PortMixInfoPeer>,
}

impl PortMixInfo {
    /// Insert a peer ID for the given mix.
    pub fn insert(&mut self, mix_id: MixId, peer_id: PortId, properties: BTreeMap<String, String>) {
        self.peers.push(PortMixInfoPeer {
            mix_id,
            peer_id,
            properties,
        });
    }

    /// Remove a peer ID for the given mix.
    pub fn remove(&mut self, mix_id: MixId) {
        self.peers.retain(|peer| peer.mix_id != mix_id);
    }
}

#[derive(Default)]
pub struct Ports {
    input_ports: Vec<Port>,
    output_ports: Vec<Port>,
}

impl Ports {
    /// Construct a new collection of ports.
    #[inline]
    pub(crate) fn new() -> Self {
        Self {
            input_ports: Vec::new(),
            output_ports: Vec::new(),
        }
    }

    /// Access input ports.
    pub fn inputs(&self) -> &[Port] {
        &self.input_ports
    }

    /// Access input ports mutably.
    pub fn inputs_mut(&mut self) -> &mut [Port] {
        &mut self.input_ports
    }

    /// Access output ports.
    pub fn outputs(&self) -> &[Port] {
        &self.output_ports
    }

    /// Access output ports mutably.
    pub fn outputs_mut(&mut self) -> &mut [Port] {
        &mut self.output_ports
    }

    /// Insert a new port in the specified direction and return the inserted
    /// port for configuration.
    pub fn insert(&mut self, direction: Direction) -> Result<&mut Port> {
        let ports = self.get_direction_mut(direction)?;

        let Ok(id) = u32::try_from(ports.len()) else {
            bail!("Too many ports in {direction:?} direction");
        };

        let id = PortId(id);

        let mut port = Port {
            direction,
            id,
            modified: true,
            name: String::new(),
            port_buffers: PortBuffers::new(direction),
            io_clock: None,
            io_position: None,
            mixes: PortMixes::default(),
            format: None,
            param_values: BTreeMap::new(),
            param_flags: BTreeMap::new(),
            mix_info: PortMixInfo::default(),
        };

        ports.push(port);
        Ok(&mut ports[id.index()])
    }

    /// Get a port.
    pub fn get(&self, direction: Direction, id: PortId) -> Result<&Port> {
        let ports = self.get_direction(direction)?;

        let Some(port) = ports.get(id.index()) else {
            bail!("Port {id} not found in {direction:?} ports");
        };

        Ok(port)
    }

    /// Get a port mutably.
    pub fn get_mut(&mut self, direction: Direction, id: PortId) -> Result<&mut Port> {
        let ports = self.get_direction_mut(direction)?;

        let Some(port) = ports.get_mut(id.index()) else {
            bail!("Port {id} not found in {direction:?} ports");
        };

        Ok(port)
    }

    #[inline]
    fn get_direction(&self, dir: Direction) -> Result<&Vec<Port>> {
        match dir {
            Direction::INPUT => Ok(&self.input_ports),
            Direction::OUTPUT => Ok(&self.output_ports),
            dir => panic!("Unknown port direction: {dir:?}"),
        }
    }

    #[inline]
    fn get_direction_mut(&mut self, dir: Direction) -> Result<&mut Vec<Port>> {
        match dir {
            Direction::INPUT => Ok(&mut self.input_ports),
            Direction::OUTPUT => Ok(&mut self.output_ports),
            dir => panic!("Unknown port direction: {dir:?}"),
        }
    }
}
