use core::fmt;
use core::mem::MaybeUninit;
use core::slice;

use crate::error::ErrorKind;
use crate::visitor::Visitor;
use crate::{Error, Reader, WORD_SIZE};

/// A slice of words that can be used for decoding.
pub struct Slice([u32]);

impl Slice {
    /// Creates a new `Slice` from a byte slice.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::Slice;
    ///
    /// let array = Slice::new(&[1, 2, 3]);
    /// assert_eq!(array.len(), 3);
    /// assert_eq!(array.as_slice(), &[1, 2, 3]);
    /// ```
    pub const fn new(slice: &[u32]) -> &Self {
        // SAFETY: The slice is guaranteed to be valid and initialized.
        unsafe { &*(slice as *const [u32] as *const Self) }
    }

    /// Returns the length of the slice.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::Slice;
    ///
    /// let slice = Slice::new(&[1, 2, 3]);
    /// assert_eq!(slice.len(), 3);
    /// ```
    pub const fn len(&self) -> usize {
        self.0.len()
    }

    /// Test if the slice is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::Slice;
    ///
    /// let slice = Slice::new(&[1, 2, 3]);
    /// assert_eq!(slice.len(), 3);
    /// assert!(!slice.is_empty());
    /// ```
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the contents of the slice.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::Slice;
    ///
    /// let slice = Slice::new(&[1, 2, 3]);
    /// assert_eq!(slice.len(), 3);
    /// assert!(!slice.is_empty());
    /// assert_eq!(slice.as_slice(), &[1, 2, 3]);
    /// ```
    pub const fn as_slice(&self) -> &[u32] {
        &self.0
    }
}

/// Perform a partial comparison between two slices.
///
/// # Examples
///
/// ```
/// use pod::Slice;
///
/// let slice1 = Slice::new(&[1, 2, 3]);
/// let slice2 = Slice::new(&[1, 2, 3, 4]);
///
/// assert_ne!(slice1, slice2);
/// assert_eq!(slice1, slice1);
/// ```
impl PartialEq for Slice {
    #[inline]
    fn eq(&self, other: &Slice) -> bool {
        self.as_slice() == other.as_slice()
    }
}

/// Perform a partial comparison between two slices.
///
/// # Examples
///
/// ```
/// use pod::Slice;
///
/// let slice1 = Slice::new(&[1, 2, 3]);
/// let slice2: &[u32] = &[1, 2, 3, 4][..];
///
/// assert_ne!(*slice1, *slice2);
/// assert_eq!(*slice1, *slice1);
/// assert_eq!(*slice2, *slice2);
/// ```
impl PartialEq<[u32]> for Slice {
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
/// use pod::Slice;
///
/// let slice1 = Slice::new(&[1, 2, 3]);
/// let slice2: &[u32] = &[1, 2, 3, 4][..];
///
/// assert_ne!(*slice1, *slice2);
/// assert_eq!(*slice1, *slice1);
/// assert_eq!(*slice2, *slice2);
/// ```
impl<const N: usize> PartialEq<[u32; N]> for Slice {
    #[inline]
    fn eq(&self, other: &[u32; N]) -> bool {
        self.as_slice() == &other[..]
    }
}

/// Perform a partial comparison between a slice and an array.
///
/// # Examples
///
/// ```
/// use pod::Slice;
///
/// let slice1 = Slice::new(&[1, 2, 3]);
/// let slice2: &[u32] = &[1, 2, 3, 4][..];
///
/// assert_ne!(*slice1, *slice2);
/// assert_eq!(*slice1, *slice1);
/// assert_eq!(*slice2, *slice2);
/// ```
impl<const N: usize> PartialEq<&[u32; N]> for Slice {
    #[inline]
    fn eq(&self, other: &&[u32; N]) -> bool {
        self.as_slice() == &other[..]
    }
}

/// Perform a partial comparison between two slices.
///
/// # Examples
///
/// ```
/// use pod::Slice;
///
/// let slice1 = Slice::new(&[1, 2, 3]);
/// let slice2: &[u32] = &[1, 2, 3, 4][..];
///
/// assert_ne!(slice1, slice2);
/// assert_eq!(slice1, slice1);
/// assert_eq!(slice2, slice2);
/// ```
impl PartialEq<&[u32]> for Slice {
    #[inline]
    fn eq(&self, other: &&[u32]) -> bool {
        self.as_slice() == *other
    }
}

impl Eq for Slice {}

impl fmt::Debug for Slice {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_slice().fmt(f)
    }
}

impl<'de> Reader<'de> for &'de Slice {
    type Mut<'this>
        = &'this mut &'de Slice
    where
        Self: 'this;

    #[inline]
    fn borrow_mut(&mut self) -> Self::Mut<'_> {
        self
    }

    #[inline]
    fn peek_words_uninit(&self, out: &mut [MaybeUninit<u32>]) -> Result<(), Error> {
        if out.len() > self.len() {
            return Err(Error::new(ErrorKind::BufferUnderflow));
        }

        // SAFETY: The start pointer is valid since it hasn't reached the end yet.
        unsafe {
            self.0
                .as_ptr()
                .cast::<MaybeUninit<u32>>()
                .copy_to_nonoverlapping(out.as_mut_ptr(), out.len());
        }

        Ok(())
    }

    #[inline]
    fn read_words_uninit(&mut self, out: &mut [MaybeUninit<u32>]) -> Result<(), Error> {
        if out.len() > self.len() {
            return Err(Error::new(ErrorKind::BufferUnderflow));
        }

        // SAFETY: The start pointer is valid since it hasn't reached the end yet.
        unsafe {
            self.0
                .as_ptr()
                .cast::<MaybeUninit<u32>>()
                .copy_to_nonoverlapping(out.as_mut_ptr(), out.len());
        }

        *self = Slice::new(&self.0[out.len()..]);
        Ok(())
    }

    #[inline]
    fn read_bytes<V>(&mut self, len: usize, visitor: V) -> Result<V::Ok, Error>
    where
        V: Visitor<'de, [u8]>,
    {
        let req = len.div_ceil(WORD_SIZE).next_multiple_of(2);

        let Some((head, tail)) = self.0.split_at_checked(req) else {
            return Err(Error::new(ErrorKind::BufferUnderflow));
        };

        let value = unsafe { slice::from_raw_parts(head.as_ptr().cast::<u8>(), len) };
        let ok = visitor.visit_borrowed(value)?;
        *self = Slice::new(tail);
        Ok(ok)
    }
}
