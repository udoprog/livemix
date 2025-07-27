use core::fmt;
use core::mem::{self, ManuallyDrop, MaybeUninit};
use core::ptr;
use core::slice;

use crate::error::ErrorKind;
use crate::utils::BytesInhabited;
use crate::{Error, Reader, Visitor, Writer};

const DEFAULT_SIZE: usize = 128;

/// A fixed-size buffer with a flexible read and write position.
///
/// The initialized slice of the buffer is defined by the region betweeen the
/// `read` and `write` positions.
///
/// # Examples
///
/// ```
/// use pod::{Array, Reader, Writer};
///
/// let mut buf = Array::<u32, 16>::from_slice(&[1, 2, 3, 4]);
/// assert_eq!(buf.remaining(), 4);
/// buf.write(5u32)?;
/// assert_eq!(buf.as_slice(), &[1, 2, 3, 4, 5]);
/// assert_eq!(buf.remaining(), 5);
/// assert_eq!(buf.read::<[u32; 1]>()?, [1]);
/// assert_eq!(buf.as_slice(), &[2, 3, 4, 5]);
/// assert_eq!(buf.read::<u64>()?, 2u64 + (3u64 << 32));
/// assert_eq!(buf.remaining(), 2);
/// # Ok::<_, pod::Error>(())
/// ```
///
/// Trying to read data from the array in a manner which is *not* correctly
/// aligned will errors:
///
/// ```compile_fail
/// use pod::{Array, Reader};
///
/// let mut buf = Array::<u64, 16>::from_slice(&[1, 2, 3, 4]);
/// // This must fail because it's not possible to read half of a word out of the array.
/// buf.read::<u32>()?;
/// # Ok::<_, pod::Error>(())
/// ```
#[repr(C, align(8))]
pub struct Array<T = u64, const N: usize = DEFAULT_SIZE> {
    data: [MaybeUninit<T>; N],
    read: usize,
    write: usize,
}

impl<T, const N: usize> Array<T, N> {
    /// Construct a new array buffer with a default size.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::Array;
    ///
    /// let buf = Array::<u64>::new();
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

    /// Push a value into the array.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::Array;
    ///
    /// let mut buf = Array::<String>::new();
    /// buf.push("Hello".to_string())?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    pub fn push(&mut self, value: T) -> Result<(), Error> {
        if self.write >= N {
            return Err(Error::new(ErrorKind::BufferOverflow));
        }

        // SAFETY: We are writing to a valid position in the buffer.
        unsafe {
            self.data
                .as_mut_ptr()
                .add(self.write)
                .cast::<T>()
                .write(value);
        }

        self.write += 1;
        Ok(())
    }

    /// Pop the next value to read from the array.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::Array;
    ///
    /// let mut buf = Array::<String>::new();
    /// buf.push("Hello".to_string())?;
    /// buf.push("World".to_string())?;
    ///
    /// assert_eq!(buf.pop_front(), Some("Hello".to_string()));
    /// assert_eq!(buf.pop_front(), Some("World".to_string()));
    /// assert!(buf.is_empty());
    /// assert_eq!(buf.pop_front(), None);
    /// # Ok::<_, pod::Error>(())
    /// ```
    pub fn pop_front(&mut self) -> Option<T> {
        if self.read == self.write {
            return None;
        }

        // SAFETY: The buffer is initialized in the `self.read..self.write`
        // range.
        unsafe {
            let value = self.data.as_ptr().add(self.read).cast::<T>().read();
            self.read += 1;

            if self.read == self.write {
                self.read = 0;
                self.write = 0;
            }

            Some(value)
        }
    }
}

impl<T> Default for Array<T> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const N: usize> Array<T, N> {
    /// Construct from an initialized array.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::Array;
    ///
    /// let buf = Array::<u64, 3>::from_array([1, 2, 3]);
    /// assert_eq!(buf.remaining(), 3);
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
    /// use pod::Array;
    ///
    /// Array::<u64, 16>::from_slice(&[0; 32]);
    /// ```
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::Array;
    ///
    /// let buf = Array::<u64, 16>::from_slice(&[1, 2, 3]);
    /// assert_eq!(buf.remaining(), 3);
    /// assert_eq!(buf.as_slice(), &[1, 2, 3]);
    /// ```
    pub const fn from_slice(words: &[T]) -> Self
    where
        T: Copy,
    {
        assert!(words.len() <= N, "Array size exceeds buffer size");

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
                read: 0,
                write,
            }
        }
    }

    /// Returns if the array is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::Array;
    ///
    /// let mut buf = Array::<u64>::new();
    /// assert!(buf.is_empty());
    /// buf.push(42)?;
    /// assert!(!buf.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    pub const fn is_empty(&self) -> bool {
        self.read == self.write
    }

    /// Returns the number of words that can be read.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Array, Reader};
    ///
    /// let mut array = Array::from_array([1u32, 2, 3]);
    /// assert_eq!(array.remaining(), 3);
    ///
    /// assert_eq!(array.read::<[u32; 1]>()?, [1]);
    /// assert_eq!(array.remaining(), 2);
    /// assert_eq!(array.remaining_bytes(), 8);
    /// assert_eq!(array.as_slice(), &[2, 3]);
    ///
    /// assert_eq!(array.read::<u64>()?, 2u64 + (3u64 << 32));
    /// assert_eq!(array.remaining(), 0);
    /// # Ok::<_, pod::Error>(())
    /// ```
    pub const fn remaining(&self) -> usize {
        self.write - self.read
    }

    /// Returns the size of the remaining buffer in bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Array, Reader};
    ///
    /// let mut array = Array::from_array([1u32, 2, 3]);
    /// assert_eq!(array.remaining(), 3);
    /// assert_eq!(array.remaining_bytes(), 12);
    ///
    /// assert_eq!(array.read::<[u32; 1]>()?, [1]);
    /// assert_eq!(array.remaining(), 2);
    /// assert_eq!(array.remaining_bytes(), 8);
    /// assert_eq!(array.as_slice(), &[2, 3]);
    ///
    /// assert_eq!(array.read::<u64>()?, 2u64 + (3u64 << 32));
    /// assert_eq!(array.remaining(), 0);
    /// assert_eq!(array.remaining_bytes(), 0);
    /// # Ok::<_, pod::Error>(())
    /// ```
    pub fn remaining_bytes(&self) -> usize {
        self.remaining().wrapping_mul(mem::size_of::<T>())
    }

    /// Returns the number of words that can be written.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Array, Reader};
    ///
    /// let mut array = Array::<u32, 16>::from_slice(&[1, 2, 3]);
    /// assert_eq!(array.remaining(), 3);
    /// assert_eq!(array.remaining_mut(), 13);
    ///
    /// assert_eq!(array.read::<[u32; 1]>()?, [1]);
    /// assert_eq!(array.remaining(), 2);
    /// assert_eq!(array.as_slice(), &[2, 3]);
    ///
    /// assert_eq!(array.read::<u64>()?, 2u64 + (3u64 << 32));
    /// assert_eq!(array.remaining(), 0);
    /// # Ok::<_, pod::Error>(())
    /// ```
    pub const fn remaining_mut(&self) -> usize {
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
    /// use pod::{Array, Reader, Writer};
    ///
    /// let mut buf = Array::<u64, 3>::from_array([1, 2, 3]);
    ///
    /// assert_eq!(buf.remaining(), 3);
    ///
    /// assert_eq!(buf.as_slice(), &[1, 2, 3]);
    /// assert_eq!(buf.read::<[u64; 1]>()?, [1]);
    /// assert_eq!(buf.as_slice(), &[2, 3]);
    /// buf.clear_remaining();
    /// assert_eq!(buf.as_slice(), &[1, 2, 3]);
    /// assert_eq!(buf.read::<[u64; 1]>()?, [1]);
    /// buf.clear();
    /// assert_eq!(buf.as_slice(), &[]);
    /// assert_eq!(buf.remaining_mut(), 3);
    /// # Ok::<_, pod::Error>(())
    #[inline]
    pub fn clear(&mut self) {
        if mem::needs_drop::<T>() {
            let read = mem::take(&mut self.read);
            let write = mem::take(&mut self.write);

            // SAFETY: The buffer is guaranteed to be initialized from the
            // `self.read..self.write` range.
            unsafe {
                let slice = slice::from_raw_parts_mut(
                    self.data.as_mut_ptr().add(read).cast::<T>(),
                    write - read,
                );

                ptr::drop_in_place(slice);
            }
        } else {
            self.read = 0;
            self.write = 0;
        }
    }

    /// Resets the buffer for reading.
    ///
    /// This clears the read position, allowing the buffer to be read from the
    /// start again.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Array, Reader, Writer};
    ///
    /// let mut buf = Array::<u64>::new();
    /// buf.write(42u64)?;
    ///
    /// assert_eq!(buf.as_slice(), &[42]);
    /// assert_eq!(buf.read::<[u64; 1]>()?, [42]);
    /// assert_eq!(buf.as_slice(), &[]);
    /// buf.clear_remaining();
    ///
    /// assert_eq!(buf.as_slice(), &[42]);
    /// assert_eq!(buf.read::<[u64; 1]>()?, [42]);
    /// # Ok::<_, pod::Error>(())
    #[inline]
    pub fn clear_remaining(&mut self)
    where
        T: Copy,
    {
        self.read = 0;
    }

    /// Returns the bytes of the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Array, Writer};
    ///
    /// let mut buf = Array::<u64>::new();
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
                self.data.as_ptr().add(self.read).cast(),
                self.remaining_bytes(),
            )
        }
    }

    /// Returns the slice of remaining data to be read.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Array, Writer};
    ///
    /// let mut buf = Array::<u64>::new();
    /// assert_eq!(buf.as_slice().len(), 0);
    ///
    /// buf.write(42u64)?;
    /// assert_eq!(buf.as_slice(), &[42]);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn as_slice(&self) -> &[T] {
        // SAFETY: The buffer is guaranteed to be initialized up to `pos`.
        unsafe { slice::from_raw_parts(self.data.as_ptr().add(self.read).cast(), self.remaining()) }
    }

    /// Returns a mutable slice of the remaining data to be read.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Array, Writer};
    ///
    /// let mut buf = Array::<u64>::new();
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
        unsafe {
            slice::from_raw_parts_mut(
                self.data.as_mut_ptr().add(self.read).cast(),
                self.remaining(),
            )
        }
    }
}

/// Debug implementation for `Buf`.
///
/// # Examples
///
/// ```
/// use pod::{Array, Reader};
///
/// let mut buf = Array::from_array([1u64, 2, 3]);
/// assert_eq!(format!("{buf:?}"), "[1, 2, 3]");
/// buf.read::<u64>()?;
/// assert_eq!(format!("{buf:?}"), "[2, 3]");
///
/// # Ok::<_, pod::Error>(())
/// ```
impl<T, const N: usize> fmt::Debug for Array<T, N>
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
/// use pod::Array;
///
/// let buf1 = Array::from_array([1, 2, 3]);
/// let buf2 = Array::from_array([1, 2, 3, 4]);
///
/// assert_ne!(buf1, buf2);
/// assert_eq!(buf1, buf1);
/// ```
impl<T, U, const A: usize, const B: usize> PartialEq<Array<U, B>> for Array<T, A>
where
    T: PartialEq<U>,
{
    #[inline]
    fn eq(&self, other: &Array<U, B>) -> bool {
        self.as_slice() == other.as_slice()
    }
}

/// Perform a partial comparison between two arrays.
///
/// # Examples
///
/// ```
/// use pod::Array;
///
/// let array1 = Array::from_array([1, 2, 3]);
/// let slice2: &[u64] = &[1, 2, 3, 4][..];
///
/// assert_ne!(array1, *slice2);
/// assert_eq!(array1, array1);
/// assert_eq!(*slice2, *slice2);
/// ```
impl<T, U, const N: usize> PartialEq<[U]> for Array<T, N>
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
/// use pod::Array;
///
/// let array1 = Array::from_array([1, 2, 3]);
/// let slice2: &[u64] = &[1, 2, 3, 4][..];
///
/// assert_ne!(array1, *slice2);
/// assert_eq!(array1, array1);
/// assert_eq!(*slice2, *slice2);
/// ```
impl<T, U, const N: usize> PartialEq<[U; N]> for Array<T, N>
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
/// use pod::Array;
///
/// let slice1 = Array::from_array([1, 2, 3]);
/// let slice2: &[u64] = &[1, 2, 3, 4][..];
///
/// assert_ne!(slice1, *slice2);
/// assert_eq!(slice1, slice1);
/// assert_eq!(*slice2, *slice2);
/// ```
impl<T, U, const N: usize> PartialEq<&[U; N]> for Array<T, N>
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
/// use pod::Array;
///
/// let array1 = Array::from_array([1, 2, 3]);
/// let slice2: &[u64] = &[1, 2, 3, 4][..];
///
/// assert_ne!(array1, slice2);
/// assert_eq!(array1, array1);
/// assert_eq!(slice2, slice2);
/// ```
impl<T, U, const N: usize> PartialEq<&[U]> for Array<T, N>
where
    T: PartialEq<U>,
{
    #[inline]
    fn eq(&self, other: &&[U]) -> bool {
        self.as_slice() == *other
    }
}

impl<T, const N: usize> Eq for Array<T, N> where T: Eq {}

impl<T, const N: usize> Drop for Array<T, N> {
    fn drop(&mut self) {
        self.clear();
    }
}

#[derive(Clone, Copy)]
pub struct Pos {
    write: usize,
    len: usize,
}

impl<T, const N: usize> Writer<T> for Array<T, N>
where
    T: Copy,
{
    type Mut<'this>
        = &'this mut Array<T, N>
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
    fn distance_from(&self, pos: Self::Pos) -> Option<u32> {
        u32::try_from(
            self.write
                .checked_sub(pos.write)?
                .checked_mul(mem::size_of::<T>())?,
        )
        .ok()
    }

    #[inline]
    fn write_words(&mut self, words: &[T]) -> Result<(), Error> {
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
        let write = self.write.wrapping_add(req);

        if !(self.write..=N).contains(&write) {
            return Err(Error::new(ErrorKind::BufferOverflow));
        }

        // SAFETY: We are writing to a valid position in the buffer.
        unsafe {
            let ptr = self.data.as_mut_ptr().add(self.write).cast::<u8>();
            ptr.copy_from_nonoverlapping(bytes.as_ptr(), bytes.len());
            let pad = mem::size_of::<T>() - bytes.len() % mem::size_of::<T>();
            ptr.add(bytes.len()).write_bytes(0, pad);
        }

        self.write = write;
        Ok(())
    }
}

impl<'de, T, const N: usize> Reader<'de, T> for Array<T, N>
where
    T: 'static + Copy,
{
    type Mut<'this>
        = &'this mut Array<T, N>
    where
        Self: 'this;

    type Clone<'this> = &'this [T];

    #[inline]
    fn borrow_mut(&mut self) -> Self::Mut<'_> {
        self
    }

    #[inline]
    fn clone_reader(&self) -> Self::Clone<'_> {
        self.as_slice()
    }

    #[inline]
    fn remaining_bytes(&self) -> usize {
        Array::remaining_bytes(self)
    }

    #[inline]
    fn skip(&mut self, size: u32) -> Result<(), Error> {
        let size = size.div_ceil(mem::size_of::<T>() as u32);

        let Ok(size) = usize::try_from(size) else {
            return Err(Error::new(ErrorKind::SizeOverflow));
        };

        let read = self.read.wrapping_add(size);

        if read > self.write || read < self.read {
            return Err(Error::new(ErrorKind::BufferUnderflow));
        }

        self.read = read;
        Ok(())
    }

    #[inline]
    fn split(&mut self, at: u32) -> Result<Self::Clone<'_>, Error> {
        let at = at.div_ceil(mem::size_of::<T>() as u32);

        let Ok(at) = usize::try_from(at) else {
            return Err(Error::new(ErrorKind::SizeOverflow));
        };

        let read = self.read.wrapping_add(at);

        if read > self.write || read < self.read {
            return Err(Error::new(ErrorKind::BufferUnderflow));
        }

        let tail = unsafe { slice::from_raw_parts(self.data.as_ptr().add(self.read).cast(), at) };

        self.read = read;
        Ok(tail)
    }

    #[inline]
    fn peek_words_uninit(&self, out: &mut [MaybeUninit<T>]) -> Result<(), Error> {
        if self.remaining() < out.len() {
            return Err(Error::new(ErrorKind::BufferUnderflow));
        }

        // SAFETY: The start pointer is valid since it hasn't reached the end yet.
        unsafe {
            self.data
                .as_ptr()
                .add(self.read)
                .cast::<MaybeUninit<T>>()
                .copy_to_nonoverlapping(out.as_mut_ptr(), out.len());
        }

        Ok(())
    }

    #[inline]
    fn read_words_uninit(&mut self, out: &mut [MaybeUninit<T>]) -> Result<(), Error> {
        let read = self.read.wrapping_add(out.len());

        if read > self.write || read < self.read {
            return Err(Error::new(ErrorKind::BufferUnderflow));
        }

        // SAFETY: The start pointer is valid since it hasn't reached the end yet.
        unsafe {
            self.data
                .as_ptr()
                .add(self.read)
                .cast::<MaybeUninit<T>>()
                .copy_to_nonoverlapping(out.as_mut_ptr(), out.len());
        }

        self.read = read;
        Ok(())
    }

    #[inline]
    fn read_bytes<V>(&mut self, len: u32, visitor: V) -> Result<V::Ok, Error>
    where
        T: BytesInhabited,
        V: Visitor<'de, [u8]>,
    {
        let Ok(len) = usize::try_from(len) else {
            return Err(Error::new(ErrorKind::SizeOverflow));
        };

        let req = len.div_ceil(mem::size_of::<T>());
        let read = self.read.wrapping_add(req);

        if read > self.write || read < self.read {
            return Err(Error::new(ErrorKind::BufferUnderflow));
        }

        let data = unsafe {
            let ptr = self.data.as_ptr().add(self.read).cast::<u8>();
            slice::from_raw_parts(ptr, len)
        };

        let ok = visitor.visit_ref(data)?;

        self.read = read;
        Ok(ok)
    }

    #[inline]
    fn as_bytes(&self) -> &[u8] {
        Array::as_bytes(self)
    }

    #[inline]
    fn as_slice(&self) -> &[T] {
        Array::as_slice(self)
    }
}
