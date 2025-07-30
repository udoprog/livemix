use core::alloc::Layout;
use core::fmt;
use core::mem;
use core::ptr;
use core::slice;

use alloc::alloc;

use pod::utils::BytesInhabited;
use pod::utils::{AlignableWith, UninitAlign};

use super::AllocError;

/// A buffer which can be used in combination with a channel.
pub struct SendBuf {
    data: ptr::NonNull<u8>,
    cap: usize,
    read: usize,
    write: usize,
}

impl SendBuf {
    /// Construct a new empty buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use protocol::buf::SendBuf;
    ///
    /// let expected = u64::to_ne_bytes(0x7f7f7f77f7f7f7ffu64);
    ///
    /// let mut buf = SendBuf::new();
    /// assert!(buf.is_empty());
    /// buf.push_bytes(&0x7f7f7f77f7f7f7ffu64)?;
    /// assert_eq!(buf.len(), 8);
    /// assert_eq!(buf.as_bytes(), expected);
    /// # Ok::<_, protocol::buf::AllocError>(())
    /// ```
    #[inline]
    pub const fn new() -> Self {
        SendBuf {
            data: ptr::NonNull::<u64>::dangling().cast(),
            cap: 0,
            read: 0,
            write: 0,
        }
    }

    /// Get the remaining readable capacity of the buffer
    ///
    /// # Examples
    ///
    /// ```
    /// use protocol::buf::SendBuf;
    ///
    /// let mut buf = SendBuf::new();
    /// assert!(buf.is_empty());
    /// buf.push_bytes(&42u64)?;
    /// assert_eq!(buf.len(), 8);
    ///
    /// let expected = 42u64.to_ne_bytes();
    /// assert_eq!(buf.as_bytes(), &expected[..]);
    /// # Ok::<_, protocol::buf::AllocError>(())
    /// ```
    #[inline]
    pub fn len(&self) -> usize {
        self.write - self.read
    }

    /// Test if the buffer is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use protocol::buf::SendBuf;
    ///
    /// let mut buf = SendBuf::new();
    /// assert!(buf.is_empty());
    /// buf.push_bytes(&42u64)?;
    /// assert_eq!(buf.len(), 8);
    ///
    /// let expected = 42u64.to_ne_bytes();
    /// assert_eq!(buf.as_bytes(), &expected[..]);
    /// # Ok::<_, protocol::buf::AllocError>(())
    /// ```
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.read == self.write
    }

    /// Get the remaining readable capacity of the buffer
    ///
    /// # Examples
    ///
    /// ```
    /// use protocol::buf::SendBuf;
    ///
    /// let mut buf = SendBuf::new();
    /// assert_eq!(buf.len(), 0);
    /// buf.push_bytes(&42u64)?;
    /// assert_eq!(buf.len(), 8);
    ///
    /// let expected = 42u64.to_ne_bytes();
    /// assert_eq!(buf.remaining_bytes(), 8);
    /// assert_eq!(buf.as_bytes(), expected);
    /// # Ok::<_, protocol::buf::AllocError>(())
    /// ```
    #[inline]
    pub fn remaining_bytes(&self) -> usize {
        self.write - self.read
    }

    /// Clear the contents of the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use protocol::buf::SendBuf;
    ///
    /// let mut buf = SendBuf::new();
    /// assert!(buf.is_empty());
    /// buf.extend_from_words(&[1u64, 2])?;
    /// assert_eq!(buf.len(), 16);
    /// buf.clear();
    /// assert!(buf.is_empty());
    /// # Ok::<_, protocol::buf::AllocError>(())
    /// ```
    #[inline]
    pub fn clear(&mut self) {
        self.read = 0;
        self.write = 0;
    }

    /// Returns the bytes of the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use protocol::buf::SendBuf;
    ///
    /// let mut buf = SendBuf::new();
    /// assert_eq!(buf.len(), 0);
    /// buf.push_bytes(&42u64)?;
    /// assert_eq!(buf.len(), 8);
    ///
    /// let expected = 42u64.to_ne_bytes();
    /// assert_eq!(buf.remaining_bytes(), 8);
    /// assert_eq!(buf.as_bytes(), expected);
    /// # Ok::<_, protocol::buf::AllocError>(())
    /// ```
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        // SAFETY: The buffer is guaranteed to initialized due to invariants.
        unsafe { slice::from_raw_parts(self.as_bytes_ptr(), self.remaining_bytes()) }
    }

    /// Extend the buffer with a slice of words.
    ///
    /// # Examples
    ///
    /// ```
    /// use protocol::buf::SendBuf;
    ///
    /// let mut buf = SendBuf::new();
    /// assert!(buf.is_empty());
    /// buf.extend_from_words(&[1u64, 2, 3, 4]);
    /// assert_eq!(buf.len(), 32);
    /// # Ok::<_, protocol::buf::AllocError>(())
    /// ```
    #[inline]
    pub fn extend_from_words<T>(&mut self, words: &[T]) -> Result<(), AllocError>
    where
        T: BytesInhabited,
    {
        let len = words.len().wrapping_mul(mem::size_of::<T>());
        self.reserve(self.write + len)?;

        // SAFETY: Necessary invariants have been checked above.
        unsafe {
            self.data
                .as_ptr()
                .add(self.write)
                .copy_from_nonoverlapping(words.as_ptr().cast(), len);

            self.advance_written(len);
        }

        Ok(())
    }

    /// Push the bytes of `U` into the buffer.
    ///
    /// This guarantees that `U` is bytes-compatible with `T` and that alignment
    /// in the buffer is maintained.
    ///
    /// # Examples
    ///
    /// ```
    /// use protocol::buf::SendBuf;
    ///
    /// let mut buf = SendBuf::new();
    /// assert_eq!(buf.len(), 0);
    /// buf.push_bytes(&42u64)?;
    /// assert_eq!(buf.len(), 8);
    ///
    /// let expected = 42u64.to_ne_bytes();
    /// assert_eq!(buf.as_bytes(), expected);
    /// # Ok::<_, protocol::buf::AllocError>(())
    /// ```
    #[inline]
    pub fn push_bytes<T>(&mut self, value: &T) -> Result<(), AllocError>
    where
        T: BytesInhabited,
    {
        self.extend_from_words(slice::from_ref(value))
    }

    /// Read `T` out of the buffer.
    #[inline]
    pub fn read<T>(&mut self) -> Option<T>
    where
        T: AlignableWith,
    {
        if self.is_empty() {
            return None;
        }

        let mut value = UninitAlign::<T>::uninit();

        // SAFETY: Necessary invariants have been checked above.
        unsafe {
            self.data
                .as_ptr()
                .add(self.read)
                .copy_to_nonoverlapping(value.as_mut_ptr().cast(), value.size());

            self.advance_read(value.size());
            Some(value.assume_init())
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
    ///
    /// # Examples
    ///
    /// ```
    /// use protocol::buf::SendBuf;
    ///
    /// let expected = 0x7f7f7f7fu64;
    /// let bytes = u64::to_ne_bytes(expected);
    ///
    /// let mut buf = SendBuf::new();
    /// buf.push_bytes(&expected)?;
    ///
    /// assert_eq!(buf.as_bytes(), &bytes[..]);
    ///
    /// unsafe {
    ///     buf.advance_read_bytes(1);
    /// }
    ///
    /// assert_eq!(buf.as_bytes(), &bytes[1..]);
    /// # Ok::<_, protocol::buf::AllocError>(())
    /// ```
    #[inline]
    pub unsafe fn advance_read_bytes(&mut self, n: usize) {
        let read = self.read + n;

        debug_assert!(
            read <= self.write,
            "Read position {read} in buffer is greater than write {}",
            self.write
        );

        self.read = read;

        if self.read >= self.write {
            self.read = 0;
            self.write = 0;
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
    unsafe fn advance_read(&mut self, n: usize) {
        let read = self.read + n;

        debug_assert!(
            read <= self.write,
            "Read position {read} in buffer is greater than write {}",
            self.write
        );

        self.read = read;

        if self.read == self.write {
            self.read = 0;
            self.write = 0;
        }
    }

    /// Add that a given amount of bytes has been written.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the specified number of words
    /// `self.write..self.write + n` is a valid memory region in the buffer that
    /// has previously been written to.
    #[inline]
    unsafe fn advance_written(&mut self, n: usize) {
        let write = self.write + n;

        // Debug assertion, since this is an internal API.
        debug_assert!(
            write <= self.cap,
            "Write position {write} in buffer is greater than capacity {}",
            self.cap
        );

        self.write = write;
    }

    #[inline]
    fn as_bytes_ptr(&self) -> *const u8 {
        self.data.as_ptr().wrapping_add(self.read).cast_const()
    }

    /// Ensure up to the given length is reserved.
    fn reserve(&mut self, needed: usize) -> Result<(), AllocError> {
        if needed <= self.cap {
            return Ok(());
        }

        let cap = needed.next_power_of_two().max(16);

        let data = match self.cap {
            0 => unsafe {
                let layout =
                    Layout::from_size_align(cap, mem::align_of::<u64>()).map_err(|_| AllocError)?;

                let data = alloc::alloc_zeroed(layout);

                if data.is_null() {
                    return Err(AllocError);
                }

                ptr::NonNull::new_unchecked(data)
            },
            _ => unsafe {
                let old_layout =
                    Layout::from_size_align_unchecked(self.cap, mem::align_of::<u64>());
                let new_layout =
                    Layout::from_size_align(cap, mem::align_of::<u64>()).map_err(|_| AllocError)?;

                let data = alloc::realloc(self.data.as_ptr().cast(), old_layout, new_layout.size());

                if data.is_null() {
                    return Err(AllocError);
                }

                // Zero-initialize the region so it can be returned by
                // `as_bytes_mut`.
                data.wrapping_add(self.cap).write_bytes(0, cap - self.cap);

                ptr::NonNull::new_unchecked(data)
            },
        };

        self.data = data;
        self.cap = cap;
        Ok(())
    }

    fn free(&mut self) {
        if self.cap > 0 {
            // SAFETY: The buffer is guaranteed to be allocated with the same alignment as `A`.
            unsafe {
                let layout = Layout::from_size_align_unchecked(self.cap, mem::align_of::<u64>());
                alloc::dealloc(self.data.as_ptr().cast(), layout);
            }

            self.data = ptr::NonNull::dangling();
            self.cap = 0;
            self.read = 0;
            self.write = 0;
        }
    }
}

impl Drop for SendBuf {
    #[inline]
    fn drop(&mut self) {
        self.clear();
        self.free();
    }
}

impl fmt::Debug for SendBuf {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SendBuf").field("len", &self.len()).finish()
    }
}
