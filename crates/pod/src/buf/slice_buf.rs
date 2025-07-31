use core::fmt;
use core::marker::PhantomData;
use core::mem::MaybeUninit;
use core::ptr::NonNull;
use core::slice;

use crate::error::ErrorKind;
use crate::{AsReader, DynamicBuf, Error, Reader, SplitReader, Visitor};

use super::AllocError;

/// A buffer that represents a slice of bytes.
#[derive(Clone, Copy)]
pub struct SliceBuf<'de> {
    /// The pointer to the start of the slice.
    ptr: NonNull<u8>,
    ///  The length of the slice in bytes.
    len: usize,
    /// We need to keep track of the *original* read position to support
    /// adjusting for alignment.
    at: usize,
    /// The lifetime of the data in the slice.
    _marker: PhantomData<&'de [u8]>,
}

impl<'de> SliceBuf<'de> {
    /// Construct a new slice buffer from a slice.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::SliceBuf;
    ///
    /// let slice = SliceBuf::new(&[1, 2, 3, 4]);
    /// assert_eq!(slice.len(), 4);
    /// assert_eq!(slice.as_slice(), &[1, 2, 3, 4]);
    /// ```
    pub fn new(slice: &[u8]) -> Self {
        // SAFETY: The pointer is guaranteed to be valid since it was created
        // from a slice.
        Self {
            ptr: unsafe { NonNull::new_unchecked(slice.as_ptr().cast_mut()) },
            len: slice.len(),
            at: 0,
            _marker: PhantomData,
        }
    }

    /// Construct an owned buffer from the slice.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::SliceBuf;
    ///
    /// let slice = SliceBuf::new(&[1, 2, 3, 4]);
    /// let buf = slice.to_owned()?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    pub fn to_owned(&self) -> Result<DynamicBuf, AllocError> {
        DynamicBuf::from_slice(self.as_slice())
    }

    /// Base pointer of the slice.
    fn as_ptr(&self) -> *const u8 {
        // SAFETY: The pointer is guaranteed to be valid since it was created from
        // a slice of words.
        self.ptr.as_ptr()
    }

    /// The length of the slice in bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::SliceBuf;
    ///
    /// let slice = SliceBuf::new(&[1, 2, 3, 4]);
    /// assert!(!slice.is_empty());
    /// assert_eq!(slice.len(), 4);
    /// assert_eq!(slice.as_slice(), &[1, 2, 3, 4]);
    /// ```
    pub fn len(&self) -> usize {
        self.len
    }

    /// Check if the slice is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::SliceBuf;
    ///
    /// let slice = SliceBuf::new(&[1, 2, 3, 4]);
    /// assert!(!slice.is_empty());
    /// assert_eq!(slice.len(), 4);
    /// assert_eq!(slice.as_slice(), &[1, 2, 3, 4]);
    ///
    /// let slice = SliceBuf::new(&[]);
    /// assert!(slice.is_empty());
    /// assert_eq!(slice.len(), 0);
    /// assert_eq!(slice.as_slice(), &[]);
    /// ```
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Get the contents of the slice as a slice.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::SliceBuf;
    ///
    /// let slice = SliceBuf::new(&[1, 2, 3, 4]);
    /// assert!(!slice.is_empty());
    /// assert_eq!(slice.len(), 4);
    /// assert_eq!(slice.as_slice(), &[1, 2, 3, 4]);
    /// ```
    pub fn as_slice(&self) -> &'de [u8] {
        // SAFETY: The slice is guaranteed to be valid through construction.
        unsafe { slice::from_raw_parts(self.as_ptr(), self.len()) }
    }

    /// Split the current slice into two slices at the given position.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::SliceBuf;
    ///
    /// let slice = SliceBuf::new(&[1, 2, 3, 4]);
    /// let (a, b) = slice.split_at_checked(2).unwrap();
    /// assert_eq!(a.as_slice(), &[1, 2]);
    /// assert_eq!(b.as_slice(), &[3, 4]);
    /// ```
    pub fn split_at_checked(&self, at: usize) -> Option<(SliceBuf<'de>, SliceBuf<'de>)> {
        if at > self.len() {
            return None;
        }

        let a = SliceBuf {
            ptr: self.ptr,
            len: at,
            at: self.at,
            _marker: PhantomData,
        };

        let b = SliceBuf {
            ptr: unsafe { wrapping_add(self.ptr, at) },
            len: self.len.wrapping_sub(at),
            at: self.at.wrapping_add(at),
            _marker: PhantomData,
        };

        Some((a, b))
    }
}

impl AsReader for SliceBuf<'_> {
    type AsReader<'this>
        = SliceBuf<'this>
    where
        Self: 'this;

    #[inline]
    fn as_reader(&self) -> Self::AsReader<'_> {
        SliceBuf {
            ptr: self.ptr,
            len: self.len,
            at: self.at,
            _marker: PhantomData,
        }
    }
}

/// A stored slice position.
#[derive(Clone, Copy)]
pub struct Pos {
    at: usize,
}

impl<'de> Reader<'de> for SliceBuf<'de> {
    /// The type of a split off reader.
    type Split = SliceBuf<'de>;

    type Mut<'this>
        = &'this mut SliceBuf<'de>
    where
        Self: 'this;

    type Pos = Pos;

    #[inline]
    fn borrow_mut(&mut self) -> Self::Mut<'_> {
        self
    }

    #[inline]
    fn pos(&self) -> Self::Pos {
        Pos { at: self.at }
    }

    #[inline]
    fn distance_from(&self, pos: Self::Pos) -> usize {
        self.at.saturating_sub(pos.at)
    }

    #[inline]
    fn skip(&mut self, size: usize) -> Result<(), Error> {
        let Some((_, tail)) = self.split_at_checked(size) else {
            return Err(Error::new(ErrorKind::BufferUnderflow));
        };

        *self = tail;
        Ok(())
    }

    #[inline]
    fn unpad(&mut self, align: usize) -> Result<(), Error> {
        let remaining = self.at % align;

        if remaining == 0 {
            return Ok(());
        }

        let pad = align - remaining;

        if pad > self.len {
            return Err(Error::new(ErrorKind::BufferUnderflow));
        }

        self.len -= pad;
        self.at += pad;
        self.ptr = unsafe { wrapping_add(self.ptr, pad) };
        Ok(())
    }

    #[inline]
    fn split(&mut self, at: usize) -> Option<Self::Split> {
        let (head, tail) = self.split_at_checked(at)?;
        *self = tail;
        Some(head)
    }

    #[inline]
    fn peek_words_uninit(&self, out: &mut [MaybeUninit<u8>]) -> Result<(), Error> {
        if out.len() > self.len() {
            return Err(Error::new(ErrorKind::BufferUnderflow));
        }

        // SAFETY: The start pointer is valid since it hasn't reached the end yet.
        unsafe {
            self.as_ptr()
                .cast::<MaybeUninit<u8>>()
                .copy_to_nonoverlapping(out.as_mut_ptr(), out.len());
        }

        Ok(())
    }

    #[inline]
    fn read_words_uninit(&mut self, out: &mut [MaybeUninit<u8>]) -> Result<(), Error> {
        if out.len() > self.len {
            return Err(Error::new(ErrorKind::BufferUnderflow));
        }

        // SAFETY: The start pointer is valid since it hasn't reached the end yet.
        unsafe {
            self.as_ptr()
                .cast::<MaybeUninit<u8>>()
                .copy_to_nonoverlapping(out.as_mut_ptr(), out.len());
        }

        self.ptr = unsafe { wrapping_add(self.ptr, out.len()) };
        self.len = self.len.wrapping_sub(out.len());
        self.at = self.at.wrapping_add(out.len());
        Ok(())
    }

    #[inline]
    fn read_bytes<V>(&mut self, len: usize, visitor: V) -> Result<V::Ok, Error>
    where
        V: Visitor<'de, [u8]>,
    {
        let Some((head, tail)) = self.split_at_checked(len) else {
            return Err(Error::new(ErrorKind::BufferUnderflow));
        };

        // SAFETY: The head is guaranteed to be valid since it was split from the original slice.
        let ok = visitor.visit_borrowed(head.as_slice())?;
        *self = tail;
        Ok(ok)
    }

    #[inline]
    fn as_bytes(&self) -> &[u8] {
        self.as_slice()
    }

    #[inline]
    fn len(&self) -> usize {
        (*self).len()
    }

    #[inline]
    fn is_empty(&self) -> bool {
        (*self).is_empty()
    }

    #[inline]
    fn as_slice(&self) -> SliceBuf<'de> {
        *self
    }
}

impl SplitReader for SliceBuf<'_> {
    type TakeReader<'this>
        = SliceBuf<'this>
    where
        Self: 'this;

    #[inline]
    fn take_reader(&mut self) -> Self::TakeReader<'_> {
        let out = *self;

        let len = self.len;
        self.ptr = unsafe { wrapping_add(self.ptr, len) };
        self.len = 0;
        self.at += len;
        out
    }
}

impl fmt::Debug for SliceBuf<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.as_slice()).finish()
    }
}

unsafe fn wrapping_add<T>(ptr: NonNull<T>, addr: usize) -> NonNull<T> {
    unsafe { NonNull::new_unchecked(ptr.as_ptr().wrapping_add(addr)) }
}
