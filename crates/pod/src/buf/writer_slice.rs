use crate::writer::Pos;
use crate::{AsSlice, Slice, Writer};

/// A stored position in a writer that has not yet been realized.
pub struct WriterSlice<W, const N: usize>
where
    W: Writer,
{
    writer: W,
    pos: W::Pos,
}

impl<W, const N: usize> WriterSlice<W, N>
where
    W: Writer,
{
    /// Construct a new slice position.
    pub(crate) fn new(writer: W, pos: W::Pos) -> Self {
        Self { writer, pos }
    }
}

impl<B, const N: usize> AsSlice for WriterSlice<B, N>
where
    B: Writer,
{
    #[inline]
    fn as_slice(&self) -> Slice<'_> {
        self.writer.slice_from(self.pos.saturating_add(N))
    }
}
