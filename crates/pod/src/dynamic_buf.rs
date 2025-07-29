use core::alloc::Layout;
use core::fmt;
use core::mem;
use core::ptr;
use core::slice;

use alloc::alloc;

use crate::utils::{Align, AlignableWith, UninitAlign};

pub(crate) const ALLOC: usize = 1024;

/// A buffer which can be used in combination with a channel.
pub struct DynamicBuf<T = u64> {
    data: ptr::NonNull<T>,
    cap: usize,
    read: usize,
    write: usize,
}

impl<T> DynamicBuf<T> {
    /// The size of a word in bytes.
    pub const WORD_SIZE: usize = mem::size_of::<T>();

    /// Construct a new empty buffer.
    #[inline]
    pub fn new() -> Self {
        DynamicBuf {
            data: ptr::NonNull::<T>::dangling().cast(),
            cap: 0,
            read: 0,
            write: 0,
        }
    }

    /// Get the remaining readable capacity of the buffer
    #[inline]
    pub fn remaining(&self) -> usize {
        self.write - self.read
    }

    /// Get the remaining mutable capacity of the buffer
    #[inline]
    pub fn remaining_mut(&self) -> usize {
        self.cap - self.write
    }

    /// Test if the buffer is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.read == self.write
    }

    /// Get the slice available for reading.
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
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
    pub unsafe fn as_bytes_mut(&mut self) -> &mut [u8] {
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
    pub fn extend_from_words(&mut self, words: &[T]) {
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
                .cast::<T>()
                .copy_from_nonoverlapping(words.as_ptr(), words.len());

            self.advance_written(n);
        }
    }

    /// Write `T` to the buffer.
    #[inline]
    pub fn write<U>(&mut self, value: U)
    where
        U: AlignableWith<T>,
    {
        let value = Align(value);

        debug_assert!(
            self.write % mem::size_of::<T>() == 0,
            "Write position in buffer is not aligned for T"
        );

        self.reserve(self.write + mem::size_of::<U>());

        // SAFETY: Necessary invariants have been checked above.
        unsafe {
            self.data
                .as_ptr()
                .cast::<u8>()
                .add(self.write)
                .cast::<T>()
                .copy_from_nonoverlapping(value.as_ptr::<T>(), value.size::<T>());

            self.advance_written(mem::size_of::<U>());
        }
    }

    /// Read `T` out of the buffer.
    #[inline]
    pub fn read<U>(&mut self) -> Option<U>
    where
        U: AlignableWith<T>,
    {
        if self.remaining() < mem::size_of::<T>() {
            return None;
        }

        let mut value = UninitAlign::<U>::uninit();

        // SAFETY: Necessary invariants have been checked above.
        unsafe {
            self.data
                .as_ptr()
                .cast::<u8>()
                .add(self.read)
                .cast::<T>()
                .copy_to_nonoverlapping(value.as_mut_ptr::<T>().cast::<T>(), value.size::<T>());

            self.advance_read(mem::size_of::<U>());
            Some(value.assume_init())
        }
    }

    /// Read a slice of words from the buffer.
    #[inline]
    pub fn read_words(&mut self, size: usize) -> Option<&[T]> {
        if size > self.remaining() {
            return None;
        }

        let n = size / Self::WORD_SIZE;

        // SAFETY: Necessary invariants have been checked above.
        unsafe {
            let value = slice::from_raw_parts(
                self.data.as_ptr().cast::<u8>().add(self.read).cast::<T>(),
                n,
            );
            self.advance_read(size);
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
    pub unsafe fn advance_read(&mut self, n: usize) {
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
    pub unsafe fn advance_written(&mut self, n: usize) {
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

impl<A> Drop for DynamicBuf<A> {
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

impl<T> fmt::Debug for DynamicBuf<T>
where
    T: fmt::Debug,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.as_bytes()).finish()
    }
}
