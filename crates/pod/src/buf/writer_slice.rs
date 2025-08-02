use core::marker::PhantomData;
use core::mem;

use crate::writer::Pos;
use crate::{AsSlice, Slice, Writer};

/// A stored position in a writer that has not yet been realized.
pub struct WriterSlice<W, T>
where
    W: Writer,
{
    writer: W,
    pos: W::Pos,
    _marker: PhantomData<T>,
}

impl<W, T> WriterSlice<W, T>
where
    W: Writer,
{
    /// Construct a new slice position.
    pub(crate) fn new(writer: W, pos: W::Pos) -> Self {
        Self {
            writer,
            pos,
            _marker: PhantomData,
        }
    }
}

impl<B, T> AsSlice for WriterSlice<B, T>
where
    B: Writer,
{
    #[inline]
    fn as_slice(&self) -> Slice<'_> {
        self.writer
            .slice_from(self.pos.saturating_add(mem::size_of::<T>()))
    }
}
