use core::alloc::Layout;
use core::fmt;
use core::mem;
use core::ptr;
use core::slice;

use alloc::alloc;

use pod::utils::{AlignableWith, BytesInhabited, UninitAlign};

use super::AllocError;

pub(crate) const WANTS_BYTES: usize = 1 << 14;

/// A buffer which can be used in combination with a channel.
pub struct RecvBuf<T = u64> {
    data: ptr::NonNull<T>,
    cap: usize,
    read: usize,
    write: usize,
    // Partially written words in bytes written.
    partial_write: usize,
}

impl<T> RecvBuf<T> {
    /// The size in bytes of a word.
    pub const WORD_SIZE: usize = const {
        if mem::size_of::<T>() == 0 {
            panic!("Cannot create a RecvBuf with zero-sized type")
        }

        mem::size_of::<T>()
    };

    /// The minimum number of words that will be available when calling
    /// `as_bytes_mut`.
    const MIN_WORDS: usize = const {
        match WANTS_BYTES.wrapping_div(Self::WORD_SIZE) {
            n if n < 16 => 16,
            n => n,
        }
    };

    /// Construct a new empty buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use protocol::buf::RecvBuf;
    ///
    /// let expected = u64::to_ne_bytes(0x7f7f7f7f);
    ///
    /// let mut buf = RecvBuf::<u32>::new();
    /// assert!(buf.is_empty());
    /// buf.as_bytes_mut()?[..8].copy_from_slice(&expected[..]);
    ///
    /// unsafe {
    ///     buf.advance_written_bytes(8);
    /// }
    ///
    /// assert_eq!(buf.len(), 2);
    /// assert_eq!(buf.remaining_bytes(), 8);
    /// # Ok::<_, protocol::buf::AllocError>(())
    /// ```
    #[inline]
    pub const fn new() -> Self {
        RecvBuf {
            data: ptr::NonNull::<T>::dangling().cast(),
            cap: 0,
            read: 0,
            write: 0,
            partial_write: 0,
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
    /// let mut buf = RecvBuf::<u32>::new();
    /// assert!(buf.is_empty());
    /// buf.as_bytes_mut()?[..8].copy_from_slice(&expected[..]);
    ///
    /// unsafe {
    ///     buf.advance_written_bytes(8);
    /// }
    ///
    /// assert_eq!(buf.len(), 2);
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
    /// let mut buf = RecvBuf::<u32>::new();
    /// assert!(buf.is_empty());
    /// buf.as_bytes_mut()?[..8].copy_from_slice(&expected[..]);
    ///
    /// unsafe {
    ///     buf.advance_written_bytes(8);
    /// }
    ///
    /// assert!(!buf.is_empty());
    /// assert_eq!(buf.len(), 2);
    /// assert_eq!(buf.remaining_bytes(), 8);
    /// # Ok::<_, protocol::buf::AllocError>(())
    /// ```
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.read == self.write && self.partial_write == 0
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
    /// let mut buf = RecvBuf::<u32>::new();
    /// assert!(buf.is_empty());
    /// buf.as_bytes_mut()?[..8].copy_from_slice(&expected[..]);
    ///
    /// unsafe {
    ///     buf.advance_written_bytes(8);
    /// }
    ///
    /// assert_eq!(buf.len(), 2);
    /// assert_eq!(buf.remaining_bytes(), 8);
    /// # Ok::<_, protocol::buf::AllocError>(())
    /// ```
    #[inline]
    pub fn remaining_bytes(&self) -> usize {
        (self.write - self.read)
            .wrapping_mul(Self::WORD_SIZE)
            .wrapping_add(self.partial_write)
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
    /// let mut buf = RecvBuf::<u32>::new();
    /// assert!(buf.is_empty());
    /// buf.as_bytes_mut()?[..8].copy_from_slice(&expected[..]);
    ///
    /// unsafe {
    ///     buf.advance_written_bytes(8);
    /// }
    ///
    /// assert_eq!(buf.len(), 2);
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
        let read = mem::take(&mut self.read);
        let write = mem::take(&mut self.write);

        if mem::needs_drop::<T>() {
            unsafe {
                ptr::drop_in_place(slice::from_raw_parts_mut(
                    self.data.as_ptr().add(read),
                    write - read,
                ));
            }
        }
    }

    /// Returns the slice of data in the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use protocol::buf::RecvBuf;
    ///
    /// let expected = u32::to_ne_bytes(0x7f7f7f7fu32);
    ///
    /// let mut buf = RecvBuf::<u32>::new();
    /// assert!(buf.is_empty());
    /// buf.as_bytes_mut()?[..4].copy_from_slice(&expected[..]);
    ///
    /// unsafe {
    ///     buf.advance_written_bytes(4);
    /// }
    ///
    /// assert_eq!(buf.len(), 1);
    /// assert_eq!(buf.as_slice(), &[0x7f7f7f7fu32]);
    /// # Ok::<_, protocol::buf::AllocError>(())
    /// ```
    #[inline]
    pub fn as_slice(&self) -> &[T] {
        // SAFETY: The buffer is guaranteed to be initialized up to `pos`.
        unsafe { slice::from_raw_parts(self.as_ptr(), self.len()) }
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
    /// let expected = u32::from_ne_bytes([1, 2, 3, 4]);
    ///
    /// let mut buf = RecvBuf::<u32>::new();
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
    /// buf.as_bytes_mut()?[..1].copy_from_slice(&[4]);
    ///
    /// unsafe {
    ///     buf.advance_written_bytes(1);
    /// }
    ///
    /// assert_eq!(buf.as_slice(), &[expected]);
    /// # Ok::<_, protocol::buf::AllocError>(())
    /// ```
    #[inline]
    pub fn as_bytes_mut(&mut self) -> Result<&mut [u8], AllocError> {
        self.reserve(self.write + Self::MIN_WORDS)?;

        Ok(unsafe {
            slice::from_raw_parts_mut(self.as_bytes_ptr_mut(), self.remaining_bytes_mut())
        })
    }

    /// Read `T` out of the buffer.
    #[inline]
    pub fn read<U>(&mut self) -> Option<U>
    where
        U: AlignableWith<T>,
    {
        if self.is_empty() {
            return None;
        }

        let mut value = UninitAlign::<U>::uninit();

        // SAFETY: Necessary invariants have been checked above.
        unsafe {
            self.data
                .as_ptr()
                .add(self.read)
                .copy_to_nonoverlapping(value.as_mut_ptr::<T>().cast(), value.size::<T>());

            self.advance_read(value.size::<T>());
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
    /// let mut buf = RecvBuf::<u32>::new();
    /// assert!(buf.as_bytes_mut()?.len() > 0);
    ///
    /// buf.as_bytes_mut()?[..7].copy_from_slice(&[1, 2, 3, 4, 5, 6, 7]);
    ///
    /// unsafe {
    ///     buf.advance_written_bytes(7);
    /// }
    ///
    /// let expected = u32::from_ne_bytes([1, 2, 3, 4]);
    /// let expected2 = u32::from_ne_bytes([5, 6, 7, 8]);
    ///
    /// assert_eq!(buf.as_slice(), &[expected]);
    ///
    /// assert!(buf.read_words(2).is_none());
    /// assert_eq!(buf.read_words(1), Some(&[expected][..]));
    ///
    /// buf.as_bytes_mut()?[..3].copy_from_slice(&[8, 9, 10]);
    ///
    /// unsafe {
    ///     buf.advance_written_bytes(3);
    /// }
    ///
    /// assert_eq!(buf.read_words(1), Some(&[expected2][..]));
    /// assert!(buf.read_words(1).is_none());
    /// # Ok::<_, protocol::buf::AllocError>(())
    /// ```
    #[inline]
    pub fn read_words(&mut self, size: usize) -> Option<&[T]>
    where
        T: BytesInhabited,
    {
        if size > self.len() {
            return None;
        }

        // SAFETY: Necessary invariants have been checked above.
        unsafe {
            let value = slice::from_raw_parts(self.data.as_ptr().add(self.read).cast::<T>(), size);
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
    /// let mut buf = RecvBuf::<u32>::new();
    /// assert!(buf.as_bytes_mut()?.len() > 0);
    ///
    /// buf.as_bytes_mut()?[..7].copy_from_slice(&[1, 2, 3, 4, 5, 6, 7]);
    ///
    /// unsafe {
    ///     buf.advance_written_bytes(7);
    /// }
    ///
    /// let expected = u32::from_ne_bytes([1, 2, 3, 4]);
    /// let expected2 = u32::from_ne_bytes([5, 6, 7, 8]);
    ///
    /// assert_eq!(buf.as_slice(), &[expected]);
    ///
    /// assert!(buf.read_words(2).is_none());
    /// assert_eq!(buf.read_words(1), Some(&[expected][..]));
    ///
    /// buf.as_bytes_mut()?[..3].copy_from_slice(&[8, 9, 10]);
    ///
    /// unsafe {
    ///     buf.advance_written_bytes(3);
    /// }
    ///
    /// assert_eq!(buf.read_words(1), Some(&[expected2][..]));
    /// assert!(buf.read_words(1).is_none());
    /// # Ok::<_, protocol::buf::AllocError>(())
    /// ```
    #[inline]
    pub unsafe fn advance_written_bytes(&mut self, n: usize)
    where
        T: BytesInhabited,
    {
        let n = n + mem::take(&mut self.partial_write);
        let w = n / Self::WORD_SIZE;
        let p = n % Self::WORD_SIZE;

        let write = self.write + w;

        assert!(
            write <= self.cap,
            "Write position {} in buffer is greater than capacity {}",
            self.write,
            self.cap
        );

        self.write = write;
        self.partial_write = p;
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

        if self.read == self.write && self.partial_write == 0 {
            self.read = 0;
            self.write = 0;
            self.partial_write = 0;
        }
    }

    #[inline]
    fn as_bytes_ptr_mut(&mut self) -> *mut u8 {
        self.data
            .as_ptr()
            .wrapping_add(self.write)
            .cast::<u8>()
            .wrapping_add(self.partial_write)
    }

    #[inline]
    fn remaining_bytes_mut(&self) -> usize {
        (self.cap - self.write)
            .wrapping_mul(Self::WORD_SIZE)
            .wrapping_sub(self.partial_write)
    }

    #[inline]
    fn as_ptr(&self) -> *const T {
        self.data.as_ptr().wrapping_add(self.read).cast_const()
    }

    /// Ensure up to the given length is reserved.
    fn reserve(&mut self, needed: usize) -> Result<(), AllocError> {
        if needed <= self.cap {
            return Ok(());
        }

        let new_cap = needed.next_power_of_two().max(16);

        let data = match self.cap {
            0 => unsafe {
                let layout = Layout::array::<T>(new_cap).map_err(|_| AllocError)?;

                let data = alloc::alloc_zeroed(layout);

                if data.is_null() {
                    return Err(AllocError);
                }

                ptr::NonNull::new_unchecked(data)
            },
            _ => unsafe {
                let old_layout = Layout::array::<T>(self.cap).map_err(|_| AllocError)?;
                let new_layout = Layout::array::<T>(new_cap).map_err(|_| AllocError)?;

                let data = alloc::realloc(self.data.as_ptr().cast(), old_layout, new_layout.size());

                if data.is_null() {
                    return Err(AllocError);
                }

                data.cast::<T>()
                    .wrapping_add(self.cap)
                    .write_bytes(0, new_cap - self.cap);
                ptr::NonNull::new_unchecked(data)
            },
        };

        self.data = data.cast();
        self.cap = new_cap;
        Ok(())
    }

    fn free(&mut self) {
        if self.cap > 0 {
            // SAFETY: The buffer is guaranteed to be allocated with the same alignment as `A`.
            unsafe {
                let layout = Layout::from_size_align_unchecked(
                    self.cap.wrapping_mul(Self::WORD_SIZE),
                    mem::align_of::<T>(),
                );
                alloc::dealloc(self.data.as_ptr().cast(), layout);
            }

            self.data = ptr::NonNull::<T>::dangling().cast();
            self.cap = 0;
            self.read = 0;
            self.write = 0;
            self.partial_write = 0;
        }
    }
}

impl<T> Drop for RecvBuf<T> {
    #[inline]
    fn drop(&mut self) {
        self.clear();
        self.free();
    }
}

impl<T> fmt::Debug for RecvBuf<T>
where
    T: fmt::Debug,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.as_slice()).finish()
    }
}
