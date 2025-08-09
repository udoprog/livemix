use core::fmt;
use core::mem::{self, MaybeUninit};
use core::slice;

use crate::utils::BytesInhabited;
use crate::writer::Pos;
use crate::{AsSlice, Error, ErrorKind, Slice, SplitReader, Writer};

use super::CapacityError;

const DEFAULT_SIZE: usize = 1024;

/// A fixed-size buffer with a flexible read and write position.
///
/// The initialized slice of the buffer is defined by the region betweeen the
/// `read` and `write` positions.
///
/// # Examples
///
/// ```
/// use pod::ArrayBuf;
///
/// let expected = u64::to_ne_bytes(42);
///
/// let mut buf = ArrayBuf::<128>::from_slice(&[1u8, 2, 3, 4])?;
/// assert_eq!(buf.len(), 4);
/// buf.extend_from_words(&[42u64])?;
/// assert_eq!(&buf.as_bytes()[..4], &[1, 2, 3, 4]);
/// assert_eq!(&buf.as_bytes()[4..], &expected);
/// assert_eq!(buf.len(), 12);
/// # Ok::<_, pod::buf::CapacityError>(())
/// ```
#[repr(C, align(8))]
pub struct ArrayBuf<const N: usize = DEFAULT_SIZE> {
    data: [MaybeUninit<u8>; N],
    len: usize,
}

impl<const N: usize> ArrayBuf<N> {
    /// Construct a new array buffer with a default size.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::ArrayBuf;
    ///
    /// let buf = ArrayBuf::<3>::new();
    /// assert_eq!(buf.capacity(), 3);
    /// ```
    #[inline]
    pub const fn new() -> Self {
        // SAFETY: The buffer is a sequence of uninitialized elements.
        Self {
            data: unsafe { MaybeUninit::uninit().assume_init() },
            len: 0,
        }
    }

    /// Extend the buffer with a slice of words.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::ArrayBuf;
    ///
    /// let mut buf = ArrayBuf::default();
    ///
    /// buf.extend_from_words(&[1u8, 2, 3, 4])?;
    /// assert_eq!(buf.as_bytes(), &[1, 2, 3, 4]);
    /// # Ok::<_, pod::buf::CapacityError>(())
    /// ```
    pub fn extend_from_words<T>(&mut self, words: &[T]) -> Result<(), CapacityError>
    where
        T: BytesInhabited,
    {
        let len = words.len().wrapping_mul(mem::size_of::<T>());
        let new_len = self.len.wrapping_add(len);

        // Ensure we have enough space in the buffer.
        if !(self.len..=N).contains(&new_len) {
            return Err(CapacityError);
        }

        // SAFETY: We are writing to a valid position in the buffer.
        unsafe {
            self.data
                .as_mut_ptr()
                .add(self.len)
                .copy_from_nonoverlapping(words.as_ptr().cast(), len);
        }

        self.len = new_len;
        Ok(())
    }
}

impl Default for ArrayBuf {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> ArrayBuf<N> {
    /// Construct from an initialized array.
    ///
    /// # Errors
    ///
    /// This errors with a [`CapacityError`] if the provided slice is larger
    /// than the array being constructed.
    ///
    /// ```should_panic
    /// use pod::ArrayBuf;
    ///
    /// ArrayBuf::<16>::from_slice(&[0u32; 32])?;
    /// # Ok::<_, pod::buf::CapacityError>(())
    /// ```
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::ArrayBuf;
    ///
    /// let buf = ArrayBuf::<128>::from_slice(&[1u32, 2, 3])?;
    /// assert_eq!(buf.len(), 12);
    /// assert_eq!(buf.capacity(), 128);
    /// assert_eq!(buf.as_bytes(), &[1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0]);
    /// # Ok::<_, pod::buf::CapacityError>(())
    /// ```
    pub fn from_slice<T>(words: &[T]) -> Result<Self, CapacityError>
    where
        T: BytesInhabited,
    {
        let mut buf = Self::new();
        buf.extend_from_words(words)?;
        Ok(buf)
    }

    /// Returns the length of the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::ArrayBuf;
    ///
    /// let mut buf = ArrayBuf::default();
    /// assert!(buf.is_empty());
    /// assert_eq!(buf.len(), 0);
    /// buf.extend_from_words(&[42u64])?;
    /// assert!(!buf.is_empty());
    /// assert_eq!(buf.len(), 8);
    /// # Ok::<_, pod::buf::CapacityError>(())
    /// ```
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Test if the buffer is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::ArrayBuf;
    ///
    /// let mut buf = ArrayBuf::default();
    /// assert!(buf.is_empty());
    /// assert_eq!(buf.len(), 0);
    /// buf.extend_from_words(&[42u64])?;
    /// assert!(!buf.is_empty());
    /// assert_eq!(buf.len(), 8);
    /// # Ok::<_, pod::buf::CapacityError>(())
    /// ```
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns the capacity of the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::ArrayBuf;
    ///
    /// let buf = ArrayBuf::<128>::from_slice(&[1u8, 2, 3])?;
    /// assert_eq!(buf.len(), 3);
    /// assert_eq!(buf.capacity(), 128);
    /// # Ok::<_, pod::buf::CapacityError>(())
    /// ```
    pub const fn capacity(&self) -> usize {
        N
    }

    /// Resets the buffer to an empty state.
    ///
    /// This clears the content of the buffer that can be read, treating any
    /// previously written data as uninitialized.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Reader};
    ///
    /// let mut buf = ArrayBuf::<24>::from_slice(&[1u8, 2, 3])?;
    /// assert_eq!(buf.len(), 3);
    /// assert_eq!(buf.as_bytes(), &[1, 2, 3]);
    /// buf.clear();
    /// assert!(buf.as_bytes().is_empty());
    ///
    /// let mut buf = ArrayBuf::<16>::from_slice(&[0x10u8, 0x20])?;
    /// assert_eq!(buf.len(), 2);
    /// assert_eq!(buf.as_bytes(), &[0x10, 0x20]);
    /// buf.clear();
    /// assert!(buf.is_empty());
    /// # Ok::<_, pod::buf::CapacityError>(())
    /// ```
    #[inline]
    pub fn clear(&mut self) {
        self.len = 0;
    }

    /// Returns the bytes of the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::ArrayBuf;
    ///
    /// let expected = u64::to_ne_bytes(42);
    ///
    /// let mut buf = ArrayBuf::<128>::from_slice(&[42u64])?;
    /// assert_eq!(buf.as_bytes().len(), 8);
    /// assert_eq!(buf.as_bytes(), &expected);
    /// # Ok::<_, pod::buf::CapacityError>(())
    /// ```
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        // SAFETY: The buffer is guaranteed to be initialized up to `pos`.
        unsafe { slice::from_raw_parts(self.data.as_ptr().cast(), self.len) }
    }

    /// Returns a mutable slice of data in the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::ArrayBuf;
    ///
    /// let expected = u64::to_ne_bytes(42);
    ///
    /// let mut buf = ArrayBuf::<128>::from_slice(&[42u64])?;
    /// assert_eq!(buf.as_bytes().len(), 8);
    /// assert_eq!(buf.as_bytes(), &expected);
    ///
    /// buf.as_bytes_mut()[0] = u8::MAX;
    /// assert_eq!(buf.as_bytes()[0], u8::MAX);
    /// # Ok::<_, pod::buf::CapacityError>(())
    /// ```
    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        // SAFETY: The buffer is guaranteed to be initialized from the
        // `self.read..self.write` range.
        unsafe { slice::from_raw_parts_mut(self.data.as_mut_ptr().cast(), self.len) }
    }
}

/// Debug implementation for `ArrayBuf`.
///
/// # Examples
///
/// ```
/// use pod::ArrayBuf;
///
/// let mut buf = ArrayBuf::<128>::from_slice(&[1u8, 2, 3])?;
/// assert_eq!(format!("{buf:?}"), "[1, 2, 3]");
/// # Ok::<_, pod::buf::CapacityError>(())
/// ```
impl<const N: usize> fmt::Debug for ArrayBuf<N> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_bytes().fmt(f)
    }
}

/// Perform a partial comparison between two arrays.
///
/// # Examples
///
/// ```
/// use pod::ArrayBuf;
///
/// let buf1 = ArrayBuf::<128>::from_slice(&[1u8, 2, 3])?;
/// let buf2 = ArrayBuf::<128>::from_slice(&[1u8, 2, 3, 4])?;
///
/// assert_ne!(buf1, buf2);
/// assert_eq!(buf1, buf1);
/// # Ok::<_, pod::buf::CapacityError>(())
/// ```
impl<const A: usize, const B: usize> PartialEq<ArrayBuf<B>> for ArrayBuf<A> {
    #[inline]
    fn eq(&self, other: &ArrayBuf<B>) -> bool {
        self.as_bytes() == other.as_bytes()
    }
}

/// Perform a partial comparison between two arrays.
///
/// # Examples
///
/// ```
/// use pod::ArrayBuf;
///
/// let array1 = ArrayBuf::<128>::from_slice(&[1u8, 2, 3])?;
/// let slice2: &[u8] = &[1, 2, 3, 4][..];
///
/// assert_ne!(array1, *slice2);
/// assert_eq!(array1, array1);
/// assert_eq!(*slice2, *slice2);
/// # Ok::<_, pod::buf::CapacityError>(())
/// ```
impl<const N: usize> PartialEq<[u8]> for ArrayBuf<N> {
    #[inline]
    fn eq(&self, other: &[u8]) -> bool {
        self.as_bytes() == other
    }
}

/// Perform a partial comparison between a slice and an array.
///
/// # Examples
///
/// ```
/// use pod::ArrayBuf;
///
/// let array1 = ArrayBuf::<128>::from_slice(&[1u8, 2, 3])?;
/// let slice2: &[u8] = &[1, 2, 3, 4][..];
///
/// assert_ne!(array1, *slice2);
/// assert_eq!(array1, array1);
/// assert_eq!(*slice2, *slice2);
/// # Ok::<_, pod::buf::CapacityError>(())
/// ```
impl<const N: usize> PartialEq<[u8; N]> for ArrayBuf<N> {
    #[inline]
    fn eq(&self, other: &[u8; N]) -> bool {
        self.as_bytes() == &other[..]
    }
}

/// Perform a partial comparison between an array and a native array.
///
/// # Examples
///
/// ```
/// use pod::ArrayBuf;
///
/// let slice1 = ArrayBuf::<128>::from_slice(&[1u8, 2, 3])?;
/// let slice2: &[u8] = &[1, 2, 3, 4][..];
///
/// assert_ne!(slice1, *slice2);
/// assert_eq!(slice1, slice1);
/// assert_eq!(*slice2, *slice2);
/// # Ok::<_, pod::buf::CapacityError>(())
/// ```
impl<const N: usize> PartialEq<&[u8; N]> for ArrayBuf<N> {
    #[inline]
    fn eq(&self, other: &&[u8; N]) -> bool {
        self.as_bytes() == &other[..]
    }
}

/// Perform a partial comparison between two arrays.
///
/// # Examples
///
/// ```
/// use pod::ArrayBuf;
///
/// let array1 = ArrayBuf::<128>::from_slice(&[1u8, 2, 3])?;
/// let slice2: &[u8] = &[1, 2, 3, 4][..];
///
/// assert_ne!(array1, slice2);
/// assert_eq!(array1, array1);
/// assert_eq!(slice2, slice2);
/// # Ok::<_, pod::buf::CapacityError>(())
/// ```
impl<const N: usize> PartialEq<&[u8]> for ArrayBuf<N> {
    #[inline]
    fn eq(&self, other: &&[u8]) -> bool {
        self.as_bytes() == *other
    }
}

impl<const N: usize> Eq for ArrayBuf<N> {}

impl<const N: usize> Drop for ArrayBuf<N> {
    fn drop(&mut self) {
        self.clear();
    }
}

#[derive(Clone, Copy)]
pub struct ArrayBufPos {
    at: usize,
    len: usize,
}

impl Pos for ArrayBufPos {
    #[inline]
    fn saturating_add(self, other: usize) -> Self {
        Self {
            at: self.at.saturating_add(other),
            len: self.len.saturating_sub(other),
        }
    }
}

impl<const N: usize> Writer for ArrayBuf<N> {
    type Mut<'this>
        = &'this mut ArrayBuf<N>
    where
        Self: 'this;

    type Pos = ArrayBufPos;

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

        // Ensure we have enough space in the buffer.
        if !(self.len..=N).contains(&len) {
            return Err(Error::new(ErrorKind::CapacityError(CapacityError)));
        }

        // SAFETY: We are writing to a valid position in the buffer.
        unsafe {
            self.data
                .as_mut_ptr()
                .add(self.len)
                .copy_from_nonoverlapping(words.as_ptr().cast(), words_len);
        }

        let pos = ArrayBufPos {
            at: self.len,
            len: words_len,
        };

        self.len = len;
        Ok(pos)
    }

    #[inline]
    fn distance_from(&self, pos: &Self::Pos) -> usize {
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
        let ArrayBufPos { at, len } = pos;

        let words_len = words.len().wrapping_mul(mem::size_of::<T>());

        if len < words.len() {
            return Err(Error::new(ErrorKind::ReservedSizeMismatch {
                expected: len,
                actual: words.len(),
            }));
        }

        if !(at..=N).contains(&(at + words.len())) {
            return Err(Error::new(ErrorKind::CapacityError(CapacityError)));
        }

        // SAFETY: We are writing to a valid position in the buffer.
        unsafe {
            self.data
                .as_mut_ptr()
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
    /// use pod::{ArrayBuf, Writer};
    ///
    /// let mut buf = ArrayBuf::default();
    /// buf.write_bytes(&[1, 2, 3], 3)?;
    /// assert_eq!(buf.as_bytes(), &[1, 2, 3, 0, 0, 0]);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    fn write_bytes(&mut self, bytes: &[u8], pad: usize) -> Result<(), Error> {
        let len = self.len.wrapping_add(bytes.len().wrapping_add(pad));

        if !(self.len..=N).contains(&len) {
            return Err(Error::new(ErrorKind::CapacityError(CapacityError)));
        }

        // SAFETY: We are writing to a valid position in the buffer.
        unsafe {
            let ptr = self.data.as_mut_ptr().add(self.len).cast::<u8>();
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
    /// use pod::{ArrayBuf, Writer};
    ///
    /// let mut buf = ArrayBuf::default();
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

        if !(self.len..=N).contains(&new_len) {
            return Err(Error::new(ErrorKind::CapacityError(CapacityError)));
        }

        // SAFETY: We are writing to a valid position in the buffer.
        unsafe {
            self.data.as_mut_ptr().add(self.len).write_bytes(0, pad);
        }

        self.len = new_len;
        Ok(())
    }

    #[inline]
    fn slice_from(&self, pos: Self::Pos) -> Slice<'_> {
        let ArrayBufPos { at, .. } = pos;
        let ptr = self.data.as_ptr().wrapping_add(at.min(self.len)).cast();
        let len = self.len.saturating_sub(at);
        // SAFETY: We are ensuring that the slice returned is always validly in
        // bounds, even if empty.
        unsafe { Slice::new(slice::from_raw_parts(ptr, len)) }
    }
}

impl<const N: usize> AsSlice for ArrayBuf<N> {
    #[inline]
    fn as_slice(&self) -> Slice<'_> {
        Slice::new(self.as_bytes())
    }
}

impl<const N: usize> SplitReader for ArrayBuf<N> {
    type TakeReader<'this> = Slice<'this>;

    #[inline]
    fn take_reader(&mut self) -> Self::TakeReader<'_> {
        let ptr = self.data.as_ptr().cast::<u8>();
        let len = mem::take(&mut self.len);
        // SAFETY: The buffer is guaranteed to be initialized up to `len`.
        Slice::new(unsafe { slice::from_raw_parts(ptr, len) })
    }
}
