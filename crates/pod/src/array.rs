use core::fmt;
use core::mem::{self, ManuallyDrop, MaybeUninit};
use core::ptr;
use core::slice;

use crate::error::ErrorKind;
use crate::utils::BytesInhabited;
use crate::{AsReader, Error, Writer};

const DEFAULT_SIZE: usize = 128;

/// A fixed-size buffer with a flexible read and write position.
///
/// The initialized slice of the buffer is defined by the region betweeen the
/// `read` and `write` positions.
///
/// # Examples
///
/// ```
/// use pod::Buf;
///
/// let mut buf = Buf::<u32, 16>::from_slice(&[1, 2, 3, 4]);
/// assert_eq!(buf.len(), 4);
/// buf.push(5)?;
/// assert_eq!(buf.as_slice(), &[1, 2, 3, 4, 5]);
/// assert_eq!(buf.pop(), Some(5));
/// assert_eq!(buf.len(), 4);
/// # Ok::<_, pod::Error>(())
/// ```
///
/// Trying to read data from the array in a manner which is *not* correctly
/// aligned will errors:
///
/// ```compile_fail
/// use pod::{Buf, Reader};
///
/// let mut buf = Buf::<u64, 16>::from_slice(&[1, 2, 3, 4]);
/// // This must fail because it's not possible to read half of a word out of the array.
/// buf.read::<u32>()?;
/// # Ok::<_, pod::Error>(())
/// ```
#[repr(C, align(8))]
pub struct Buf<T = u64, const N: usize = DEFAULT_SIZE> {
    data: [MaybeUninit<T>; N],
    len: usize,
}

impl<T, const N: usize> Buf<T, N> {
    /// Construct a new array buffer with a default size.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::Buf;
    ///
    /// let buf = Buf::<u64>::new();
    /// ```
    #[inline]
    pub const fn new() -> Self {
        // SAFETY: The buffer is a sequence of uninitialized elements.
        Self {
            data: unsafe { MaybeUninit::uninit().assume_init() },
            len: 0,
        }
    }

    /// Push a value into the array.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::Buf;
    ///
    /// let mut buf = Buf::<String>::new();
    /// buf.push("Hello".to_string())?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    pub fn push(&mut self, value: T) -> Result<(), Error> {
        if self.len >= N {
            return Err(Error::new(ErrorKind::BufferOverflow));
        }

        // SAFETY: We are writing to a valid position in the buffer.
        unsafe {
            self.data
                .as_mut_ptr()
                .add(self.len)
                .cast::<T>()
                .write(value);
        }

        self.len += 1;
        Ok(())
    }

    /// Push a value from the array.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::Buf;
    ///
    /// let mut buf = Buf::<String>::new();
    /// buf.push(String::from("Hello"))?;
    /// buf.push(String::from("World"))?;
    ///
    /// assert_eq!(buf.pop(), Some(String::from("World")));
    /// assert_eq!(buf.pop(), Some(String::from("Hello")));
    /// assert_eq!(buf.pop(), None);
    /// # Ok::<_, pod::Error>(())
    /// ```
    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }

        self.len -= 1;

        // SAFETY: We are writing to a valid position in the buffer.
        let value = unsafe { self.data.as_mut_ptr().add(self.len).cast::<T>().read() };

        Some(value)
    }
}

impl<T> Default for Buf<T> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const N: usize> Buf<T, N> {
    /// Construct from an initialized array.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::Buf;
    ///
    /// let buf = Buf::<u64, 3>::from_array([1, 2, 3]);
    /// assert_eq!(buf.len(), 3);
    /// assert_eq!(buf.capacity(), 3);
    /// assert_eq!(buf.as_slice(), &[1, 2, 3]);
    /// ```
    pub const fn from_array(words: [T; N]) -> Self {
        let words = ManuallyDrop::new(words);

        // SAFETY: The array is a sequence of initialized elements.
        unsafe {
            Self {
                data: (&words as *const ManuallyDrop<[T; N]>)
                    .cast::<[MaybeUninit<T>; N]>()
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
    /// use pod::Buf;
    ///
    /// Buf::<u64, 16>::from_slice(&[0; 32]);
    /// ```
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::Buf;
    ///
    /// let buf = Buf::<u64, 16>::from_slice(&[1, 2, 3]);
    /// assert_eq!(buf.len(), 3);
    /// assert_eq!(buf.capacity(), 16);
    /// assert_eq!(buf.as_slice(), &[1, 2, 3]);
    /// ```
    pub const fn from_slice(words: &[T]) -> Self
    where
        T: BytesInhabited,
    {
        assert!(words.len() <= N, "Slice size exceeds buffer size");

        // SAFETY: The array is a sequence of initialized elements.
        unsafe {
            let mut dest: [MaybeUninit<T>; N] = MaybeUninit::uninit().assume_init();
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
    /// use pod::Buf;
    ///
    /// let mut buf = Buf::<u64>::new();
    /// assert!(buf.is_empty());
    /// assert_eq!(buf.len(), 0);
    /// buf.push(42)?;
    /// assert!(!buf.is_empty());
    /// assert_eq!(buf.len(), 1);
    /// # Ok::<_, pod::Error>(())
    /// ```
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Test if the buffer is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::Buf;
    ///
    /// let mut buf = Buf::<u64>::new();
    /// assert!(buf.is_empty());
    /// assert_eq!(buf.len(), 0);
    /// buf.push(42)?;
    /// assert!(!buf.is_empty());
    /// assert_eq!(buf.len(), 1);
    /// # Ok::<_, pod::Error>(())
    /// ```
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns the capacity of the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::Buf;
    ///
    /// let buf = Buf::<u32, 16>::from_slice(&[1, 2, 3]);
    /// assert_eq!(buf.len(), 3);
    /// assert_eq!(buf.capacity(), 16);
    /// # Ok::<_, pod::Error>(())
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
    /// use pod::{Buf, Reader};
    ///
    /// let mut buf = Buf::<u64, 3>::from_array([1, 2, 3]);
    /// assert_eq!(buf.len(), 3);
    /// assert_eq!(buf.as_slice(), &[1, 2, 3]);
    /// buf.clear();
    /// assert!(buf.as_slice().is_empty());
    ///
    /// let mut buf = Buf::<String, 2>::from_array([String::from("Hello"), String::from("World")]);
    /// assert_eq!(buf.len(), 2);
    /// assert_eq!(buf.as_slice(), &[String::from("Hello"), String::from("World")]);
    /// buf.clear();
    /// assert!(buf.as_slice().is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn clear(&mut self) {
        let len = mem::take(&mut self.len);

        if mem::needs_drop::<T>() {
            // SAFETY: The buffer is guaranteed to be initialized from the
            // `self.read..self.write` range.
            unsafe {
                let slice = slice::from_raw_parts_mut(self.data.as_mut_ptr().cast::<T>(), len);

                ptr::drop_in_place(slice);
            }
        }
    }

    /// Returns the bytes of the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Buf, Writer};
    ///
    /// let mut buf = Buf::<u64>::new();
    /// assert_eq!(buf.as_bytes().len(), 0);
    ///
    /// buf.write(42u64)?;
    /// let expected = 42u64.to_ne_bytes();
    /// assert_eq!(buf.as_bytes(), &expected[..]);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        // SAFETY: The buffer is guaranteed to be initialized up to `pos`.
        unsafe {
            slice::from_raw_parts(
                self.data.as_ptr().cast(),
                self.len.wrapping_mul(mem::size_of::<T>()),
            )
        }
    }

    /// Returns the slice of data in the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Buf, Writer};
    ///
    /// let mut buf = Buf::<u64>::new();
    /// assert_eq!(buf.as_slice().len(), 0);
    ///
    /// buf.write(42u64)?;
    /// assert_eq!(buf.as_slice(), &[42]);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn as_slice(&self) -> &[T] {
        // SAFETY: The buffer is guaranteed to be initialized up to `pos`.
        unsafe { slice::from_raw_parts(self.data.as_ptr().cast(), self.len) }
    }

    /// Returns a mutable slice of data in the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Buf, Writer};
    ///
    /// let mut buf = Buf::<u64>::new();
    /// assert_eq!(buf.as_slice().len(), 0);
    ///
    /// buf.write(42u64)?;
    /// assert_eq!(buf.as_slice(), &[42]);
    ///
    /// buf.as_slice_mut()[0] = 43;
    /// assert_eq!(buf.as_slice(), &[43]);
    /// # Ok::<_, pod::Error>(())
    /// ```
    pub fn as_slice_mut(&mut self) -> &mut [T] {
        // SAFETY: The buffer is guaranteed to be initialized from the
        // `self.read..self.write` range.
        unsafe { slice::from_raw_parts_mut(self.data.as_mut_ptr().cast(), self.len) }
    }
}

/// Debug implementation for `Buf`.
///
/// # Examples
///
/// ```
/// use pod::{Buf, Reader};
///
/// let mut buf = Buf::from_array([1, 2, 3]);
/// assert_eq!(format!("{buf:?}"), "[1, 2, 3]");
/// buf.pop();
/// assert_eq!(format!("{buf:?}"), "[1, 2]");
/// # Ok::<_, pod::Error>(())
/// ```
impl<T, const N: usize> fmt::Debug for Buf<T, N>
where
    T: fmt::Debug,
{
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
/// use pod::Buf;
///
/// let buf1 = Buf::from_array([1, 2, 3]);
/// let buf2 = Buf::from_array([1, 2, 3, 4]);
///
/// assert_ne!(buf1, buf2);
/// assert_eq!(buf1, buf1);
/// ```
impl<T, U, const A: usize, const B: usize> PartialEq<Buf<U, B>> for Buf<T, A>
where
    T: PartialEq<U>,
{
    #[inline]
    fn eq(&self, other: &Buf<U, B>) -> bool {
        self.as_slice() == other.as_slice()
    }
}

/// Perform a partial comparison between two arrays.
///
/// # Examples
///
/// ```
/// use pod::Buf;
///
/// let array1 = Buf::from_array([1, 2, 3]);
/// let slice2: &[u64] = &[1, 2, 3, 4][..];
///
/// assert_ne!(array1, *slice2);
/// assert_eq!(array1, array1);
/// assert_eq!(*slice2, *slice2);
/// ```
impl<T, U, const N: usize> PartialEq<[U]> for Buf<T, N>
where
    T: PartialEq<U>,
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
/// use pod::Buf;
///
/// let array1 = Buf::from_array([1, 2, 3]);
/// let slice2: &[u64] = &[1, 2, 3, 4][..];
///
/// assert_ne!(array1, *slice2);
/// assert_eq!(array1, array1);
/// assert_eq!(*slice2, *slice2);
/// ```
impl<T, U, const N: usize> PartialEq<[U; N]> for Buf<T, N>
where
    T: PartialEq<U>,
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
/// use pod::Buf;
///
/// let slice1 = Buf::from_array([1, 2, 3]);
/// let slice2: &[u64] = &[1, 2, 3, 4][..];
///
/// assert_ne!(slice1, *slice2);
/// assert_eq!(slice1, slice1);
/// assert_eq!(*slice2, *slice2);
/// ```
impl<T, U, const N: usize> PartialEq<&[U; N]> for Buf<T, N>
where
    T: PartialEq<U>,
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
/// use pod::Buf;
///
/// let array1 = Buf::from_array([1, 2, 3]);
/// let slice2: &[u64] = &[1, 2, 3, 4][..];
///
/// assert_ne!(array1, slice2);
/// assert_eq!(array1, array1);
/// assert_eq!(slice2, slice2);
/// ```
impl<T, U, const N: usize> PartialEq<&[U]> for Buf<T, N>
where
    T: PartialEq<U>,
{
    #[inline]
    fn eq(&self, other: &&[U]) -> bool {
        self.as_slice() == *other
    }
}

impl<T, const N: usize> Eq for Buf<T, N> where T: Eq {}

impl<T, const N: usize> Drop for Buf<T, N> {
    fn drop(&mut self) {
        self.clear();
    }
}

#[derive(Clone, Copy)]
pub struct Pos {
    write: usize,
    len: usize,
}

impl<T, const N: usize> Writer<T> for Buf<T, N>
where
    T: BytesInhabited,
{
    type Mut<'this>
        = &'this mut Buf<T, N>
    where
        Self: 'this;

    type Pos = Pos;

    #[inline]
    fn borrow_mut(&mut self) -> Self::Mut<'_> {
        self
    }

    #[inline]
    fn reserve_words(&mut self, words: &[T]) -> Result<Self::Pos, Error> {
        let write = self.len.wrapping_add(words.len());

        // Ensure we have enough space in the buffer.
        if write > N || write < self.len {
            return Err(Error::new(ErrorKind::BufferOverflow));
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
            .wrapping_mul(mem::size_of::<T>())
    }

    #[inline]
    fn write_words(&mut self, words: &[T]) -> Result<(), Error> {
        let write = self.len.wrapping_add(words.len());

        // Ensure we have enough space in the buffer.
        if write > N || write < self.len {
            return Err(Error::new(ErrorKind::BufferOverflow));
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

    #[inline]
    fn write_words_at(&mut self, pos: Self::Pos, words: &[T]) -> Result<(), Error> {
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
    fn write_bytes(&mut self, bytes: &[u8], pad: usize) -> Result<(), Error>
    where
        T: BytesInhabited,
    {
        let Some(full) = bytes.len().checked_add(pad) else {
            return Err(Error::new(ErrorKind::SizeOverflow));
        };

        let req = full.div_ceil(mem::size_of::<T>());
        let write = self.len.wrapping_add(req);

        if !(self.len..=N).contains(&write) {
            return Err(Error::new(ErrorKind::BufferOverflow));
        }

        // SAFETY: We are writing to a valid position in the buffer.
        unsafe {
            let ptr = self.data.as_mut_ptr().add(self.len).cast::<u8>();
            ptr.copy_from_nonoverlapping(bytes.as_ptr(), bytes.len());
            let pad = mem::size_of::<T>() - bytes.len() % mem::size_of::<T>();
            ptr.add(bytes.len()).write_bytes(0, pad);
        }

        self.len = write;
        Ok(())
    }
}

impl<T, const N: usize> AsReader<T> for Buf<T, N>
where
    T: 'static,
{
    type AsReader<'this> = &'this [T];

    #[inline]
    fn as_reader(&self) -> Self::AsReader<'_> {
        self.as_slice()
    }
}
