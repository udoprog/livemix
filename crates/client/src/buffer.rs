use alloc::vec::Vec;

use bittle::BitsMut;
use protocol::consts;
use protocol::consts::Direction;
use protocol::ffi;
use protocol::flags;
use protocol::id;

use crate::MixId;
use crate::PortId;
use crate::memory::Region;

#[derive(Debug)]
#[non_exhaustive]
pub struct Meta {
    pub ty: id::Meta,
    pub region: Region<[u8]>,
}

#[derive(Debug)]
#[non_exhaustive]
pub struct Data {
    pub ty: id::DataType,
    pub region: Region<[u8]>,
    pub flags: flags::DataFlag,
    pub max_size: usize,
    pub chunk: Region<ffi::Chunk>,
}

#[derive(Debug)]
#[non_exhaustive]
pub struct Buffer {
    pub id: u32,
    pub offset: usize,
    pub size: usize,
    pub metas: Vec<Meta>,
    pub datas: Vec<Data>,
}

#[derive(Debug)]
#[non_exhaustive]
pub struct Buffers {
    pub direction: Direction,
    pub port_id: PortId,
    pub mix_id: MixId,
    pub flags: u32,
    pub buffers: Vec<Buffer>,
    /// The buffers which are available in this set.
    pub available: u128,
}

impl Buffers {
    /// Reset the buffer.
    pub(crate) fn reset(&mut self) {
        self.available = 0;

        for n in 0..self.buffers.len() {
            self.available.set_bit(n as u32);
        }
    }
}
