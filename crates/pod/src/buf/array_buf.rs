use core::fmt;
use core::mem::{self, ManuallyDrop, MaybeUninit};
use core::slice;

use crate::error::ErrorKind;
use crate::{AsReader, Error, SplitReader, Writer};

use super::CapacityError;

const DEFAULT_SIZE: usize = 128;

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
/// let mut buf = ArrayBuf::<16>::from_slice(&[1, 2, 3, 4]);
/// assert_eq!(buf.len(), 4);
/// buf.extend_from_words(&[5])?;
/// assert_eq!(buf.as_slice(), &[1, 2, 3, 4, 5]);
/// assert_eq!(buf.len(), 5);
/// # Ok::<_, pod::buf::CapacityError>(())
/// ```
#[repr(C, align(8))]
pub struct ArrayBuf<const N: usize = DEFAULT_SIZE> {
    data: [MaybeUninit<u64>; N],
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
    /// buf.extend_from_words(&[1, 2, 3, 4])?;
    /// assert_eq!(buf.as_slice(), &[1, 2, 3, 4]);
    /// # Ok::<_, pod::buf::CapacityError>(())
    /// ```
    pub fn extend_from_words(&mut self, words: &[u64]) -> Result<(), CapacityError> {
        let write = self.len.wrapping_add(words.len());

        // Ensure we have enough space in the buffer.
        if write > N || write < self.len {
            return Err(CapacityError);
        }

        // SAFETY: We are writing to a valid position in the buffer.
        unsafe {
            self.data
                .as_mut_ptr()
                .add(self.len)
                .copy_from_nonoverlapping(words.as_ptr().cast(), words.len());
        }

        self.len = write;
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
    /// # Examples
    ///
    /// ```
    /// use pod::ArrayBuf;
    ///
    /// let buf = ArrayBuf::<3>::from_array([1, 2, 3]);
    /// assert_eq!(buf.len(), 3);
    /// assert_eq!(buf.capacity(), 3);
    /// assert_eq!(buf.as_slice(), &[1, 2, 3]);
    /// ```
    pub const fn from_array(words: [u64; N]) -> Self {
        let words = ManuallyDrop::new(words);

        // SAFETY: The array is a sequence of initialized elements.
        unsafe {
            Self {
                data: (&words as *const ManuallyDrop<[u64; N]>)
                    .cast::<[MaybeUninit<u64>; N]>()
                    .read(),
                len: N,
            }
        }
    }

    /// Construct from an initialized array.
    ///
    /// # Panics
    ///
    /// Panics if the length of the slice exceeds the buffer size.
    ///
    /// ```should_panic
    /// use pod::ArrayBuf;
    ///
    /// ArrayBuf::<16>::from_slice(&[0; 32]);
    /// ```
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::ArrayBuf;
    ///
    /// let buf = ArrayBuf::<16>::from_slice(&[1, 2, 3]);
    /// assert_eq!(buf.len(), 3);
    /// assert_eq!(buf.capacity(), 16);
    /// assert_eq!(buf.as_slice(), &[1, 2, 3]);
    /// ```
    pub const fn from_slice(words: &[u64]) -> Self {
        assert!(words.len() <= N, "Slice size exceeds buffer size");

        // SAFETY: The array is a sequence of initialized elements.
        unsafe {
            let mut dest: [MaybeUninit<u64>; N] = MaybeUninit::uninit().assume_init();
            let mut write = 0;

            while write < words.len() {
                dest[write] = MaybeUninit::new(words[write]);
                write += 1;
            }

            Self {
                data: dest,
                len: write,
            }
        }
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
    /// buf.extend_from_words(&[42])?;
    /// assert!(!buf.is_empty());
    /// assert_eq!(buf.len(), 1);
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
    /// buf.extend_from_words(&[42])?;
    /// assert!(!buf.is_empty());
    /// assert_eq!(buf.len(), 1);
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
    /// let buf = ArrayBuf::<16>::from_slice(&[1, 2, 3]);
    /// assert_eq!(buf.len(), 3);
    /// assert_eq!(buf.capacity(), 16);
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
    /// let mut buf = ArrayBuf::<3>::from_array([1, 2, 3]);
    /// assert_eq!(buf.len(), 3);
    /// assert_eq!(buf.as_slice(), &[1, 2, 3]);
    /// buf.clear();
    /// assert!(buf.as_slice().is_empty());
    ///
    /// let mut buf = ArrayBuf::from_array([100, 200]);
    /// assert_eq!(buf.len(), 2);
    /// assert_eq!(buf.as_slice(), &[100, 200]);
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
    /// let mut buf = ArrayBuf::default();
    /// assert_eq!(buf.as_bytes().len(), 0);
    /// buf.extend_from_words(&[42])?;
    /// assert_eq!(buf.as_bytes(), &expected[..]);
    /// # Ok::<_, pod::buf::CapacityError>(())
    /// ```
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        // SAFETY: The buffer is guaranteed to be initialized up to `pos`.
        unsafe {
            slice::from_raw_parts(
                self.data.as_ptr().cast(),
                self.len.wrapping_mul(mem::size_of::<u64>()),
            )
        }
    }

    /// Returns the slice of data in the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::ArrayBuf;
    ///
    /// let mut buf = ArrayBuf::default();
    /// assert_eq!(buf.as_slice().len(), 0);
    /// buf.extend_from_words(&[42])?;
    /// assert_eq!(buf.as_slice(), &[42]);
    /// # Ok::<_, pod::buf::CapacityError>(())
    /// ```
    #[inline]
    pub fn as_slice(&self) -> &[u64] {
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
    /// let mut buf = ArrayBuf::default();
    /// assert_eq!(buf.as_slice().len(), 0);
    /// buf.extend_from_words(&[42])?;
    /// assert_eq!(buf.as_slice(), &[42]);
    ///
    /// buf.as_slice_mut()[0] = 43;
    /// assert_eq!(buf.as_slice(), &[43]);
    /// # Ok::<_, pod::buf::CapacityError>(())
    /// ```
    pub fn as_slice_mut(&mut self) -> &mut [u64] {
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
/// let mut buf = ArrayBuf::from_array([1, 2, 3]);
/// assert_eq!(format!("{buf:?}"), "[1, 2, 3]");
/// # Ok::<_, pod::buf::CapacityError>(())
/// ```
impl<const N: usize> fmt::Debug for ArrayBuf<N> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_slice().fmt(f)
    }
}

/// Perform a partial comparison between two arrays.
///
/// # Examples
///
/// ```
/// use pod::ArrayBuf;
///
/// let buf1 = ArrayBuf::from_array([1, 2, 3]);
/// let buf2 = ArrayBuf::from_array([1, 2, 3, 4]);
///
/// assert_ne!(buf1, buf2);
/// assert_eq!(buf1, buf1);
/// ```
impl<const A: usize, const B: usize> PartialEq<ArrayBuf<B>> for ArrayBuf<A> {
    #[inline]
    fn eq(&self, other: &ArrayBuf<B>) -> bool {
        self.as_slice() == other.as_slice()
    }
}

/// Perform a partial comparison between two arrays.
///
/// # Examples
///
/// ```
/// use pod::ArrayBuf;
///
/// let array1 = ArrayBuf::from_array([1, 2, 3]);
/// let slice2: &[u64] = &[1, 2, 3, 4][..];
///
/// assert_ne!(array1, *slice2);
/// assert_eq!(array1, array1);
/// assert_eq!(*slice2, *slice2);
/// ```
impl<U, const N: usize> PartialEq<[U]> for ArrayBuf<N>
where
    u64: PartialEq<U>,
{
    #[inline]
    fn eq(&self, other: &[U]) -> bool {
        self.as_slice() == other
    }
}

/// Perform a partial comparison between a slice and an array.
///
/// # Examples
///
/// ```
/// use pod::ArrayBuf;
///
/// let array1 = ArrayBuf::from_array([1, 2, 3]);
/// let slice2: &[u64] = &[1, 2, 3, 4][..];
///
/// assert_ne!(array1, *slice2);
/// assert_eq!(array1, array1);
/// assert_eq!(*slice2, *slice2);
/// ```
impl<U, const N: usize> PartialEq<[U; N]> for ArrayBuf<N>
where
    u64: PartialEq<U>,
{
    #[inline]
    fn eq(&self, other: &[U; N]) -> bool {
        self.as_slice() == &other[..]
    }
}

/// Perform a partial comparison between an array and a native array.
///
/// # Examples
///
/// ```
/// use pod::ArrayBuf;
///
/// let slice1 = ArrayBuf::from_array([1, 2, 3]);
/// let slice2: &[u64] = &[1, 2, 3, 4][..];
///
/// assert_ne!(slice1, *slice2);
/// assert_eq!(slice1, slice1);
/// assert_eq!(*slice2, *slice2);
/// ```
impl<U, const N: usize> PartialEq<&[U; N]> for ArrayBuf<N>
where
    u64: PartialEq<U>,
{
    #[inline]
    fn eq(&self, other: &&[U; N]) -> bool {
        self.as_slice() == &other[..]
    }
}

/// Perform a partial comparison between two arrays.
///
/// # Examples
///
/// ```
/// use pod::ArrayBuf;
///
/// let array1 = ArrayBuf::from_array([1, 2, 3]);
/// let slice2: &[u64] = &[1, 2, 3, 4][..];
///
/// assert_ne!(array1, slice2);
/// assert_eq!(array1, array1);
/// assert_eq!(slice2, slice2);
/// ```
impl<U, const N: usize> PartialEq<&[U]> for ArrayBuf<N>
where
    u64: PartialEq<U>,
{
    #[inline]
    fn eq(&self, other: &&[U]) -> bool {
        self.as_slice() == *other
    }
}

impl<const N: usize> Eq for ArrayBuf<N> {}

impl<const N: usize> Drop for ArrayBuf<N> {
    fn drop(&mut self) {
        self.clear();
    }
}

#[derive(Clone, Copy)]
pub struct Pos {
    write: usize,
    len: usize,
}

impl<const N: usize> Writer for ArrayBuf<N> {
    type Mut<'this>
        = &'this mut ArrayBuf<N>
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

        // Ensure we have enough space in the buffer.
        if write > N || write < self.len {
            return Err(Error::new(ErrorKind::CapacityError(CapacityError)));
        }

        // SAFETY: We are writing to a valid position in the buffer.
        unsafe {
            self.data
                .as_mut_ptr()
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

        // SAFETY: We are writing to a valid position in the buffer.
        unsafe {
            self.data
                .as_mut_ptr()
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

        let req = full.div_ceil(mem::size_of::<u64>());
        let write = self.len.wrapping_add(req);

        if !(self.len..=N).contains(&write) {
            return Err(Error::new(ErrorKind::CapacityError(CapacityError)));
        }

        // SAFETY: We are writing to a valid position in the buffer.
        unsafe {
            let ptr = self.data.as_mut_ptr().add(self.len).cast::<u8>();
            ptr.copy_from_nonoverlapping(bytes.as_ptr(), bytes.len());
            let pad = mem::size_of::<u64>() - bytes.len() % mem::size_of::<u64>();
            ptr.add(bytes.len()).write_bytes(0, pad);
        }

        self.len = write;
        Ok(())
    }
}

impl<const N: usize> AsReader for ArrayBuf<N> {
    type AsReader<'this> = &'this [u64];

    #[inline]
    fn as_reader(&self) -> Self::AsReader<'_> {
        self.as_slice()
    }
}

impl<const N: usize> SplitReader for ArrayBuf<N> {
    type TakeReader<'this> = &'this [u64];

    #[inline]
    fn take_reader(&mut self) -> Self::TakeReader<'_> {
        let ptr = self.data.as_ptr().cast::<u64>();
        let len = self.len;
        self.len = 0;
        // SAFETY: The buffer is guaranteed to be initialized up to `len`.
        unsafe { slice::from_raw_parts(ptr, len) }
    }
}
