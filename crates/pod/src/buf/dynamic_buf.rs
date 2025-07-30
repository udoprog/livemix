use core::alloc::Layout;
use core::error;
use core::fmt;
use core::mem;
use core::ptr;
use core::slice;

use alloc::alloc;

use crate::error::ErrorKind;
use crate::utils::BytesInhabited;
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
pub struct DynamicBuf<T = u64> {
    data: ptr::NonNull<T>,
    cap: usize,
    read: usize,
    write: usize,
}

impl<T> DynamicBuf<T> {
    /// The size in bytes of a word.
    pub const WORD_SIZE: usize = const {
        if mem::size_of::<T>() == 0 {
            panic!("Cannot create a DynamicBuf with zero-sized type")
        }

        mem::size_of::<T>()
    };

    /// Construct a new empty buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{DynamicBuf, Writer};
    ///
    /// let mut buf = DynamicBuf::<u64>::new();
    /// assert!(buf.is_empty());
    /// buf.extend_from_words(&[42])?;
    /// assert_eq!(buf.len(), 1);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub const fn new() -> Self {
        DynamicBuf {
            data: ptr::NonNull::<T>::dangling().cast(),
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
    /// use pod::{DynamicBuf, Writer};
    ///
    /// let mut buf = DynamicBuf::<u64>::new();
    /// assert_eq!(buf.len(), 0);
    /// buf.extend_from_words(&[42])?;
    /// assert_eq!(buf.len(), 1);
    /// # Ok::<_, pod::Error>(())
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
    /// use pod::{DynamicBuf, Writer};
    ///
    /// let mut buf = DynamicBuf::<u64>::new();
    /// assert!(buf.is_empty());
    /// buf.extend_from_words(&[42])?;
    /// assert!(!buf.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.read == self.write
    }

    /// Clear the contents of the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::DynamicBuf;
    ///
    /// let mut buf = DynamicBuf::<u32>::new();
    ///
    /// buf.extend_from_words(&[1, 2])?;
    /// assert_eq!(buf.as_slice(), &[1, 2]);
    /// buf.clear();
    /// assert_eq!(buf.as_slice(), &[]);
    /// # Ok::<_, pod::Error>(())
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
    /// use pod::DynamicBuf;
    ///
    /// let mut buf = DynamicBuf::<u64>::new();
    /// assert_eq!(buf.as_slice().len(), 0);
    /// buf.extend_from_words(&[1, 2, 3, 4])?;
    /// assert_eq!(buf.as_slice(), &[1, 2, 3, 4]);
    /// # Ok::<_, pod::Error>(())
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
    /// use pod::DynamicBuf;
    ///
    /// let mut buf = DynamicBuf::<u32>::new();
    /// assert!(buf.is_empty());
    /// buf.extend_from_words(&[1, 2, 3, 4]);
    /// assert_eq!(buf.len(), 4);
    ///
    /// assert_eq!(buf.as_slice(), &[1, 2, 3, 4]);
    /// # Ok::<_, pod::Error>(())
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
        }
    }
}

impl<T> Drop for DynamicBuf<T> {
    #[inline]
    fn drop(&mut self) {
        self.clear();
        self.free();
    }
}

impl<T> AsReader<T> for DynamicBuf<T>
where
    T: 'static,
{
    type AsReader<'this> = &'this [T];

    #[inline]
    fn as_reader(&self) -> Self::AsReader<'_> {
        self.as_slice()
    }
}

#[derive(Clone, Copy)]
pub struct Pos {
    write: usize,
    len: usize,
}

impl<T> Writer<T> for DynamicBuf<T>
where
    T: BytesInhabited,
{
    type Mut<'this>
        = &'this mut DynamicBuf<T>
    where
        Self: 'this;

    type Pos = Pos;

    #[inline]
    fn borrow_mut(&mut self) -> Self::Mut<'_> {
        self
    }

    #[inline]
    fn reserve_words(&mut self, words: &[T]) -> Result<Self::Pos, Error> {
        let write = self.write.wrapping_add(words.len());

        self.reserve(write)?;

        // SAFETY: We are writing to a valid position in the buffer.
        unsafe {
            self.data
                .as_ptr()
                .add(self.write)
                .copy_from_nonoverlapping(words.as_ptr().cast(), words.len());
        }

        let pos = Pos {
            write: self.write,
            len: words.len(),
        };

        self.write = write;
        Ok(pos)
    }

    #[inline]
    fn distance_from(&self, pos: Self::Pos) -> usize {
        self.write
            .wrapping_sub(pos.write)
            .wrapping_mul(Self::WORD_SIZE)
    }

    #[inline]
    fn write_words(&mut self, words: &[T]) -> Result<(), Error> {
        self.extend_from_words(words)?;
        Ok(())
    }

    #[inline]
    fn write_words_at(&mut self, pos: Self::Pos, words: &[T]) -> Result<(), Error> {
        let Pos { write, len } = pos;

        if len < words.len() {
            return Err(Error::new(ErrorKind::ReservedSizeMismatch {
                expected: len,
                actual: words.len(),
            }));
        }

        if write.wrapping_add(len) > self.write {
            return Err(Error::new(ErrorKind::ReservedOverflow {
                write,
                len,
                capacity: self.write,
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
    fn write_bytes(&mut self, bytes: &[u8], pad: usize) -> Result<(), Error>
    where
        T: BytesInhabited,
    {
        let Some(full) = bytes.len().checked_add(pad) else {
            return Err(Error::new(ErrorKind::SizeOverflow));
        };

        let write = self.write.wrapping_add(full.div_ceil(Self::WORD_SIZE));

        self.reserve(write)?;

        // SAFETY: We are writing to a valid position in the buffer.
        unsafe {
            let ptr = self.data.as_ptr().wrapping_add(self.write).cast::<u8>();
            ptr.copy_from_nonoverlapping(bytes.as_ptr(), bytes.len());
            let pad = Self::WORD_SIZE - bytes.len() % Self::WORD_SIZE;
            ptr.wrapping_add(bytes.len()).write_bytes(0, pad);
        }

        self.write = write;
        Ok(())
    }
}

impl<T> fmt::Debug for DynamicBuf<T>
where
    T: fmt::Debug,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.as_slice()).finish()
    }
}
