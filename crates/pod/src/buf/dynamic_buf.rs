use core::alloc::Layout;
use core::error;
use core::fmt;
use core::mem;
use core::ptr;
use core::slice;

use alloc::alloc;

use crate::SplitReader;
use crate::error::ErrorKind;
use crate::utils::BytesInhabited;
use crate::{AsReader, Error, Writer};

use super::CapacityError;

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
    /// buf.extend_from_words(&[1u64, 2])?;
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
    /// buf.extend_from_words(&[1u64, 2, 3, 4])?;
    /// assert_eq!(buf.as_slice(), &[1, 2, 3, 4]);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn as_slice(&self) -> &[u64] {
        // SAFETY: The buffer is guaranteed to be initialized up to `pos`.
        unsafe {
            slice::from_raw_parts(
                self.data.as_ptr().cast::<u64>().cast_const(),
                self.len.wrapping_div(mem::size_of::<u64>()),
            )
        }
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
    /// buf.extend_from_words(&[1u64, 2, 3, 4])?;
    /// assert_eq!(buf.as_slice(), &[1, 2, 3, 4]);
    /// buf.as_slice_mut()[2] = 5;
    /// assert_eq!(buf.as_slice(), &[1, 2, 5, 4]);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn as_slice_mut(&mut self) -> &mut [u64] {
        // SAFETY: The buffer is guaranteed to be initialized up to `pos`.
        unsafe {
            slice::from_raw_parts_mut(
                self.data.as_ptr().cast::<u64>(),
                self.len.wrapping_div(mem::size_of::<u64>()),
            )
        }
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
    /// buf.extend_from_words(&[1u64, 2, 3, 4]);
    /// assert_eq!(buf.len(), 32);
    /// assert_eq!(buf.as_slice(), &[1, 2, 3, 4]);
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
        let ptr = self.data.as_ptr().cast_const().cast();
        let len = self.len.wrapping_div(mem::size_of::<u64>());
        self.len = 0;
        // SAFETY: The buffer is guaranteed to be initialized up to `len`.
        unsafe { slice::from_raw_parts(ptr, len) }
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
    fn reserve_words(&mut self, words: &[u64]) -> Result<Self::Pos, Error> {
        let words_len = words.len().wrapping_mul(mem::size_of::<u64>());
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
    fn write_words(&mut self, words: &[u64]) -> Result<(), Error> {
        self.extend_from_words(words)?;
        Ok(())
    }

    #[inline]
    fn write_words_at(&mut self, pos: Self::Pos, words: &[u64]) -> Result<(), Error> {
        let Pos { at, len } = pos;

        let words_len = words.len().wrapping_mul(mem::size_of::<u64>());

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

    #[inline]
    fn write_bytes(&mut self, bytes: &[u8], pad: usize) -> Result<(), Error> {
        let padded_len = bytes
            .len()
            .wrapping_add(pad)
            .next_multiple_of(mem::size_of::<u64>());
        let len = self.len.wrapping_add(padded_len);

        if len < self.len {
            return Err(Error::new(ErrorKind::CapacityError(CapacityError)));
        }

        self.reserve(len)?;

        // SAFETY: We are writing to a valid position in the buffer.
        unsafe {
            let ptr = self.data.as_ptr().wrapping_add(self.len).cast::<u8>();
            ptr.copy_from_nonoverlapping(bytes.as_ptr(), bytes.len());
            let pad = padded_len - bytes.len();
            ptr.wrapping_add(bytes.len()).write_bytes(0, pad);
        }

        self.len = len;
        Ok(())
    }
}

impl fmt::Debug for DynamicBuf {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.as_slice()).finish()
    }
}
