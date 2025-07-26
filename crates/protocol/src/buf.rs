use core::alloc::Layout;
use core::mem;
use core::ptr;
use core::slice;

use alloc::alloc;
use pod::Pod;
use pod::utils::{Align, AlignableWith, UninitAlign};

use crate::types::Frame;
use crate::types::Header;

pub(crate) const ALLOC: usize = 1024;

/// A buffer which can be used in combination with a channel.
pub struct Buf<A = u64>
where
    A: Copy,
{
    data: ptr::NonNull<A>,
    cap: usize,
    read: usize,
    write: usize,
}

impl<A> Buf<A>
where
    A: Copy,
{
    pub(crate) const WORD_SIZE: usize = mem::size_of::<A>();

    /// Construct a new empty buffer.
    #[inline]
    pub fn new() -> Self {
        Buf {
            data: ptr::NonNull::<A>::dangling().cast(),
            cap: 0,
            read: 0,
            write: 0,
        }
    }

    /// Get the remaining readable capacity of the buffer
    #[inline]
    pub(crate) fn remaining(&self) -> usize {
        self.write - self.read
    }

    /// Get the remaining mutable capacity of the buffer
    #[inline]
    pub(crate) fn remaining_mut(&self) -> usize {
        self.cap - self.write
    }

    /// Test if the buffer is empty.
    #[inline]
    pub(crate) fn is_empty(&self) -> bool {
        self.read == self.write
    }

    /// Get the slice available for reading.
    #[inline]
    pub(crate) fn as_bytes(&self) -> &[u8] {
        // SAFETY: The buffer is guaranteed to initialized due to invariants.
        unsafe {
            slice::from_raw_parts(
                self.data.as_ptr().cast::<u8>().add(self.read),
                self.remaining(),
            )
        }
    }

    /// Get a slice available for writing.
    ///
    /// This ensures that at least `ALLOC` bytes are available for writing and
    /// will reallocate if necessary to ensure that.
    ///
    /// # Safety
    ///
    /// Since this might cause the buffer to be unaligned, it is the caller's
    /// responsibility to ensure that writes (if any) in the future are aligned.
    #[inline]
    pub(crate) unsafe fn as_bytes_mut(&mut self) -> &mut [u8] {
        self.reserve(self.write + ALLOC);

        unsafe {
            slice::from_raw_parts_mut(
                self.data.as_ptr().cast::<u8>().add(self.write),
                self.remaining_mut(),
            )
        }
    }

    /// Extend the buffer with a slice of words.
    #[inline]
    pub(crate) fn extend_from_words(&mut self, words: &[A]) {
        debug_assert!(
            self.write % Self::WORD_SIZE == 0,
            "Write position in buffer is not aligned for T"
        );

        let n = words.len() * Self::WORD_SIZE;
        self.reserve(self.write + n);

        // SAFETY: Necessary invariants have been checked above.
        unsafe {
            self.data
                .as_ptr()
                .cast::<u8>()
                .add(self.write)
                .cast::<A>()
                .copy_from_nonoverlapping(words.as_ptr(), words.len());

            self.set_written(n);
        }
    }

    /// Write `T` to the buffer.
    #[inline]
    pub(crate) fn write<T>(&mut self, value: T)
    where
        T: AlignableWith<A>,
    {
        let value = Align(value);

        debug_assert!(
            self.write % mem::size_of::<T>() == 0,
            "Write position in buffer is not aligned for T"
        );

        self.reserve(self.write + mem::size_of::<T>());

        // SAFETY: Necessary invariants have been checked above.
        unsafe {
            self.data
                .as_ptr()
                .cast::<u8>()
                .add(self.write)
                .cast::<A>()
                .copy_from_nonoverlapping(value.as_ptr::<A>(), value.size::<A>());

            self.set_written(mem::size_of::<T>());
        }
    }

    /// Read `T` out of the buffer.
    #[inline]
    pub(crate) fn read<T>(&mut self) -> Option<T>
    where
        T: AlignableWith<A>,
    {
        if self.remaining() < mem::size_of::<T>() {
            return None;
        }

        let mut value = UninitAlign::<T>::uninit();

        // SAFETY: Necessary invariants have been checked above.
        unsafe {
            self.data
                .as_ptr()
                .cast::<u8>()
                .add(self.read)
                .cast::<A>()
                .copy_to_nonoverlapping(value.as_mut_ptr::<A>().cast::<A>(), value.size::<A>());

            self.set_read(mem::size_of::<T>());
            Some(value.assume_init())
        }
    }

    /// Read a slice of words from the buffer.
    #[inline]
    pub(crate) fn read_words(&mut self, size: usize) -> Option<&[A]> {
        if size > self.remaining() {
            return None;
        }

        let n = size / Self::WORD_SIZE;

        // SAFETY: Necessary invariants have been checked above.
        unsafe {
            let value = slice::from_raw_parts(
                self.data.as_ptr().cast::<u8>().add(self.read).cast::<A>(),
                n,
            );
            self.set_read(size);
            Some(value)
        }
    }

    /// Add that a given amount of bytes has been read.
    ///
    /// Note that this is safe since we always ensure that the buffer is
    /// zero-initialized.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the specified number of bytes
    /// `self.read..self.read + n` is a valid memory region in the buffer that
    /// has previously been written to.
    #[inline]
    pub(crate) unsafe fn set_read(&mut self, n: usize) {
        self.read = self.read + n;

        if self.read == self.write {
            self.read = 0;
            self.write = 0;
        }

        debug_assert!(
            self.read <= self.write,
            "Read position {} in buffer is greater than write {}",
            self.read,
            self.write
        );
    }

    /// Add that a given amount of bytes has been written.
    ///
    /// Note that this is safe since we always ensure that the buffer is
    /// zero-initialized.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the specified number of bytes
    /// `self.write..self.write + n` is a valid memory region in the buffer that
    /// has previously been written to.
    #[inline]
    pub(crate) unsafe fn set_written(&mut self, n: usize) {
        self.write = self.write + n;

        debug_assert!(
            self.write <= self.cap,
            "Write position {} in buffer is greater than capacity {}",
            self.write,
            self.cap
        );
    }

    /// Ensure up to the given length is reserved.
    #[inline]
    pub(crate) fn reserve(&mut self, needed: usize) {
        if self.cap >= needed {
            return;
        }

        let new_cap = needed.next_power_of_two().max(Self::WORD_SIZE);

        let ptr = if self.cap == 0 {
            unsafe {
                let layout = Layout::from_size_align_unchecked(new_cap, Self::WORD_SIZE);
                let ptr = alloc::alloc(layout);

                if ptr.is_null() {
                    alloc::handle_alloc_error(layout);
                }

                ptr::NonNull::new_unchecked(ptr)
            }
        } else {
            unsafe {
                let layout = Layout::from_size_align_unchecked(self.cap, Self::WORD_SIZE);
                let ptr = alloc::realloc(self.data.as_ptr().cast(), layout, new_cap);

                if ptr.is_null() {
                    alloc::handle_alloc_error(layout);
                }

                ptr::NonNull::new_unchecked(ptr)
            }
        };

        // SAFETY: Zero the buffer allowing it to be conveniently used.
        unsafe {
            ptr.as_ptr()
                .add(self.cap)
                .write_bytes(0, new_cap - self.cap);
        }

        self.data = ptr.cast();
        self.cap = new_cap;
    }
}

impl Buf {
    /// Read a frame from the current buffer.
    pub fn frame(&mut self, header: Header) -> Option<Frame<'_>> {
        let size = header.size() as usize;

        if size > self.remaining() {
            return None;
        }

        let words = self.read_words(size)?;

        Some(Frame {
            header,
            pod: Pod::new(words),
        })
    }
}

impl<A> Drop for Buf<A>
where
    A: Copy,
{
    #[inline]
    fn drop(&mut self) {
        if self.cap > 0 {
            // SAFETY: The buffer is guaranteed to be allocated with the same alignment as `A`.
            unsafe {
                let layout = Layout::from_size_align_unchecked(self.cap, Self::WORD_SIZE);
                alloc::dealloc(self.data.as_ptr().cast(), layout);
            }

            self.data = ptr::NonNull::<A>::dangling().cast();
            self.cap = 0;
            self.read = 0;
            self.write = 0;
        }
    }
}
