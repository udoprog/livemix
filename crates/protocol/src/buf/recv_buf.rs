use core::alloc::Layout;
use core::fmt;
use core::mem;
use core::mem::MaybeUninit;
use core::ptr;
use core::slice;

use alloc::alloc;

use pod::utils::BytesInhabited;

use super::AllocError;

pub(crate) const WANTS_BYTES: usize = 1 << 14;

/// A buffer which can be used in combination with a channel.
pub struct RecvBuf {
    data: ptr::NonNull<u8>,
    cap: usize,
    read: usize,
    write: usize,
}

impl RecvBuf {
    /// Construct a new empty buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use protocol::buf::RecvBuf;
    ///
    /// let expected = u64::to_ne_bytes(0x7f7f7f7f);
    ///
    /// let mut buf = RecvBuf::new();
    /// assert!(buf.is_empty());
    /// buf.as_bytes_mut()?[..8].copy_from_slice(&expected[..]);
    ///
    /// unsafe {
    ///     buf.advance_written_bytes(8);
    /// }
    ///
    /// assert_eq!(buf.len(), 8);
    /// assert_eq!(buf.remaining_bytes(), 8);
    /// # Ok::<_, protocol::buf::AllocError>(())
    /// ```
    #[inline]
    pub const fn new() -> Self {
        RecvBuf {
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
    /// use protocol::buf::RecvBuf;
    ///
    /// let expected = u64::to_ne_bytes(0x7f7f7f7f);
    ///
    /// let mut buf = RecvBuf::new();
    /// assert!(buf.is_empty());
    /// buf.as_bytes_mut()?[..8].copy_from_slice(&expected[..]);
    ///
    /// unsafe {
    ///     buf.advance_written_bytes(8);
    /// }
    ///
    /// assert_eq!(buf.len(), 8);
    /// assert_eq!(buf.remaining_bytes(), 8);
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
    /// use protocol::buf::RecvBuf;
    ///
    /// let expected = u64::to_ne_bytes(0x7f7f7f7f);
    ///
    /// let mut buf = RecvBuf::new();
    /// assert!(buf.is_empty());
    /// buf.as_bytes_mut()?[..8].copy_from_slice(&expected[..]);
    ///
    /// unsafe {
    ///     buf.advance_written_bytes(8);
    /// }
    ///
    /// assert!(!buf.is_empty());
    /// assert_eq!(buf.len(), 8);
    /// assert_eq!(buf.remaining_bytes(), 8);
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
    /// use protocol::buf::RecvBuf;
    ///
    /// let expected = u64::to_ne_bytes(0x7f7f7f7f);
    ///
    /// let mut buf = RecvBuf::new();
    /// assert!(buf.is_empty());
    /// buf.as_bytes_mut()?[..8].copy_from_slice(&expected[..]);
    ///
    /// unsafe {
    ///     buf.advance_written_bytes(8);
    /// }
    ///
    /// assert_eq!(buf.len(), 8);
    /// assert_eq!(buf.remaining_bytes(), 8);
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
    /// use protocol::buf::RecvBuf;
    ///
    /// let expected = u64::to_ne_bytes(0x7f7f7f7f);
    ///
    /// let mut buf = RecvBuf::new();
    /// assert!(buf.is_empty());
    /// buf.as_bytes_mut()?[..8].copy_from_slice(&expected[..]);
    ///
    /// unsafe {
    ///     buf.advance_written_bytes(8);
    /// }
    ///
    /// assert_eq!(buf.len(), 8);
    /// assert_eq!(buf.remaining_bytes(), 8);
    ///
    /// buf.clear();
    ///
    /// assert!(buf.is_empty());
    /// assert_eq!(buf.len(), 0);
    /// assert_eq!(buf.remaining_bytes(), 0);
    /// # Ok::<_, protocol::buf::AllocError>(())
    /// ```
    #[inline]
    pub fn clear(&mut self) {
        self.read = 0;
        self.write = 0;
    }

    /// Returns the slice of data in the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use protocol::buf::RecvBuf;
    ///
    /// let expected = u64::to_ne_bytes(0x123456789abcdef0);
    ///
    /// let mut buf = RecvBuf::new();
    /// assert!(buf.is_empty());
    /// buf.as_bytes_mut()?[..8].copy_from_slice(&expected[..]);
    ///
    /// unsafe {
    ///     buf.advance_written_bytes(8);
    /// }
    ///
    /// assert_eq!(buf.len(), 8);
    /// assert_eq!(buf.as_slice(), &[0x123456789abcdef0]);
    /// # Ok::<_, protocol::buf::AllocError>(())
    /// ```
    #[inline]
    pub fn as_slice(&self) -> &[u64] {
        let read = self.read.next_multiple_of(mem::size_of::<u64>());

        if read >= self.write {
            return &[];
        }

        let ptr = self
            .data
            .as_ptr()
            .wrapping_add(read)
            .cast_const()
            .cast::<u64>();
        let len = (self.write - read).wrapping_div(mem::size_of::<u64>());

        // SAFETY: The buffer is guaranteed to be initialized up to `pos`.
        unsafe { slice::from_raw_parts(ptr, len) }
    }

    /// Get an initialized slice of bytes available for writing.
    ///
    /// This is useful since it allows writing native aligned values from a byte
    /// array from APIs like [`Read`].
    ///
    /// The number of bytes written should must be communicated through
    /// [`RecvBuf::advance_written_bytes`].
    ///
    /// [`Read`]: std::io::Read
    ///
    /// # Examples
    ///
    /// ```
    /// use protocol::buf::RecvBuf;
    ///
    /// let expected = u64::from_ne_bytes([1, 2, 3, 4, 5, 6, 7, 8]);
    ///
    /// let mut buf = RecvBuf::new();
    /// assert!(buf.as_bytes_mut()?.len() > 0);
    ///
    /// buf.as_bytes_mut()?[..3].copy_from_slice(&[1, 2, 3]);
    ///
    /// unsafe {
    ///     buf.advance_written_bytes(3);
    /// }
    ///
    /// assert_eq!(buf.as_slice(), &[]);
    ///
    /// buf.as_bytes_mut()?[..5].copy_from_slice(&[4, 5, 6, 7, 8]);
    ///
    /// unsafe {
    ///     buf.advance_written_bytes(5);
    /// }
    ///
    /// assert_eq!(buf.as_slice(), &[expected]);
    /// # Ok::<_, protocol::buf::AllocError>(())
    /// ```
    #[inline]
    pub fn as_bytes_mut(&mut self) -> Result<&mut [u8], AllocError> {
        self.reserve(self.write + WANTS_BYTES)?;

        Ok(unsafe {
            slice::from_raw_parts_mut(self.as_bytes_ptr_mut(), self.remaining_bytes_mut())
        })
    }

    /// Read `T` out of the buffer.
    #[inline]
    pub fn read<U>(&mut self) -> Option<U>
    where
        U: BytesInhabited,
    {
        if self.len() < mem::size_of::<U>() {
            return None;
        }

        let mut value = MaybeUninit::<U>::uninit();

        // SAFETY: Necessary invariants have been checked above.
        unsafe {
            self.data
                .as_ptr()
                .add(self.read)
                .copy_to_nonoverlapping(value.as_mut_ptr().cast(), mem::size_of::<U>());

            self.advance_read(mem::size_of::<U>());
            Some(value.assume_init())
        }
    }

    /// Read a slice of words from the buffer.
    ///
    /// This requires that `T` implements `BytesInhabited`.
    ///
    /// # Examples
    ///
    /// ```
    /// use protocol::buf::RecvBuf;
    ///
    /// let mut buf = RecvBuf::new();
    /// assert!(buf.as_bytes_mut()?.len() > 0);
    ///
    /// buf.as_bytes_mut()?[..7].copy_from_slice(&[1, 2, 3, 4, 5, 6, 7]);
    ///
    /// unsafe {
    ///     buf.advance_written_bytes(7);
    /// }
    ///
    /// assert!(buf.read_words(8).is_none());
    ///
    /// buf.as_bytes_mut()?[..1].copy_from_slice(&[8]);
    ///
    /// unsafe {
    ///     buf.advance_written_bytes(1);
    /// }
    ///
    /// let expected = u64::from_ne_bytes([1, 2, 3, 4, 5, 6, 7, 8]);
    ///
    /// assert_eq!(buf.as_slice(), &[expected]);
    ///
    /// assert!(buf.read_words(16).is_none());
    /// assert_eq!(buf.read_words(8), Some(&[expected][..]));
    /// # Ok::<_, protocol::buf::AllocError>(())
    /// ```
    #[inline]
    pub fn read_words(&mut self, size: usize) -> Option<&[u64]> {
        if self.read % mem::align_of::<u64>() != 0 || size > self.len() {
            return None;
        }

        // SAFETY: Necessary invariants have been checked above.
        unsafe {
            let ptr = self.data.as_ptr().add(self.read).cast();
            let len = size.wrapping_div(mem::size_of::<u64>());
            let value = slice::from_raw_parts(ptr, len);
            self.advance_read(size);
            Some(value)
        }
    }

    /// Add that a given amount of bytes has been written.
    ///
    /// Note that this is safe since we always ensure that the buffer is
    /// zero-initialized.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the specified number of bytes fits with the
    /// buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use protocol::buf::RecvBuf;
    ///
    /// let mut buf = RecvBuf::new();
    /// assert!(buf.as_bytes_mut()?.len() > 0);
    ///
    /// buf.as_bytes_mut()?[..7].copy_from_slice(&[1, 2, 3, 4, 5, 6, 7]);
    ///
    /// unsafe {
    ///     buf.advance_written_bytes(7);
    /// }
    ///
    /// assert_eq!(buf.as_slice(), &[]);
    /// assert!(buf.read_words(8).is_none());
    ///
    /// let expected = u64::from_ne_bytes([1, 2, 3, 4, 5, 6, 7, 8]);
    ///
    /// buf.as_bytes_mut()?[..1].copy_from_slice(&[8]);
    ///
    /// unsafe {
    ///     buf.advance_written_bytes(1);
    /// }
    ///
    /// assert_eq!(buf.as_slice(), &[expected]);
    /// assert_eq!(buf.read_words(8), Some(&[expected][..]));
    /// # Ok::<_, protocol::buf::AllocError>(())
    /// ```
    #[inline]
    pub unsafe fn advance_written_bytes(&mut self, n: usize) {
        let write = self.write + n;

        assert!(
            write <= self.cap,
            "Write position {} in buffer is greater than capacity {}",
            self.write,
            self.cap
        );

        self.write = write;
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

    #[inline]
    fn as_bytes_ptr_mut(&mut self) -> *mut u8 {
        self.data.as_ptr().wrapping_add(self.write)
    }

    #[inline]
    fn remaining_bytes_mut(&self) -> usize {
        self.cap - self.write
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

impl Drop for RecvBuf {
    #[inline]
    fn drop(&mut self) {
        self.clear();
        self.free();
    }
}

impl fmt::Debug for RecvBuf {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.as_slice()).finish()
    }
}
