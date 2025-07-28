//! Trait used for collecting events.
//!
//! Two notable implementations of this trait are
//! [StackVec][crate::stack::StackVec] and [Vec].

use alloc::vec::Vec;
use pod::Buf;

/// Trait used for collecting events.
pub trait Events<T> {
    /// Test if the collection is empty.
    fn is_empty(&self) -> bool;

    /// Report the remaining capacity.
    ///
    /// Implementer must guarantee that [push][Events::push] never returns
    /// `false` if this is non-zero.
    fn remaining_mut(&self) -> usize;

    /// Push an event returning a `bool` that if `true` indicates that the event was successfully stored.
    fn push(&mut self, event: T) -> bool;
}

impl<T, const N: usize> Events<T> for Buf<T, N> {
    #[inline]
    fn is_empty(&self) -> bool {
        Buf::is_empty(self)
    }

    #[inline]
    fn remaining_mut(&self) -> usize {
        N.wrapping_sub(Buf::len(self))
    }

    #[inline]
    fn push(&mut self, event: T) -> bool {
        Buf::push(self, event).is_ok()
    }
}

impl<T> Events<T> for Vec<T> {
    #[inline]
    fn is_empty(&self) -> bool {
        self.is_empty()
    }

    #[inline]
    fn remaining_mut(&self) -> usize {
        usize::MAX.saturating_sub(self.len())
    }

    #[inline]
    fn push(&mut self, event: T) -> bool {
        self.push(event);
        true
    }
}
