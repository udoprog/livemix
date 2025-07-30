use core::alloc::Layout;
use core::error;
use core::fmt;
use core::mem;
use core::ptr;
use core::slice;

use alloc::alloc;

use crate::SplitReader;
use crate::error::ErrorKind;
use crate::{AsReader, Error, Writer};

/// An allocation error has occured when trying to reserve space in the [`DynamicBuf`].
#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
#[non_exhaustive]
pub struct AllocError;

impl error::Error for AllocError {}

impl fmt::Display for AllocError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Allocation error")
    }
}

/// A buffer which can be used in combination with a channel.
pub struct DynamicBuf {
    data: ptr::NonNull<u64>,
    cap: usize,
    len: usize,
}

impl DynamicBuf {
    /// Construct a new empty buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{DynamicBuf, Writer};
    ///
    /// let mut buf = DynamicBuf::new();
    /// assert!(buf.is_empty());
    /// buf.extend_from_words(&[42])?;
    /// assert_eq!(buf.len(), 1);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub const fn new() -> Self {
        DynamicBuf {
            data: ptr::NonNull::<u64>::dangling(),
            cap: 0,
            len: 0,
        }
    }

    /// Get the remaining readable capacity of the buffer
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{DynamicBuf, Writer};
    ///
    /// let mut buf = DynamicBuf::new();
    /// assert_eq!(buf.len(), 0);
    /// buf.extend_from_words(&[42])?;
    /// assert_eq!(buf.len(), 1);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Test if the buffer is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{DynamicBuf, Writer};
    ///
    /// let mut buf = DynamicBuf::new();
    /// assert!(buf.is_empty());
    /// buf.extend_from_words(&[42])?;
    /// assert!(!buf.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Clear the contents of the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::DynamicBuf;
    ///
    /// let mut buf = DynamicBuf::new();
    ///
    /// buf.extend_from_words(&[1, 2])?;
    /// assert_eq!(buf.as_slice(), &[1, 2]);
    /// buf.clear();
    /// assert_eq!(buf.as_slice(), &[]);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn clear(&mut self) {
        self.len = 0;
    }

    /// Returns the slice of data in the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::DynamicBuf;
    ///
    /// let mut buf = DynamicBuf::new();
    /// assert_eq!(buf.as_slice().len(), 0);
    /// buf.extend_from_words(&[1, 2, 3, 4])?;
    /// assert_eq!(buf.as_slice(), &[1, 2, 3, 4]);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn as_slice(&self) -> &[u64] {
        // SAFETY: The buffer is guaranteed to be initialized up to `pos`.
        unsafe { slice::from_raw_parts(self.data.as_ptr().cast_const(), self.len()) }
    }

    /// Returns the mutable slice of data in the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::DynamicBuf;
    ///
    /// let mut buf = DynamicBuf::new();
    /// assert_eq!(buf.as_slice().len(), 0);
    /// buf.extend_from_words(&[1, 2, 3, 4])?;
    /// assert_eq!(buf.as_slice(), &[1, 2, 3, 4]);
    /// buf.as_slice_mut()[2] = 5;
    /// assert_eq!(buf.as_slice(), &[1, 2, 5, 4]);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn as_slice_mut(&mut self) -> &mut [u64] {
        // SAFETY: The buffer is guaranteed to be initialized up to `pos`.
        unsafe { slice::from_raw_parts_mut(self.data.as_ptr(), self.len()) }
    }

    /// Extend the buffer with a slice of words.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::DynamicBuf;
    ///
    /// let mut buf = DynamicBuf::new();
    /// assert!(buf.is_empty());
    /// buf.extend_from_words(&[1, 2, 3, 4]);
    /// assert_eq!(buf.len(), 4);
    /// assert_eq!(buf.as_slice(), &[1, 2, 3, 4]);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn extend_from_words(&mut self, words: &[u64]) -> Result<(), AllocError> {
        self.reserve(self.len + words.len())?;

        // SAFETY: Necessary invariants have been checked above.
        unsafe {
            self.data
                .as_ptr()
                .add(self.len)
                .copy_from_nonoverlapping(words.as_ptr(), words.len());

            self.advance_written(words.len());
        }

        Ok(())
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
        let write = self.len + n;

        // Debug assertion, since this is an internal API.
        debug_assert!(
            write <= self.cap,
            "Write position {write} in buffer is greater than capacity {}",
            self.cap
        );

        self.len = write;
    }

    /// Ensure up to the given length is reserved.
    fn reserve(&mut self, needed: usize) -> Result<(), AllocError> {
        if needed <= self.cap {
            return Ok(());
        }

        let new_cap = needed.next_power_of_two().max(16);

        let data = match self.cap {
            0 => unsafe {
                let layout = Layout::array::<u64>(new_cap).map_err(|_| AllocError)?;

                let data = alloc::alloc(layout);

                if data.is_null() {
                    return Err(AllocError);
                }

                ptr::NonNull::new_unchecked(data)
            },
            _ => unsafe {
                let old_layout = Layout::array::<u64>(self.cap).map_err(|_| AllocError)?;
                let new_layout = Layout::array::<u64>(new_cap).map_err(|_| AllocError)?;

                let data = alloc::realloc(self.data.as_ptr().cast(), old_layout, new_layout.size());

                if data.is_null() {
                    return Err(AllocError);
                }

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
                    self.cap.wrapping_mul(mem::size_of::<u64>()),
                    mem::align_of::<u64>(),
                );
                alloc::dealloc(self.data.as_ptr().cast(), layout);
            }

            self.data = ptr::NonNull::<u64>::dangling().cast();
            self.cap = 0;
            self.len = 0;
        }
    }
}

impl Drop for DynamicBuf {
    #[inline]
    fn drop(&mut self) {
        self.clear();
        self.free();
    }
}

impl AsReader for DynamicBuf {
    type AsReader<'this> = &'this [u64];

    #[inline]
    fn as_reader(&self) -> Self::AsReader<'_> {
        self.as_slice()
    }
}

impl SplitReader for DynamicBuf {
    type TakeReader<'this> = &'this [u64];

    #[inline]
    fn take_reader(&mut self) -> Self::TakeReader<'_> {
        let ptr = self.data.as_ptr().cast_const();
        let len = self.len;
        self.len = 0;
        // SAFETY: The buffer is guaranteed to be initialized up to `len`.
        unsafe { slice::from_raw_parts(ptr, len) }
    }
}

#[derive(Clone, Copy)]
pub struct Pos {
    write: usize,
    len: usize,
}

impl Writer for DynamicBuf {
    type Mut<'this>
        = &'this mut DynamicBuf
    where
        Self: 'this;

    type Pos = Pos;

    #[inline]
    fn borrow_mut(&mut self) -> Self::Mut<'_> {
        self
    }

    #[inline]
    fn reserve_words(&mut self, words: &[u64]) -> Result<Self::Pos, Error> {
        let write = self.len.wrapping_add(words.len());

        self.reserve(write)?;

        // SAFETY: We are writing to a valid position in the buffer.
        unsafe {
            self.data
                .as_ptr()
                .add(self.len)
                .copy_from_nonoverlapping(words.as_ptr().cast(), words.len());
        }

        let pos = Pos {
            write: self.len,
            len: words.len(),
        };

        self.len = write;
        Ok(pos)
    }

    #[inline]
    fn distance_from(&self, pos: Self::Pos) -> usize {
        self.len
            .wrapping_sub(pos.write)
            .wrapping_mul(mem::size_of::<u64>())
    }

    #[inline]
    fn write_words(&mut self, words: &[u64]) -> Result<(), Error> {
        self.extend_from_words(words)?;
        Ok(())
    }

    #[inline]
    fn write_words_at(&mut self, pos: Self::Pos, words: &[u64]) -> Result<(), Error> {
        let Pos { write, len } = pos;

        if len < words.len() {
            return Err(Error::new(ErrorKind::ReservedSizeMismatch {
                expected: len,
                actual: words.len(),
            }));
        }

        if write.wrapping_add(len) > self.len {
            return Err(Error::new(ErrorKind::ReservedOverflow {
                write,
                len,
                capacity: self.len,
            }));
        }

        // SAFETY: We are writing to a valid position in the buffer.
        unsafe {
            self.data
                .as_ptr()
                .add(write)
                .copy_from_nonoverlapping(words.as_ptr().cast(), words.len());
        }

        Ok(())
    }

    #[inline]
    fn write_bytes(&mut self, bytes: &[u8], pad: usize) -> Result<(), Error> {
        let Some(full) = bytes.len().checked_add(pad) else {
            return Err(Error::new(ErrorKind::SizeOverflow));
        };

        let write = self.len.wrapping_add(full.div_ceil(mem::size_of::<u64>()));

        self.reserve(write)?;

        // SAFETY: We are writing to a valid position in the buffer.
        unsafe {
            let ptr = self.data.as_ptr().wrapping_add(self.len).cast::<u8>();
            ptr.copy_from_nonoverlapping(bytes.as_ptr(), bytes.len());
            let pad = mem::size_of::<u64>() - bytes.len() % mem::size_of::<u64>();
            ptr.wrapping_add(bytes.len()).write_bytes(0, pad);
        }

        self.len = write;
        Ok(())
    }
}

impl fmt::Debug for DynamicBuf {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.as_slice()).finish()
    }
}
