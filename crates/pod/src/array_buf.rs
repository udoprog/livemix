use core::fmt;
use core::mem::MaybeUninit;
use core::slice;

use crate::error::ErrorKind;
use crate::{DWORD_SIZE, Error, Reader, Slice, Visitor, WORD_SIZE, Writer};

const DEFAULT_SIZE: usize = 1024;

/// A fixed-size buffer with a flexible read and write position.
///
/// The initialized slice of the buffer is defined by the region betweeen the
/// `read` and `write` positions.
///
/// # Examples
///
/// ```
/// use pod::{ArrayBuf, Reader, Writer};
///
/// let mut buf = ArrayBuf::<16>::from_slice(&[1, 2, 3, 4]);
/// assert_eq!(buf.read(), 4);
/// buf.write_u32(5)?;
/// assert_eq!(buf.as_slice(), &[1, 2, 3, 4, 5]);
/// assert_eq!(buf.read(), 5);
/// assert_eq!(buf.array()?, [1]);
/// assert_eq!(buf.as_slice(), &[2, 3, 4, 5]);
/// assert_eq!(buf.read_u64()?, 2u64 + (3u64 << 32));
/// assert_eq!(buf.read(), 2);
/// # Ok::<_, pod::Error>(())
/// ```
#[repr(C, align(8))]
pub struct ArrayBuf<const N: usize = DEFAULT_SIZE> {
    data: [MaybeUninit<u32>; N],
    read: usize,
    write: usize,
}

impl ArrayBuf {
    /// Construct a new array buffer with a default size.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::ArrayBuf;
    ///
    /// let buf = ArrayBuf::new();
    /// ```
    #[inline]
    pub const fn new() -> Self {
        // SAFETY: The buffer is a sequence of uninitialized elements.
        Self {
            data: unsafe { MaybeUninit::uninit().assume_init() },
            read: 0,
            write: 0,
        }
    }
}

impl Default for ArrayBuf {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> ArrayBuf<N> {
    /// Construct a new array buffer with a default size.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::ArrayBuf;
    ///
    /// let buf = ArrayBuf::<16>::with_size();
    /// ```
    #[inline]
    pub const fn with_size() -> Self {
        // SAFETY: The buffer is a sequence of uninitialized elements.
        Self {
            data: unsafe { MaybeUninit::uninit().assume_init() },
            read: 0,
            write: 0,
        }
    }

    /// Construct from an initialized array.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::ArrayBuf;
    ///
    /// let buf = ArrayBuf::from_array([1, 2, 3]);
    /// assert_eq!(buf.read(), 3);
    /// assert_eq!(buf.as_slice(), &[1, 2, 3]);
    /// ```
    pub const fn from_array(words: [u32; N]) -> Self {
        // SAFETY: The array is a sequence of initialized elements.
        unsafe {
            Self {
                data: (&words as *const [u32; N])
                    .cast::<[MaybeUninit<u32>; N]>()
                    .read(),
                read: 0,
                write: N,
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
    /// assert_eq!(buf.read(), 3);
    /// assert_eq!(buf.as_slice(), &[1, 2, 3]);
    /// ```
    pub const fn from_slice(words: &[u32]) -> Self {
        assert!(words.len() <= N, "Array size exceeds buffer size");

        // SAFETY: The array is a sequence of initialized elements.
        unsafe {
            let mut dest: [MaybeUninit<u32>; N] = MaybeUninit::uninit().assume_init();
            let mut write = 0;

            while write < words.len() {
                dest[write] = MaybeUninit::new(words[write]);
                write += 1;
            }

            Self {
                data: dest,
                read: 0,
                write,
            }
        }
    }

    /// Returns the number of 32-bit words that can be read.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Reader};
    ///
    /// let mut array = ArrayBuf::from_array([1, 2, 3]);
    /// assert_eq!(array.read(), 3);
    ///
    /// assert_eq!(array.array()?, [1]);
    /// assert_eq!(array.read(), 2);
    /// assert_eq!(array.as_slice(), &[2, 3]);
    ///
    /// assert_eq!(array.read_u64()?, 2u64 + (3u64 << 32));
    /// assert_eq!(array.read(), 0);
    /// # Ok::<_, pod::Error>(())
    /// ```
    pub const fn read(&self) -> usize {
        self.write - self.read
    }

    /// Returns the number of 32-bit words that can be written.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Reader};
    ///
    /// let mut array = ArrayBuf::<16>::from_slice(&[1, 2, 3]);
    /// assert_eq!(array.read(), 3);
    /// assert_eq!(array.write(), 13);
    ///
    /// assert_eq!(array.array::<1>()?, [1]);
    /// assert_eq!(array.read(), 2);
    /// assert_eq!(array.as_slice(), &[2, 3]);
    ///
    /// assert_eq!(array.read_u64()?, 2u64 + (3u64 << 32));
    /// assert_eq!(array.read(), 0);
    /// # Ok::<_, pod::Error>(())
    /// ```
    pub const fn write(&self) -> usize {
        N - self.write
    }

    /// Resets the buffer to an empty state.
    ///
    /// This clears the content of the buffer that can be read, treating any
    /// previously written data as uninitialized.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Reader, Writer};
    ///
    /// let mut buf = ArrayBuf::from_array([1, 2, 3]);
    ///
    /// assert_eq!(buf.read(), 3);
    ///
    /// assert_eq!(buf.as_slice(), &[1, 2, 3]);
    /// assert_eq!(buf.array()?, [1]);
    /// assert_eq!(buf.as_slice(), &[2, 3]);
    /// buf.clear_read();
    /// assert_eq!(buf.as_slice(), &[1, 2, 3]);
    /// assert_eq!(buf.array()?, [1]);
    /// buf.clear();
    /// assert_eq!(buf.as_slice(), &[]);
    /// assert_eq!(buf.write(), 3);
    /// # Ok::<_, pod::Error>(())
    #[inline]
    pub fn clear(&mut self) {
        self.read = 0;
        self.write = 0;
    }

    /// Resets the buffer for reading.
    ///
    /// This clears the read position, allowing the buffer to be read from the
    /// start again.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Reader, Writer};
    ///
    /// let mut buf = ArrayBuf::new();
    /// buf.write_u32(42)?;
    ///
    /// assert_eq!(buf.as_slice(), &[42]);
    /// assert_eq!(buf.array()?, [42]);
    /// assert_eq!(buf.as_slice(), &[]);
    /// buf.clear_read();
    ///
    /// assert_eq!(buf.as_slice(), &[42]);
    /// assert_eq!(buf.array()?, [42]);
    /// # Ok::<_, pod::Error>(())
    #[inline]
    pub fn clear_read(&mut self) {
        self.read = 0;
    }

    /// Returns the slice of remaining data to be read.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Writer};
    ///
    /// let mut buf = ArrayBuf::new();
    /// assert_eq!(buf.as_slice().len(), 0);
    ///
    /// buf.write_u32(42)?;
    /// assert_eq!(buf.as_slice(), &[42]);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn as_slice(&self) -> &[u32] {
        // SAFETY: The buffer is guaranteed to be initialized up to `pos`.
        unsafe { slice::from_raw_parts(self.data.as_ptr().add(self.read).cast(), self.read()) }
    }

    /// Returns the initialized slice wrapped as a [`Slice`].
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Writer};
    ///
    /// let mut buf = ArrayBuf::new();
    /// assert_eq!(buf.as_slice().len(), 0);
    ///
    /// buf.write_u32(42)?;
    /// assert_eq!(buf.as_slice(), &[42u32]);
    /// assert_eq!(buf.as_slice(), &[42u32][..]);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn as_reader_slice(&self) -> &Slice {
        Slice::new(self.as_slice())
    }
}

/// Debug implementation for `Buf`.
///
/// # Examples
///
/// ```
/// use pod::{ArrayBuf, Reader};
///
/// let mut buf = ArrayBuf::from_array([1, 2, 3]);
/// assert_eq!(format!("{buf:?}"), "[1, 2, 3]");
/// buf.array::<1>()?;
/// assert_eq!(format!("{buf:?}"), "[2, 3]");
///
/// # Ok::<_, pod::Error>(())
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
/// let slice2: &[u32] = &[1, 2, 3, 4][..];
///
/// assert_ne!(array1, *slice2);
/// assert_eq!(array1, array1);
/// assert_eq!(*slice2, *slice2);
/// ```
impl<const N: usize> PartialEq<[u32]> for ArrayBuf<N> {
    #[inline]
    fn eq(&self, other: &[u32]) -> bool {
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
/// let slice2: &[u32] = &[1, 2, 3, 4][..];
///
/// assert_ne!(array1, *slice2);
/// assert_eq!(array1, array1);
/// assert_eq!(*slice2, *slice2);
/// ```
impl<const N: usize> PartialEq<[u32; N]> for ArrayBuf<N> {
    #[inline]
    fn eq(&self, other: &[u32; N]) -> bool {
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
/// let slice2: &[u32] = &[1, 2, 3, 4][..];
///
/// assert_ne!(slice1, *slice2);
/// assert_eq!(slice1, slice1);
/// assert_eq!(*slice2, *slice2);
/// ```
impl<const N: usize> PartialEq<&[u32; N]> for ArrayBuf<N> {
    #[inline]
    fn eq(&self, other: &&[u32; N]) -> bool {
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
/// let slice2: &[u32] = &[1, 2, 3, 4][..];
///
/// assert_ne!(array1, slice2);
/// assert_eq!(array1, array1);
/// assert_eq!(slice2, slice2);
/// ```
impl<const N: usize> PartialEq<&[u32]> for ArrayBuf<N> {
    #[inline]
    fn eq(&self, other: &&[u32]) -> bool {
        self.as_slice() == *other
    }
}

impl<const N: usize> Eq for ArrayBuf<N> {}

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
    fn reserve_words(&mut self, words: &[u32]) -> Result<Self::Pos, Error> {
        let write = self.write.wrapping_add(words.len());

        // Ensure we have enough space in the buffer.
        if write > N || write < self.write {
            return Err(Error::new(ErrorKind::BufferOverflow));
        }

        // SAFETY: We are writing to a valid position in the buffer.
        unsafe {
            self.data
                .as_mut_ptr()
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
        (self.write - pos.write) * WORD_SIZE
    }

    #[inline]
    fn write_zeros(&mut self, words: usize) -> Result<(), Error> {
        let write = self.write.wrapping_add(words);

        // Ensure we have enough space in the buffer.
        if write > N || write < self.write {
            return Err(Error::new(ErrorKind::BufferOverflow));
        }

        // SAFETY: We are writing to valid positions in the buffer.
        unsafe {
            self.data.as_mut_ptr().add(self.write).write_bytes(0, words);
        }

        self.write = write;
        Ok(())
    }

    #[inline]
    fn write_words(&mut self, words: &[u32]) -> Result<(), Error> {
        let write = self.write.wrapping_add(words.len());

        // Ensure we have enough space in the buffer.
        if write > N || write < self.write {
            return Err(Error::new(ErrorKind::BufferOverflow));
        }

        // SAFETY: We are writing to a valid position in the buffer.
        unsafe {
            self.data
                .as_mut_ptr()
                .add(self.write)
                .copy_from_nonoverlapping(words.as_ptr().cast(), words.len());
        }

        self.write = write;
        Ok(())
    }

    #[inline]
    fn write_words_at(&mut self, pos: Self::Pos, words: &[u32]) -> Result<(), Error> {
        let Pos { write, len } = pos;

        if len < words.len() {
            return Err(Error::new(ErrorKind::PositionSizeMismatch {
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

        let req = full.div_ceil(WORD_SIZE).next_multiple_of(2);
        let write = self.write.wrapping_add(req);

        if !(self.write..=N).contains(&write) {
            return Err(Error::new(ErrorKind::BufferOverflow));
        }

        // SAFETY: We are writing to a valid position in the buffer.
        unsafe {
            let ptr = self.data.as_mut_ptr().add(self.write).cast::<u8>();
            ptr.copy_from_nonoverlapping(bytes.as_ptr(), bytes.len());
            let pad = DWORD_SIZE - bytes.len() % DWORD_SIZE;
            ptr.add(bytes.len()).write_bytes(0, pad);
        }

        self.write = write;
        Ok(())
    }
}

impl<'de, const N: usize> Reader<'de> for ArrayBuf<N> {
    type Mut<'this>
        = &'this mut ArrayBuf<N>
    where
        Self: 'this;

    #[inline]
    fn borrow_mut(&mut self) -> Self::Mut<'_> {
        self
    }

    #[inline]
    fn peek_words_uninit(&self, out: &mut [MaybeUninit<u32>]) -> Result<(), Error> {
        if self.read() < out.len() {
            return Err(Error::new(ErrorKind::BufferUnderflow));
        }

        // SAFETY: The start pointer is valid since it hasn't reached the end yet.
        unsafe {
            self.data
                .as_ptr()
                .add(self.read)
                .cast::<MaybeUninit<u32>>()
                .copy_to_nonoverlapping(out.as_mut_ptr(), out.len());
        }

        Ok(())
    }

    #[inline]
    fn read_words_uninit(&mut self, out: &mut [MaybeUninit<u32>]) -> Result<(), Error> {
        let read = self.read.wrapping_add(out.len());

        if read > self.write || read < self.read {
            return Err(Error::new(ErrorKind::BufferUnderflow));
        }

        // SAFETY: The start pointer is valid since it hasn't reached the end yet.
        unsafe {
            self.data
                .as_ptr()
                .add(self.read)
                .cast::<MaybeUninit<u32>>()
                .copy_to_nonoverlapping(out.as_mut_ptr(), out.len());
        }

        self.read = read;
        Ok(())
    }

    #[inline]
    fn read_bytes<V>(&mut self, len: usize, visitor: V) -> Result<V::Ok, Error>
    where
        V: Visitor<'de, [u8]>,
    {
        let req = len.div_ceil(WORD_SIZE).next_multiple_of(2);
        let read = self.read.wrapping_add(req);

        if read > self.write || read < self.read {
            return Err(Error::new(ErrorKind::BufferUnderflow));
        }

        unsafe {
            let ptr = self.data.as_ptr().add(read).cast::<u8>();
            let bytes = slice::from_raw_parts(ptr, len);
            let ok = visitor.visit_ref(bytes)?;
            self.read = read;
            Ok(ok)
        }
    }
}
