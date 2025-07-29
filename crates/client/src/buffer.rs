use protocol::{consts, flags, id};

#[derive(Debug)]
pub(crate) struct BufferMeta {
    pub(crate) ty: id::MetaType,
    pub(crate) size: u32,
}

#[derive(Debug)]
pub(crate) struct BufferData {
    pub(crate) ty: id::DataType,
    pub(crate) data: u32,
    pub(crate) flags: flags::DataFlag,
    pub(crate) offset: u32,
    pub(crate) max_size: u32,
}

#[derive(Debug)]
pub(crate) struct Buffer {
    pub(crate) mem_type: Option<id::DataType>,
    pub(crate) mem_id: u32,
    pub(crate) offset: u32,
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
