use core::alloc::Layout;
use core::fmt;
use core::mem;
use core::ptr;
use core::slice;

use alloc::alloc;

use pod::utils::{Align, AlignableWith, BytesInhabited, UninitAlign};

use super::AllocError;

/// A buffer which can be used in combination with a channel.
pub struct SendBuf<T = u64> {
    data: ptr::NonNull<T>,
    cap: usize,
    read: usize,
    write: usize,
    // Partially written words in bytes written.
    partial_read: usize,
}

impl<T> SendBuf<T> {
    /// The size in bytes of a word.
    pub const WORD_SIZE: usize = const {
        if mem::size_of::<T>() == 0 {
            panic!("Cannot create a SendBuf with zero-sized type")
        }

        mem::size_of::<T>()
    };

    /// Construct a new empty buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use protocol::buf::SendBuf;
    ///
    /// let expected = u64::to_ne_bytes(0x7f7f7f77f7f7f7ffu64);
    ///
    /// let mut buf = SendBuf::<u64>::new();
    /// assert!(buf.is_empty());
    /// buf.push_bytes(0x7f7f7f77f7f7f7ffu64)?;
    /// assert_eq!(buf.len(), 1);
    /// assert_eq!(buf.as_bytes(), expected);
    /// # Ok::<_, protocol::buf::AllocError>(())
    /// ```
    #[inline]
    pub const fn new() -> Self {
        SendBuf {
            data: ptr::NonNull::<T>::dangling().cast(),
            cap: 0,
            read: 0,
            write: 0,
            partial_read: 0,
        }
    }

    /// Get the remaining readable capacity of the buffer
    ///
    /// # Examples
    ///
    /// ```
    /// use protocol::buf::SendBuf;
    ///
    /// let mut buf = SendBuf::<u64>::new();
    /// assert!(buf.is_empty());
    /// buf.push_bytes(42u64)?;
    /// assert_eq!(buf.len(), 1);
    ///
    /// let expected = 42u64.to_ne_bytes();
    /// assert_eq!(buf.as_bytes(), &expected[..]);
    /// # Ok::<_, protocol::buf::AllocError>(())
    /// ```
    #[inline]
    pub fn len(&self) -> usize {
        let add = self.partial_read.next_multiple_of(Self::WORD_SIZE) / Self::WORD_SIZE;
        (self.write - self.read).wrapping_sub(add)
    }

    /// Test if the buffer is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use protocol::buf::SendBuf;
    ///
    /// let mut buf = SendBuf::<u64>::new();
    /// assert!(buf.is_empty());
    /// buf.push_bytes(42u64)?;
    /// assert_eq!(buf.len(), 1);
    ///
    /// let expected = 42u64.to_ne_bytes();
    /// assert_eq!(buf.as_bytes(), &expected[..]);
    /// # Ok::<_, protocol::buf::AllocError>(())
    /// ```
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.read == self.write && self.partial_read == 0
    }

    /// Get the remaining readable capacity of the buffer
    ///
    /// # Examples
    ///
    /// ```
    /// use protocol::buf::SendBuf;
    ///
    /// let mut buf = SendBuf::<u32>::new();
    /// assert_eq!(buf.len(), 0);
    /// buf.push_bytes(42u64)?;
    /// assert_eq!(buf.len(), 2);
    ///
    /// let expected = 42u64.to_ne_bytes();
    /// assert_eq!(buf.remaining_bytes(), 8);
    /// assert_eq!(buf.as_bytes(), expected);
    /// # Ok::<_, protocol::buf::AllocError>(())
    /// ```
    #[inline]
    pub fn remaining_bytes(&self) -> usize {
        (self.write - self.read)
            .wrapping_mul(Self::WORD_SIZE)
            .wrapping_sub(self.partial_read)
    }

    /// Clear the contents of the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use protocol::buf::SendBuf;
    ///
    /// let mut buf = SendBuf::<u32>::new();
    /// assert!(buf.is_empty());
    ///
    /// buf.extend_from_words(&[1, 2])?;
    /// assert_eq!(buf.as_slice(), &[1, 2]);
    ///
    /// buf.clear();
    /// assert!(buf.is_empty());
    /// assert_eq!(buf.as_slice(), &[]);
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

    /// Returns the bytes of the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use protocol::buf::SendBuf;
    ///
    /// let mut buf = SendBuf::<u32>::new();
    /// assert_eq!(buf.len(), 0);
    /// buf.push_bytes(42u64)?;
    /// assert_eq!(buf.len(), 2);
    ///
    /// let expected = 42u64.to_ne_bytes();
    /// assert_eq!(buf.remaining_bytes(), 8);
    /// assert_eq!(buf.as_bytes(), expected);
    /// # Ok::<_, protocol::buf::AllocError>(())
    /// ```
    #[inline]
    pub fn as_bytes(&self) -> &[u8]
    where
        T: BytesInhabited,
    {
        // SAFETY: The buffer is guaranteed to initialized due to invariants.
        unsafe { slice::from_raw_parts(self.as_bytes_ptr(), self.remaining_bytes()) }
    }

    /// Returns the slice of data in the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use protocol::buf::SendBuf;
    ///
    /// let mut buf = SendBuf::<u64>::new();
    /// assert_eq!(buf.as_slice().len(), 0);
    ///
    /// buf.extend_from_words(&[42])?;
    /// assert_eq!(buf.as_slice(), &[42]);
    /// # Ok::<_, protocol::buf::AllocError>(())
    /// ```
    #[inline]
    pub fn as_slice(&self) -> &[T] {
        // SAFETY: The buffer is guaranteed to be initialized up to `pos`.
        unsafe { slice::from_raw_parts(self.as_ptr(), self.len()) }
    }

    /// Extend the buffer with a slice of words.
    ///
    /// # Examples
    ///
    /// ```
    /// use protocol::buf::SendBuf;
    ///
    /// let mut buf = SendBuf::<u32>::new();
    /// assert!(buf.is_empty());
    /// buf.extend_from_words(&[1, 2, 3, 4]);
    /// assert_eq!(buf.len(), 4);
    ///
    /// assert_eq!(buf.as_slice(), &[1, 2, 3, 4]);
    /// # Ok::<_, protocol::buf::AllocError>(())
    /// ```
    #[inline]
    pub fn extend_from_words(&mut self, words: &[T]) -> Result<(), AllocError> {
        self.reserve(self.write + words.len())?;

        // SAFETY: Necessary invariants have been checked above.
        unsafe {
            self.data
                .as_ptr()
                .add(self.write)
                .copy_from_nonoverlapping(words.as_ptr(), words.len());

            self.advance_written(words.len());
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
    /// let mut buf = SendBuf::<u32>::new();
    /// assert_eq!(buf.len(), 0);
    /// buf.push_bytes(42u64);
    /// assert_eq!(buf.len(), 2);
    ///
    /// let expected = 42u64.to_ne_bytes();
    /// assert_eq!(buf.as_bytes(), expected);
    /// # Ok::<_, protocol::buf::AllocError>(())
    /// ```
    #[inline]
    pub fn push_bytes<U>(&mut self, value: U) -> Result<(), AllocError>
    where
        U: AlignableWith<T>,
    {
        let value = Align(value);
        self.reserve(self.write + value.size::<T>())?;

        // SAFETY: Necessary invariants have been checked above.
        unsafe {
            self.data
                .as_ptr()
                .add(self.write)
                .copy_from_nonoverlapping(value.as_ptr::<T>(), value.size::<T>());

            self.advance_written(value.size::<T>());
        }

        Ok(())
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
    /// use protocol::buf::SendBuf;
    ///
    /// let expected = 42u32;
    ///
    /// let mut buf = SendBuf::<u32>::new();
    ///
    /// buf.push_bytes(expected)?;
    ///
    /// assert_eq!(buf.read_words(1), Some(&[expected][..]));
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
    /// let expected = 0x7f7f7f7fu32;
    /// let bytes = u32::to_ne_bytes(expected);
    ///
    /// let mut buf = SendBuf::<u32>::new();
    /// buf.push_bytes(expected)?;
    ///
    /// assert_eq!(buf.as_bytes(), &bytes[..]);
    /// assert_eq!(buf.as_slice(), &[expected]);
    ///
    /// unsafe {
    ///     buf.advance_read_bytes(1);
    /// }
    ///
    /// assert_eq!(buf.as_bytes(), &bytes[1..]);
    /// assert_eq!(buf.as_slice(), &[]);
    /// # Ok::<_, protocol::buf::AllocError>(())
    /// ```
    #[inline]
    pub unsafe fn advance_read_bytes(&mut self, n: usize) {
        let n = n + mem::take(&mut self.partial_read);
        let r = n / Self::WORD_SIZE;
        let p = n % Self::WORD_SIZE;

        let read = self.read + r;

        debug_assert!(
            read <= self.write,
            "Read position {read} in buffer is greater than write {}",
            self.write
        );

        self.read = read;
        self.partial_read = p;

        if self.read >= self.write {
            self.read = 0;
            self.write = 0;
            self.partial_read = 0;
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

        if self.read == self.write && self.partial_read == 0 {
            self.read = 0;
            self.write = 0;
            self.partial_read = 0;
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
        self.data
            .as_ptr()
            .wrapping_add(self.read)
            .cast::<u8>()
            .wrapping_add(self.partial_read)
            .cast_const()
    }

    #[inline]
    fn as_ptr(&self) -> *const T {
        let add = self.partial_read.next_multiple_of(Self::WORD_SIZE) / Self::WORD_SIZE;

        self.data
            .as_ptr()
            .wrapping_add(self.read)
            .cast_const()
            .wrapping_add(add)
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
            self.partial_read = 0;
        }
    }
}

impl<T> Drop for SendBuf<T> {
    #[inline]
    fn drop(&mut self) {
        self.clear();
        self.free();
    }
}

impl<T> fmt::Debug for SendBuf<T>
where
    T: fmt::Debug,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.as_slice()).finish()
    }
}
