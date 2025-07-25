use core::alloc::Layout;
use core::cell::Cell;
use core::mem;
use core::ptr;
use core::slice;

use alloc::alloc;
use pod::Pod;

use crate::types::Frame;
use crate::types::Header;

pub(crate) const WORD_SIZE: usize = mem::size_of::<u64>();
pub(crate) const ALLOC: usize = 1024;

// Trait implemented for types which the buffer is guaranteed to be aligned for.
pub(crate) unsafe trait WordAligned: Copy {}

unsafe impl WordAligned for [u32; 0] {}
unsafe impl WordAligned for [u32; 2] {}
unsafe impl WordAligned for [u32; 4] {}
unsafe impl WordAligned for u64 {}
unsafe impl<const N: usize> WordAligned for [u64; N] {}

/// A buffer which can be used in combination with a channel.
pub struct Buf {
    data: ptr::NonNull<u8>,
    cap: usize,
    read: Cell<usize>,
    write: Cell<usize>,
}

impl Buf {
    /// Construct a new empty buffer.
    #[inline]
    pub fn new() -> Self {
        Buf {
            data: ptr::NonNull::<u64>::dangling().cast(),
            cap: 0,
            read: Cell::new(0),
            write: Cell::new(0),
        }
    }

    /// Get the remaining readable capacity of the buffer
    #[inline]
    pub(crate) fn remaining(&self) -> usize {
        self.write.get() - self.read.get()
    }

    /// Get the remaining mutable capacity of the buffer
    #[inline]
    pub(crate) fn remaining_mut(&self) -> usize {
        self.cap - self.write.get()
    }

    /// Test if the buffer is empty.
    #[inline]
    pub(crate) fn is_empty(&self) -> bool {
        self.read.get() == self.write.get()
    }

    /// Get the slice available for reading.
    #[inline]
    pub(crate) fn as_bytes(&self) -> &[u8] {
        // SAFETY: The buffer is guaranteed to initialized due to invariants.
        unsafe { slice::from_raw_parts(self.data.as_ptr().add(self.read.get()), self.remaining()) }
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
        self.reserve(self.write.get() + ALLOC);

        unsafe {
            slice::from_raw_parts_mut(
                self.data.as_ptr().add(self.write.get()),
                self.remaining_mut(),
            )
        }
    }

    /// Extend the buffer with a slice of words.
    #[inline]
    pub(crate) fn extend_from_words(&mut self, words: &[u64]) {
        debug_assert!(
            self.write.get() % WORD_SIZE == 0,
            "Write position in buffer is not aligned for T"
        );

        let n = words.len() * WORD_SIZE;
        self.reserve(self.write.get() + n);

        // SAFETY: Necessary invariants have been checked above.
        unsafe {
            self.data
                .as_ptr()
                .add(self.write.get())
                .cast::<u64>()
                .copy_from_nonoverlapping(words.as_ptr(), words.len());

            self.set_written(n);
        }
    }

    /// Write `T` to the buffer.
    #[inline]
    pub(crate) fn write<T>(&mut self, value: T)
    where
        T: WordAligned,
    {
        debug_assert!(
            self.write.get() % mem::size_of::<T>() == 0,
            "Write position in buffer is not aligned for T"
        );

        self.reserve(self.write.get() + mem::size_of::<T>());

        // SAFETY: Necessary invariants have been checked above.
        unsafe {
            self.data
                .as_ptr()
                .add(self.write.get())
                .cast::<T>()
                .write(value);
            self.set_written(mem::size_of::<T>());
        }
    }

    /// Read `T` out of the buffer.
    #[inline]
    pub(crate) fn read<T>(&self) -> Option<T>
    where
        T: WordAligned,
    {
        if self.remaining() < mem::size_of::<T>() {
            return None;
        }

        // SAFETY: Necessary invariants have been checked above.
        unsafe {
            let value = self.data.as_ptr().add(self.read.get()).cast::<T>().read();
            self.set_read(mem::size_of::<T>());
            Some(value)
        }
    }

    /// Read `T` out of the buffer.
    #[inline]
    pub(crate) fn peek<T>(&self) -> Option<&T>
    where
        T: WordAligned,
    {
        if self.remaining() < mem::size_of::<T>() {
            return None;
        }

        // SAFETY: Necessary invariants have been checked above.
        Some(unsafe { &*self.data.as_ptr().add(self.read.get()).cast::<T>() })
    }

    /// Read a slice of words from the buffer.
    #[inline]
    pub(crate) fn read_words(&self, size: usize) -> Option<&[u64]> {
        if size > self.remaining() {
            return None;
        }

        let n = size / WORD_SIZE;

        // SAFETY: Necessary invariants have been checked above.
        unsafe {
            let value =
                slice::from_raw_parts(self.data.as_ptr().add(self.read.get()).cast::<u64>(), n);
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
    pub(crate) unsafe fn set_read(&self, n: usize) {
        self.read.set(self.read.get() + n);

        if self.read.get() == self.write.get() {
            self.read.set(0);
            self.write.set(0);
        }

        debug_assert!(
            self.read.get() <= self.write.get(),
            "Read position {} in buffer is greater than write {}",
            self.read.get(),
            self.write.get()
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
    pub(crate) unsafe fn set_written(&self, n: usize) {
        self.write.set(self.write.get() + n);

        debug_assert!(
            self.write.get() <= self.cap,
            "Write position {} in buffer is greater than capacity {}",
            self.write.get(),
            self.cap
        );
    }

    /// Ensure up to the given length is reserved.
    #[inline]
    pub(crate) fn reserve(&mut self, needed: usize) {
        if self.cap >= needed {
            return;
        }

        let new_cap = needed.next_power_of_two().max(WORD_SIZE);

        let ptr = if self.cap == 0 {
            unsafe {
                let layout = Layout::from_size_align_unchecked(new_cap, WORD_SIZE);
                let ptr = alloc::alloc(layout);

                if ptr.is_null() {
                    alloc::handle_alloc_error(layout);
                }

                ptr::NonNull::new_unchecked(ptr)
            }
        } else {
            unsafe {
                let layout = Layout::from_size_align_unchecked(self.cap, WORD_SIZE);
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

        self.data = ptr;
        self.cap = new_cap;
    }

    /// Read a frame from the current buffer.
    pub fn frame(&self, header: Header) -> Option<Frame<'_>> {
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

impl Drop for Buf {
    #[inline]
    fn drop(&mut self) {
        if self.cap > 0 {
            // SAFETY: The buffer is guaranteed to be allocated with the same alignment as `u64`.
            unsafe {
                let layout = Layout::from_size_align_unchecked(self.cap, WORD_SIZE);
                alloc::dealloc(self.data.as_ptr(), layout);
            }

            self.data = ptr::NonNull::<u64>::dangling().cast();
            self.cap = 0;
            self.read.set(0);
            self.write.set(0);
        }
    }
}
