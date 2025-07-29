use core::alloc::Layout;
use core::error;
use core::fmt;
use core::mem;
use core::ptr;
use core::slice;

use alloc::alloc;

use crate::error::ErrorKind;
use crate::utils::{Align, AlignableWith, BytesInhabited, UninitAlign};
use crate::{AsReader, Error, Writer};

pub(crate) const WANTS_BYTES: usize = 1 << 14;

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
    // Partially written words in bytes written.
    partial_read: usize,
    partial_write: usize,
}

impl<T> DynamicBuf<T> {
    /// The size in bytes of a word.
    pub const WORD_SIZE: usize = const {
        if mem::size_of::<T>() == 0 {
            panic!("Cannot create a DynamicBuf with zero-sized type")
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
    /// use pod::{DynamicBuf, Writer};
    ///
    /// let mut buf = DynamicBuf::<u64>::new();
    /// assert!(buf.is_empty());
    /// buf.push_bytes(42u64)?;
    /// assert_eq!(buf.len(), 1);
    ///
    /// let expected = 42u64.to_ne_bytes();
    /// assert_eq!(buf.as_bytes(), &expected[..]);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub const fn new() -> Self {
        DynamicBuf {
            data: ptr::NonNull::<T>::dangling().cast(),
            cap: 0,
            read: 0,
            write: 0,
            partial_read: 0,
            partial_write: 0,
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
    /// assert!(buf.is_empty());
    /// buf.push_bytes(42u64)?;
    /// assert_eq!(buf.len(), 1);
    ///
    /// let expected = 42u64.to_ne_bytes();
    /// assert_eq!(buf.as_bytes(), &expected[..]);
    /// # Ok::<_, pod::Error>(())
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
    /// use pod::{DynamicBuf, Writer};
    ///
    /// let mut buf = DynamicBuf::<u64>::new();
    /// assert!(buf.is_empty());
    /// buf.push_bytes(42u64)?;
    /// assert_eq!(buf.len(), 1);
    ///
    /// let expected = 42u64.to_ne_bytes();
    /// assert_eq!(buf.as_bytes(), &expected[..]);
    /// # Ok::<_, pod::Error>(())
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
    /// use pod::{DynamicBuf, Writer};
    ///
    /// let mut buf = DynamicBuf::<u32>::new();
    /// assert_eq!(buf.len(), 0);
    /// buf.push_bytes(42u64)?;
    /// assert_eq!(buf.len(), 2);
    ///
    /// let expected = 42u64.to_ne_bytes();
    /// assert_eq!(buf.remaining_bytes(), 8);
    /// assert_eq!(buf.as_bytes(), expected);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn remaining_bytes(&self) -> usize {
        (self.write - self.read)
            .wrapping_mul(Self::WORD_SIZE)
            .wrapping_add(self.partial_write)
            .wrapping_sub(self.partial_read)
    }

    /// Clear the contents of the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::DynamicBuf;
    ///
    /// let mut buf = DynamicBuf::<u32>::new();
    /// assert!(buf.is_empty());
    ///
    /// buf.push(1)?;
    /// buf.push(2)?;
    /// assert_eq!(buf.as_slice(), &[1, 2]);
    ///
    /// buf.as_slice_mut()[0] = 3;
    /// assert_eq!(buf.as_slice(), &[3, 2]);
    ///
    /// buf.clear();
    /// assert!(buf.is_empty());
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

    /// Returns the bytes of the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{DynamicBuf, Writer};
    ///
    /// let mut buf = DynamicBuf::<u32>::new();
    /// assert_eq!(buf.len(), 0);
    /// buf.push_bytes(42u64)?;
    /// assert_eq!(buf.len(), 2);
    ///
    /// let expected = 42u64.to_ne_bytes();
    /// assert_eq!(buf.remaining_bytes(), 8);
    /// assert_eq!(buf.as_bytes(), expected);
    /// # Ok::<_, pod::Error>(())
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
    /// use pod::DynamicBuf;
    ///
    /// let mut buf = DynamicBuf::<u64>::new();
    /// assert_eq!(buf.as_slice().len(), 0);
    ///
    /// buf.push(42u64)?;
    /// assert_eq!(buf.as_slice(), &[42]);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn as_slice(&self) -> &[T] {
        // SAFETY: The buffer is guaranteed to be initialized up to `pos`.
        unsafe { slice::from_raw_parts(self.as_ptr(), self.len()) }
    }

    /// Returns a mutable slice of data in the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{DynamicBuf, Writer};
    ///
    /// let mut buf = DynamicBuf::<u64>::new();
    /// assert_eq!(buf.len(), 0);
    /// buf.write(42u64)?;
    /// assert_eq!(buf.len(), 1);
    /// assert_eq!(buf.as_slice(), &[42]);
    ///
    /// buf.as_slice_mut()[0] = 43;
    /// assert_eq!(buf.as_slice(), &[43]);
    /// # Ok::<_, pod::Error>(())
    /// ```
    pub fn as_slice_mut(&mut self) -> &mut [T] {
        // SAFETY: The buffer is guaranteed to be initialized from the
        // `self.read..self.write` range.
        unsafe { slice::from_raw_parts_mut(self.as_ptr_mut(), self.len()) }
    }

    /// Get an initialized slice of bytes available for writing.
    ///
    /// This is useful since it allows writing native aligned values from a byte
    /// array from APIs like [`Read`].
    ///
    /// The number of bytes written should must be communicated through
    /// [`DynamicBuf::advance_written_bytes`].
    ///
    /// [`Read`]: std::io::Read
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::DynamicBuf;
    ///
    /// let expected = u32::from_ne_bytes([1, 2, 3, 4]);
    ///
    /// let mut buf = DynamicBuf::<u32>::new();
    /// assert!(buf.as_bytes_mut()?.len() > 0);
    ///
    /// buf.as_bytes_mut()?[..3].copy_from_slice(&[1, 2, 3]);
    ///
    /// unsafe {
    ///     buf.advance_written_bytes(3);
    /// }
    ///
    /// assert_eq!(buf.as_bytes(), &[1, 2, 3]);
    /// assert_eq!(buf.as_slice(), &[]);
    ///
    /// buf.as_bytes_mut()?[..1].copy_from_slice(&[4]);
    ///
    /// unsafe {
    ///     buf.advance_written_bytes(1);
    /// }
    ///
    /// assert_eq!(buf.as_bytes(), &[1, 2, 3, 4]);
    /// assert_eq!(buf.as_slice(), &[expected]);
    ///
    /// unsafe {
    ///     buf.advance_read_bytes(1);
    /// }
    ///
    /// assert_eq!(buf.as_bytes(), &[2, 3, 4]);
    /// assert_eq!(buf.as_slice(), &[]);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn as_bytes_mut(&mut self) -> Result<&mut [u8], AllocError> {
        self.reserve(self.write + Self::MIN_WORDS)?;

        Ok(unsafe {
            slice::from_raw_parts_mut(self.as_bytes_ptr_mut(), self.remaining_bytes_mut())
        })
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

    /// Push a value into the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::DynamicBuf;
    ///
    /// let mut buf = DynamicBuf::<u32>::new();
    /// assert_eq!(buf.as_slice().len(), 0);
    ///
    /// buf.push(1u32)?;
    /// buf.push(2u32)?;
    /// assert_eq!(buf.as_slice(), &[1, 2]);
    ///
    /// buf.as_slice_mut()[0] = 3;
    /// assert_eq!(buf.as_slice(), &[3, 2]);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn push(&mut self, value: T) -> Result<(), AllocError> {
        self.reserve(self.write + 1)?;

        // SAFETY: Necessary invariants have been checked above.
        unsafe {
            self.data.as_ptr().wrapping_add(self.write).write(value);
            self.advance_written(1);
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
    /// use pod::DynamicBuf;
    ///
    /// let mut buf = DynamicBuf::<u32>::new();
    /// assert_eq!(buf.len(), 0);
    /// buf.push_bytes(42u64);
    /// assert_eq!(buf.len(), 2);
    ///
    /// let expected = 42u64.to_ne_bytes();
    /// assert_eq!(buf.as_bytes(), expected);
    /// # Ok::<_, pod::Error>(())
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
    /// use pod::DynamicBuf;
    ///
    /// let mut buf = DynamicBuf::<u32>::new();
    /// assert_eq!(buf.as_slice().len(), 0);
    ///
    /// buf.push(1u32)?;
    /// buf.push(2u32)?;
    /// assert_eq!(buf.as_slice(), &[1, 2]);
    ///
    /// buf.as_slice_mut()[0] = 3;
    /// assert_eq!(buf.as_slice(), &[3, 2]);
    ///
    /// let slice = buf.read_words(2).unwrap();
    /// assert_eq!(slice, &[3, 2]);
    /// assert!(buf.read_words(1).is_none());
    /// assert!(buf.read_words(2).is_none());
    /// # Ok::<_, pod::Error>(())
    /// ```
    ///
    /// When interacting with bytes-oriented writes:
    ///
    /// ```
    /// use pod::DynamicBuf;
    ///
    /// let mut buf = DynamicBuf::<u32>::new();
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
    /// assert_eq!(buf.as_bytes(), &[1, 2, 3, 4, 5, 6, 7]);
    /// assert_eq!(buf.as_slice(), &[expected]);
    ///
    /// assert!(buf.read_words(2).is_none());
    /// assert_eq!(buf.read_words(1), Some(&[expected][..]));
    /// assert_eq!(buf.as_bytes(), &[5, 6, 7]);
    ///
    /// buf.as_bytes_mut()?[..3].copy_from_slice(&[8, 9, 10]);
    ///
    /// unsafe {
    ///     buf.advance_written_bytes(3);
    /// }
    ///
    /// assert_eq!(buf.read_words(1), Some(&[expected2][..]));
    /// assert!(buf.read_words(1).is_none());
    /// # Ok::<_, pod::Error>(())
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
    /// use pod::DynamicBuf;
    ///
    /// let expected = u32::from_ne_bytes([1, 2, 3, 4]);
    ///
    /// let mut buf = DynamicBuf::<u32>::new();
    /// assert!(buf.as_bytes_mut()?.len() > 0);
    ///
    /// buf.as_bytes_mut()?[..4].copy_from_slice(&[1, 2, 3, 4]);
    ///
    /// unsafe {
    ///     buf.advance_written_bytes(4);
    /// }
    ///
    /// assert_eq!(buf.as_bytes(), &[1, 2, 3, 4]);
    /// assert_eq!(buf.as_slice(), &[expected]);
    ///
    /// unsafe {
    ///     buf.advance_read_bytes(1);
    /// }
    ///
    /// assert_eq!(buf.as_bytes(), &[2, 3, 4]);
    /// assert_eq!(buf.as_slice(), &[]);
    /// # Ok::<_, pod::Error>(())
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
    /// use pod::DynamicBuf;
    ///
    /// let mut buf = DynamicBuf::<u32>::new();
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
    /// assert_eq!(buf.as_bytes(), &[1, 2, 3, 4, 5, 6, 7]);
    /// assert_eq!(buf.as_slice(), &[expected]);
    ///
    /// assert!(buf.read_words(2).is_none());
    /// assert_eq!(buf.read_words(1), Some(&[expected][..]));
    /// assert_eq!(buf.as_bytes(), &[5, 6, 7]);
    ///
    /// buf.as_bytes_mut()?[..3].copy_from_slice(&[8, 9, 10]);
    ///
    /// unsafe {
    ///     buf.advance_written_bytes(3);
    /// }
    ///
    /// assert_eq!(buf.read_words(1), Some(&[expected2][..]));
    /// assert!(buf.read_words(1).is_none());
    /// # Ok::<_, pod::Error>(())
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

        if self.read == self.write && self.partial_read == self.partial_write {
            self.read = 0;
            self.write = 0;
            self.partial_read = 0;
            self.partial_write = 0;
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
        // This might leak bytes-oriented writes, but that is OK.
        self.partial_write = 0;
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
        let add = self.partial_read.next_multiple_of(Self::WORD_SIZE) / Self::WORD_SIZE;

        self.data
            .as_ptr()
            .wrapping_add(self.read)
            .cast_const()
            .wrapping_add(add)
    }

    #[inline]
    fn as_ptr_mut(&mut self) -> *mut T {
        self.data.as_ptr().wrapping_add(self.read)
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

        debug_assert!(self.write <= self.cap);
        debug_assert!(write <= self.cap);
        assert_eq!(self.partial_write, 0);

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
