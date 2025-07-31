use core::alloc::Layout;
use core::error;
use core::fmt;
use core::mem;
use core::ptr;
use core::slice;

use alloc::alloc;

use crate::Slice;
use crate::SplitReader;
use crate::error::ErrorKind;
use crate::utils::BytesInhabited;
use crate::{AsSlice, Error, Writer};

use super::CapacityError;

/// An allocation error has occured when trying to reserve space in the [`DynamicBuf`].
#[derive(Debug, PartialEq)]
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
    data: ptr::NonNull<u8>,
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
    /// buf.extend_from_words(&[42u64])?;
    /// assert_eq!(buf.len(), 8);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub const fn new() -> Self {
        DynamicBuf {
            data: ptr::NonNull::<u64>::dangling().cast(),
            cap: 0,
            len: 0,
        }
    }

    /// Construct a and initialize a new dynamic buffer with the contents of the
    /// given slice.
    pub fn from_slice(data: &[u8]) -> Result<Self, AllocError> {
        unsafe {
            let layout = Layout::from_size_align(data.len(), mem::align_of::<u64>())
                .map_err(|_| AllocError)?;
            let ptr = alloc::alloc(layout);

            if ptr.is_null() {
                return Err(AllocError);
            }

            ptr.copy_from_nonoverlapping(data.as_ptr(), data.len());

            Ok(DynamicBuf {
                data: ptr::NonNull::new_unchecked(ptr).cast(),
                cap: data.len(),
                len: data.len(),
            })
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
    /// buf.extend_from_words(&[42u64])?;
    /// assert_eq!(buf.len(), 8);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub const fn len(&self) -> usize {
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
    /// buf.extend_from_words(&[42u64])?;
    /// assert!(!buf.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub const fn is_empty(&self) -> bool {
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
    /// buf.extend_from_words(&[1u8, 2])?;
    /// assert_eq!(buf.as_bytes(), &[1, 2]);
    ///
    /// buf.clear();
    /// assert_eq!(buf.as_bytes(), &[]);
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
    /// assert_eq!(buf.len(), 0);
    ///
    /// buf.extend_from_words(&[1u8, 2, 3, 4])?;
    /// assert_eq!(buf.as_bytes(), &[1, 2, 3, 4]);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        // SAFETY: The buffer is guaranteed to be initialized up to `pos`.
        unsafe { slice::from_raw_parts(self.data.as_ptr(), self.len) }
    }

    /// Returns the mutable slice of data in the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::DynamicBuf;
    ///
    /// let mut buf = DynamicBuf::new();
    /// assert_eq!(buf.as_bytes().len(), 0);
    ///
    /// buf.extend_from_words(&[1u8, 2, 3, 4])?;
    /// assert_eq!(buf.as_bytes(), &[1, 2, 3, 4]);
    ///
    /// buf.as_bytes_mut()[2] = 5;
    /// assert_eq!(buf.as_bytes(), &[1, 2, 5, 4]);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        // SAFETY: The buffer is guaranteed to be initialized up to `pos`.
        unsafe { slice::from_raw_parts_mut(self.data.as_ptr(), self.len) }
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
    ///
    /// buf.extend_from_words(&[1u8, 2, 3, 4]);
    /// assert_eq!(buf.len(), 4);
    /// assert_eq!(buf.as_bytes(), &[1, 2, 3, 4]);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn extend_from_words<T>(&mut self, words: &[T]) -> Result<(), AllocError>
    where
        T: BytesInhabited,
    {
        let len = words.len().wrapping_mul(mem::size_of::<T>());
        self.reserve(self.len + len)?;

        // SAFETY: Necessary invariants have been checked above.
        unsafe {
            self.data
                .as_ptr()
                .add(self.len)
                .copy_from_nonoverlapping(words.as_ptr().cast(), len);

            self.advance_written(len);
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
        let len = self.len + n;

        // Debug assertion, since this is an internal API.
        debug_assert!(
            len <= self.cap,
            "Write position {len} in buffer is greater than capacity {}",
            self.cap
        );

        self.len = len;
    }

    /// Ensure up to the given length is reserved.
    fn reserve(&mut self, needed: usize) -> Result<(), AllocError> {
        if needed <= self.cap {
            return Ok(());
        }

        let new_cap = needed
            .next_power_of_two()
            .max(16)
            .div_ceil(mem::size_of::<u64>());

        let (data, cap) = match self.cap {
            0 => unsafe {
                let layout = Layout::array::<u64>(new_cap).map_err(|_| AllocError)?;

                let data = alloc::alloc(layout);

                if data.is_null() {
                    return Err(AllocError);
                }

                (ptr::NonNull::new_unchecked(data), layout.size())
            },
            _ => unsafe {
                let old_layout =
                    Layout::from_size_align_unchecked(self.cap, mem::align_of::<u64>());
                let new_layout = Layout::array::<u64>(new_cap).map_err(|_| AllocError)?;

                let data = alloc::realloc(self.data.as_ptr().cast(), old_layout, new_layout.size());

                if data.is_null() {
                    return Err(AllocError);
                }

                (ptr::NonNull::new_unchecked(data), new_layout.size())
            },
        };

        self.data = data.cast();
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

impl AsSlice for DynamicBuf {
    #[inline]
    fn as_slice(&self) -> Slice<'_> {
        Slice::new(self.as_bytes())
    }
}

impl SplitReader for DynamicBuf {
    type TakeReader<'this> = Slice<'this>;

    #[inline]
    fn take_reader(&mut self) -> Self::TakeReader<'_> {
        let ptr = self.data.as_ptr().cast_const();
        let len = mem::take(&mut self.len);
        // SAFETY: The buffer is guaranteed to be initialized up to `len`.
        Slice::new(unsafe { slice::from_raw_parts(ptr, len) })
    }
}

#[derive(Clone, Copy)]
pub struct Pos {
    at: usize,
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
    fn reserve<T>(&mut self, words: &[T]) -> Result<Self::Pos, Error>
    where
        T: BytesInhabited,
    {
        let words_len = words.len().wrapping_mul(mem::size_of::<T>());
        let len = self.len.wrapping_add(words_len);

        self.reserve(len)?;

        // SAFETY: We are writing to a valid position in the buffer.
        unsafe {
            self.data
                .as_ptr()
                .add(self.len)
                .copy_from_nonoverlapping(words.as_ptr().cast::<u8>(), words_len);
        }

        let pos = Pos {
            at: self.len,
            len: words_len,
        };

        self.len = len;
        Ok(pos)
    }

    #[inline]
    fn distance_from(&self, pos: Self::Pos) -> usize {
        self.len.wrapping_sub(pos.at)
    }

    #[inline]
    fn write<T>(&mut self, words: &[T]) -> Result<(), Error>
    where
        T: BytesInhabited,
    {
        self.extend_from_words(words)?;
        Ok(())
    }

    #[inline]
    fn write_at<T>(&mut self, pos: Self::Pos, words: &[T]) -> Result<(), Error>
    where
        T: BytesInhabited,
    {
        let Pos { at, len } = pos;

        let words_len = words.len().wrapping_mul(mem::size_of::<T>());

        if len < words_len {
            return Err(Error::new(ErrorKind::ReservedSizeMismatch {
                expected: len,
                actual: words_len,
            }));
        }

        if at.wrapping_add(len) > self.len {
            return Err(Error::new(ErrorKind::ReservedOverflow {
                write: at,
                len,
                capacity: self.len,
            }));
        }

        // SAFETY: We are writing to a valid position in the buffer.
        unsafe {
            self.data
                .as_ptr()
                .add(at)
                .copy_from_nonoverlapping(words.as_ptr().cast(), words_len);
        }

        Ok(())
    }

    /// Write a slice of bytes to the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{DynamicBuf, Writer};
    ///
    /// let mut buf = DynamicBuf::new();
    /// buf.write_bytes(&[1, 2, 3], 3)?;
    /// assert_eq!(buf.as_bytes(), &[1, 2, 3, 0, 0, 0]);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    fn write_bytes(&mut self, bytes: &[u8], pad: usize) -> Result<(), Error> {
        let len = self.len.wrapping_add(bytes.len().wrapping_add(pad));

        if len < self.len {
            return Err(Error::new(ErrorKind::CapacityError(CapacityError)));
        }

        self.reserve(len)?;

        // SAFETY: We are writing to a valid position in the buffer.
        unsafe {
            let ptr = self.data.as_ptr().wrapping_add(self.len).cast::<u8>();
            ptr.copy_from_nonoverlapping(bytes.as_ptr(), bytes.len());
            ptr.add(bytes.len()).write_bytes(0, pad);
        }

        self.len = len;
        Ok(())
    }

    /// Pad a buffer to the specified alignment.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{DynamicBuf, Writer};
    ///
    /// let mut buf = DynamicBuf::default();
    /// buf.write_bytes(&[1, 2, 3], 3)?;
    /// assert_eq!(buf.as_bytes(), &[1, 2, 3, 0, 0, 0]);
    /// buf.pad(8)?;
    /// assert_eq!(buf.as_bytes(), &[1, 2, 3, 0, 0, 0, 0, 0]);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    fn pad(&mut self, align: usize) -> Result<(), Error> {
        let remaining = self.len % align;

        if remaining == 0 {
            return Ok(());
        }

        let pad = align - remaining;
        let new_len = self.len.wrapping_add(pad);

        self.reserve(new_len)?;

        // SAFETY: We are writing to a valid position in the buffer.
        unsafe {
            self.data.as_ptr().add(self.len).write_bytes(0, pad);
        }

        self.len = new_len;
        Ok(())
    }
}

impl Default for DynamicBuf {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for DynamicBuf {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.as_bytes()).finish()
    }
}
