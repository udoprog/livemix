use alloc::vec::Vec;

use protocol::consts;
use protocol::flags;
use protocol::id;

use crate::memory::Region;

#[derive(Debug)]
pub(crate) struct BufferMeta {
    pub(crate) ty: id::MetaType,
    pub(crate) size: u32,
}

#[derive(Debug)]
pub(crate) struct BufferData {
    pub(crate) ty: id::DataType,
    pub(crate) region: Region,
    pub(crate) flags: flags::DataFlag,
    pub(crate) max_size: usize,
}

#[derive(Debug)]
pub(crate) struct Buffer {
    pub(crate) mem_id: u32,
    pub(crate) offset: i32,
    pub(crate) size: u32,
    pub(crate) metas: Vec<BufferMeta>,
    pub(crate) datas: Vec<BufferData>,
}

#[derive(Debug)]
pub(crate) struct Buffers {
    pub(crate) direction: consts::Direction,
    pub(crate) mix_id: u32,
    pub(crate) flags: u32,
    pub(crate) buffers: Vec<Buffer>,
}
