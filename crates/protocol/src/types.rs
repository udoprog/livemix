//! Types which are used in the pipewire protocol.

use core::fmt;

use pod::Pod;

use crate::buf::WordAligned;

// SAFETY: The header is both word-aligned and word-sized.
unsafe impl WordAligned for Header {}

#[repr(C, align(8))]
#[derive(Clone, Copy)]
pub struct Header {
    id: u32,
    size_with_opcode: u32,
    seq: u32,
    n_fds: u32,
}

impl Header {
    /// Construct a new header.
    #[inline]
    pub(crate) fn new(id: u32, op_code: u8, size: u32, seq: u32, n_fds: u32) -> Self {
        let size_with_opcode = ((op_code as u32) << 24) | (size & 0xffffff);

        Self {
            id,
            size_with_opcode,
            seq,
            n_fds,
        }
    }

    /// Get the id of the message.
    #[inline]
    pub fn id(&self) -> u32 {
        self.id
    }

    /// Get the opcode of the message.
    #[inline]
    pub fn op_code(&self) -> u32 {
        self.size_with_opcode >> 24
    }

    /// Get the size of the message.
    #[inline]
    pub fn size(&self) -> u32 {
        self.size_with_opcode & 0xffffff
    }

    /// Get the number of file descriptors.
    #[inline]
    pub fn n_fds(&self) -> u32 {
        self.n_fds
    }
}

impl fmt::Debug for Header {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Header")
            .field("id", &self.id)
            .field("size", &self.size())
            .field("op_code", &self.op_code())
            .field("seq", &self.seq)
            .field("n_fds", &self.n_fds)
            .finish()
    }
}

/// A read frame with a prepared pod for reading.
#[derive(Debug)]
#[non_exhaustive]
pub struct Frame<'recv> {
    /// The header of the frame.
    pub header: Header,
    /// The contents of the frame.
    pub pod: Pod<&'recv [u64]>,
}
