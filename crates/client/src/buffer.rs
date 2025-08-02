use alloc::vec::Vec;

use protocol::consts;
use protocol::ffi;
use protocol::flags;
use protocol::id;

use crate::memory::Region;

#[derive(Debug)]
#[non_exhaustive]
pub struct Meta {
    pub ty: id::Meta,
    pub size: u32,
    pub region: Region<()>,
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
    pub mem_id: u32,
    pub offset: i32,
    pub size: u32,
    pub metas: Vec<Meta>,
    pub datas: Vec<Data>,
}

#[derive(Debug)]
#[non_exhaustive]
pub struct Buffers {
    pub direction: consts::Direction,
    pub mix_id: Option<u32>,
    pub flags: u32,
    pub buffers: Vec<Buffer>,
}
