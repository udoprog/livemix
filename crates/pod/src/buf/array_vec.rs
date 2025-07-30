use core::fmt;
use core::mem::{self, ManuallyDrop, MaybeUninit};
use core::ptr;
use core::slice;

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
/// use pod::buf::ArrayVec;
///
/// let mut buf = ArrayVec::<u32, 16>::from_slice(&[1, 2, 3, 4]);
/// assert_eq!(buf.len(), 4);
/// buf.push(5)?;
/// assert_eq!(buf.as_slice(), &[1, 2, 3, 4, 5]);
/// assert_eq!(buf.pop(), Some(5));
/// assert_eq!(buf.len(), 4);
/// # Ok::<_, pod::buf::CapacityError>(())
/// ```
///
/// Trying to read data from the array in a manner which is *not* correctly
/// aligned will errors:
///
/// ```compile_fail
/// use pod::buf::ArrayVec;
///
/// let mut buf = ArrayVec::<u64, 16>::from_slice(&[1, 2, 3, 4]);
/// // This must fail because it's not possible to read half of a word out of the array.
/// buf.read::<u32>()?;
/// # Ok::<_, pod::buf::CapacityError>(())
/// ```
pub struct ArrayVec<T, const N: usize = DEFAULT_SIZE> {
    data: [MaybeUninit<T>; N],
    len: usize,
}

impl<T, const N: usize> ArrayVec<T, N> {
    /// Construct a new array buffer with a default size.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::buf::ArrayVec;
    ///
    /// let buf = ArrayVec::<u64>::new();
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
    /// use pod::buf::ArrayVec;
    ///
    /// let mut buf = ArrayVec::<String>::new();
    /// buf.push("Hello".to_string())?;
    /// # Ok::<_, pod::buf::CapacityError>(())
    /// ```
    pub fn push(&mut self, value: T) -> Result<(), CapacityError> {
        if self.len >= N {
            return Err(CapacityError);
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

    /// Extend the buffer with a slice of values.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::buf::ArrayVec;
    ///
    /// let mut buf = ArrayVec::<u64, 5>::new();
    /// buf.extend_from_slice(&[1, 2, 3])?;
    /// assert_eq!(buf.as_slice(), &[1, 2, 3]);
    /// # Ok::<_, pod::buf::CapacityError>(())
    /// ```
    pub fn extend_from_slice(&mut self, slice: &[T]) -> Result<(), CapacityError>
    where
        T: Copy,
    {
        let len = self.len.wrapping_add(slice.len());

        if len > N {
            return Err(CapacityError);
        }

        // SAFETY: We are writing to a valid position in the buffer.
        unsafe {
            self.data
                .as_mut_ptr()
                .add(self.len)
                .cast::<T>()
                .copy_from_nonoverlapping(slice.as_ptr(), slice.len());
        }

        self.len = len;
        Ok(())
    }

    /// Push a value from the array.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::buf::ArrayVec;
    ///
    /// let mut buf = ArrayVec::<String>::new();
    /// buf.push(String::from("Hello"))?;
    /// buf.push(String::from("World"))?;
    ///
    /// assert_eq!(buf.pop(), Some(String::from("World")));
    /// assert_eq!(buf.pop(), Some(String::from("Hello")));
    /// assert_eq!(buf.pop(), None);
    /// # Ok::<_, pod::buf::CapacityError>(())
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

    /// Construct from an initialized array.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::buf::ArrayVec;
    ///
    /// let buf = ArrayVec::<u64, 3>::from_array([1, 2, 3]);
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
    /// use pod::buf::ArrayVec;
    ///
    /// ArrayVec::<u64, 16>::from_slice(&[0; 32]);
    /// ```
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::buf::ArrayVec;
    ///
    /// let buf = ArrayVec::<u64, 16>::from_slice(&[1, 2, 3]);
    /// assert_eq!(buf.len(), 3);
    /// assert_eq!(buf.capacity(), 16);
    /// assert_eq!(buf.as_slice(), &[1, 2, 3]);
    /// ```
    pub const fn from_slice(words: &[T]) -> Self
    where
        T: Copy,
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
    /// use pod::buf::ArrayVec;
    ///
    /// let mut buf = ArrayVec::<u64>::new();
    /// assert!(buf.is_empty());
    /// assert_eq!(buf.len(), 0);
    /// buf.push(42)?;
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
    /// use pod::buf::ArrayVec;
    ///
    /// let mut buf = ArrayVec::<u64>::new();
    /// assert!(buf.is_empty());
    /// assert_eq!(buf.len(), 0);
    /// buf.push(42)?;
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
    /// use pod::buf::ArrayVec;
    ///
    /// let buf = ArrayVec::<u32, 16>::from_slice(&[1, 2, 3]);
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
    /// use pod::buf::ArrayVec;
    ///
    /// let mut buf = ArrayVec::<u64, 3>::from_array([1, 2, 3]);
    /// assert_eq!(buf.len(), 3);
    /// assert_eq!(buf.as_slice(), &[1, 2, 3]);
    /// buf.clear();
    /// assert!(buf.as_slice().is_empty());
    ///
    /// let mut buf = ArrayVec::<String, 2>::from_array([String::from("Hello"), String::from("World")]);
    /// assert_eq!(buf.len(), 2);
    /// assert_eq!(buf.as_slice(), &[String::from("Hello"), String::from("World")]);
    /// buf.clear();
    /// assert!(buf.as_slice().is_empty());
    /// # Ok::<_, pod::buf::CapacityError>(())
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

    /// Returns the slice of data in the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::buf::ArrayVec;
    ///
    /// let mut buf = ArrayVec::<u64>::new();
    /// assert_eq!(buf.as_slice().len(), 0);
    /// buf.push(42)?;
    /// assert_eq!(buf.as_slice(), &[42]);
    /// # Ok::<_, pod::buf::CapacityError>(())
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
    /// use pod::buf::ArrayVec;
    ///
    /// let mut buf = ArrayVec::<u64>::new();
    /// assert_eq!(buf.as_slice().len(), 0);
    ///
    /// buf.push(42)?;
    /// assert_eq!(buf.as_slice(), &[42]);
    ///
    /// buf.as_slice_mut()[0] = 43;
    /// assert_eq!(buf.as_slice(), &[43]);
    /// # Ok::<_, pod::buf::CapacityError>(())
    /// ```
    pub fn as_slice_mut(&mut self) -> &mut [T] {
        // SAFETY: The buffer is guaranteed to be initialized from the
        // `self.read..self.write` range.
        unsafe { slice::from_raw_parts_mut(self.data.as_mut_ptr().cast(), self.len) }
    }

    /// Try to coerce into an initialized inner array, checking if it has been
    /// initialized first.
    pub fn into_inner(self) -> Option<[T; N]> {
        if self.len == N {
            let this = ManuallyDrop::new(self);

            // SAFETY: The buffer is guaranteed to be initialized.
            Some(unsafe { ptr::read(&this.data as *const _ as *const [T; N]) })
        } else {
            None
        }
    }
}

impl<T> Default for ArrayVec<T> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

/// Debug implementation for `ArrayVec`.
///
/// # Examples
///
/// ```
/// use pod::buf::ArrayVec;
///
/// let mut buf = ArrayVec::from_array([1, 2, 3]);
/// assert_eq!(format!("{buf:?}"), "[1, 2, 3]");
/// buf.pop();
/// assert_eq!(format!("{buf:?}"), "[1, 2]");
/// # Ok::<_, pod::buf::CapacityError>(())
/// ```
impl<T, const N: usize> fmt::Debug for ArrayVec<T, N>
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
/// use pod::buf::ArrayVec;
///
/// let buf1 = ArrayVec::from_array([1, 2, 3]);
/// let buf2 = ArrayVec::from_array([1, 2, 3, 4]);
///
/// assert_ne!(buf1, buf2);
/// assert_eq!(buf1, buf1);
/// ```
impl<T, U, const A: usize, const B: usize> PartialEq<ArrayVec<U, B>> for ArrayVec<T, A>
where
    T: PartialEq<U>,
{
    #[inline]
    fn eq(&self, other: &ArrayVec<U, B>) -> bool {
        self.as_slice() == other.as_slice()
    }
}

/// Perform a partial comparison between two arrays.
///
/// # Examples
///
/// ```
/// use pod::buf::ArrayVec;
///
/// let array1 = ArrayVec::from_array([1, 2, 3]);
/// let slice2: &[u64] = &[1, 2, 3, 4][..];
///
/// assert_ne!(array1, *slice2);
/// assert_eq!(array1, array1);
/// assert_eq!(*slice2, *slice2);
/// ```
impl<T, U, const N: usize> PartialEq<[U]> for ArrayVec<T, N>
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
/// use pod::buf::ArrayVec;
///
/// let array1 = ArrayVec::from_array([1, 2, 3]);
/// let slice2: &[u64] = &[1, 2, 3, 4][..];
///
/// assert_ne!(array1, *slice2);
/// assert_eq!(array1, array1);
/// assert_eq!(*slice2, *slice2);
/// ```
impl<T, U, const N: usize> PartialEq<[U; N]> for ArrayVec<T, N>
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
/// use pod::buf::ArrayVec;
///
/// let slice1 = ArrayVec::from_array([1, 2, 3]);
/// let slice2: &[u64] = &[1, 2, 3, 4][..];
///
/// assert_ne!(slice1, *slice2);
/// assert_eq!(slice1, slice1);
/// assert_eq!(*slice2, *slice2);
/// ```
impl<T, U, const N: usize> PartialEq<&[U; N]> for ArrayVec<T, N>
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
/// use pod::buf::ArrayVec;
///
/// let array1 = ArrayVec::from_array([1, 2, 3]);
/// let slice2: &[u64] = &[1, 2, 3, 4][..];
///
/// assert_ne!(array1, slice2);
/// assert_eq!(array1, array1);
/// assert_eq!(slice2, slice2);
/// ```
impl<T, U, const N: usize> PartialEq<&[U]> for ArrayVec<T, N>
where
    T: PartialEq<U>,
{
    #[inline]
    fn eq(&self, other: &&[U]) -> bool {
        self.as_slice() == *other
    }
}

impl<T, const N: usize> Eq for ArrayVec<T, N> where T: Eq {}

impl<T, const N: usize> Drop for ArrayVec<T, N> {
    fn drop(&mut self) {
        self.clear();
    }
}
