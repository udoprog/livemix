use core::mem::MaybeUninit;

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
    pub region: Region<[MaybeUninit<u8>]>,
}

#[derive(Debug)]
#[non_exhaustive]
pub struct Data {
    pub(crate) ty: id::DataType,
    pub(crate) region: Region<[MaybeUninit<u8>]>,
    pub flags: flags::DataFlag,
    pub chunk: Region<ffi::Chunk>,
}

impl Data {
    /// Read the valid region of the data according to the associated chunk.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the region is valid.
    pub unsafe fn valid_region(&self) -> Option<Region<[u8]>> {
        unsafe {
            let chunk = self.chunk.as_ref();
            let offset = chunk.offset as usize % self.region.len();
            let size = (chunk.size as usize - offset).min(self.region.len());
            Some(self.region.slice(offset, size)?.cast_array_unchecked())
        }
    }

    /// Return the uninitialized region of the data.
    pub fn uninit_region(&self) -> Region<[MaybeUninit<u8>]> {
        self.region.clone()
    }

    /// Write a complete chunk to the data region.
    pub fn write_chunk(&mut self, chunk: ffi::Chunk) {
        /// SAFETY: We assume the chunk region is valid through construction.
        unsafe {
            self.chunk.write(chunk);
        }
    }
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
