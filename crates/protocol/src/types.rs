//! Types which are used in the pipewire protocol.

use core::fmt;

use pod::utils::AlignableWith;

// SAFETY: The header is both word-aligned and word-sized.
unsafe impl AlignableWith for Header {}

#[repr(C, align(8))]
#[derive(Default, Clone, Copy)]
pub struct Header {
    id: u32,
    size_with_op: u32,
    seq: u32,
    n_fds: u32,
}

impl Header {
    /// Construct a new header.
    #[inline]
    pub(crate) fn new(id: u32, op: u8, size: u32, seq: u32, n_fds: u32) -> Option<Self> {
        if size > 0xffffff {
            return None;
        }

        let size_with_op = ((op as u32) << 24) | (size & 0xffffff);

        Some(Self {
            id,
            size_with_op,
            seq,
            n_fds,
        })
    }

    /// Get the id of the message.
    #[inline]
    pub fn id(&self) -> u32 {
        self.id
    }

    /// Get the opcode of the message.
    #[inline]
    pub fn op(&self) -> u8 {
        (self.size_with_op >> 24) as u8
    }

    /// Get the size of the message.
    #[inline]
    pub fn size(&self) -> u32 {
        self.size_with_op & 0xffffff
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
            .field("op", &self.op())
            .field("seq", &self.seq)
            .field("n_fds", &self.n_fds)
            .finish()
    }
}
