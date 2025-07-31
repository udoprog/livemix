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
    /// We need to keep track of some alignment relative to the original slice
    /// to ensure that we can correctly unpad the slice as it's being read.
    ///
    /// Note this means that we don't support alignment requests larger than 256
    /// bytes.
    off: u8,
    /// The lifetime of the data in the slice.
    _marker: PhantomData<&'de [u8]>,
}

impl<'de> SliceBuf<'de> {
    /// Construct a new slice buffer from a slice.
    ///
    /// # Examples
    ///
    /// ```
    /// let slice = pod::buf::slice(&[1, 2, 3, 4]);
    /// assert_eq!(slice.len(), 4);
    /// assert_eq!(slice.as_bytes(), &[1, 2, 3, 4]);
    /// ```
    pub fn new(slice: &[u8]) -> Self {
        // SAFETY: The pointer is guaranteed to be valid since it was created
        // from a slice.
        Self {
            ptr: unsafe { NonNull::new_unchecked(slice.as_ptr().cast_mut()) },
            len: slice.len(),
            off: 0,
            _marker: PhantomData,
        }
    }

    /// Construct an owned buffer from the slice.
    ///
    /// # Examples
    ///
    /// ```
    /// let slice = pod::buf::slice(&[1, 2, 3, 4]);
    /// let buf = slice.to_owned()?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    pub fn to_owned(&self) -> Result<DynamicBuf, AllocError> {
        DynamicBuf::from_slice(self.as_bytes())
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
    /// let slice = pod::buf::slice(&[1, 2, 3, 4]);
    /// assert!(!slice.is_empty());
    /// assert_eq!(slice.len(), 4);
    /// assert_eq!(slice.as_bytes(), &[1, 2, 3, 4]);
    /// ```
    pub fn len(&self) -> usize {
        self.len
    }

    /// Check if the slice is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// let slice = pod::buf::slice(&[1, 2, 3, 4]);
    /// assert!(!slice.is_empty());
    /// assert_eq!(slice.len(), 4);
    /// assert_eq!(slice.as_bytes(), &[1, 2, 3, 4]);
    ///
    /// let slice = pod::buf::slice(&[]);
    /// assert!(slice.is_empty());
    /// assert_eq!(slice.len(), 0);
    /// assert_eq!(slice.as_bytes(), &[]);
    /// ```
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Get the contents of the slice as a slice.
    ///
    /// # Examples
    ///
    /// ```
    /// let slice = pod::buf::slice(&[1, 2, 3, 4]);
    /// assert!(!slice.is_empty());
    /// assert_eq!(slice.len(), 4);
    /// assert_eq!(slice.as_bytes(), &[1, 2, 3, 4]);
    /// ```
    pub fn as_bytes(&self) -> &'de [u8] {
        // SAFETY: The slice is guaranteed to be valid through construction.
        unsafe { slice::from_raw_parts(self.as_ptr(), self.len()) }
    }

    /// Split the current slice into two slices at the given position.
    ///
    /// # Examples
    ///
    /// ```
    /// let slice = pod::buf::slice(&[1, 2, 3, 4]);
    /// let (a, b) = slice.split_at_checked(2).unwrap();
    /// assert_eq!(a.as_bytes(), &[1, 2]);
    /// assert_eq!(b.as_bytes(), &[3, 4]);
    /// ```
    pub fn split_at_checked(&self, at: usize) -> Option<(SliceBuf<'de>, SliceBuf<'de>)> {
        if at > self.len() {
            return None;
        }

        let a = SliceBuf {
            ptr: self.ptr,
            len: at,
            off: self.off,
            _marker: PhantomData,
        };

        let b = SliceBuf {
            ptr: unsafe { wrapping_add(self.ptr, at) },
            len: self.len.wrapping_sub(at),
            off: (self.off as usize).wrapping_add(at) as u8,
            _marker: PhantomData,
        };

        Some((a, b))
    }

    #[inline]
    fn offset(&mut self, size: usize) {
        self.off = (self.off as usize).wrapping_add(size) as u8;
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
            off: self.off,
            _marker: PhantomData,
        }
    }
}

/// A stored slice position.
#[derive(Clone, Copy)]
pub struct Pos<'de> {
    ptr: NonNull<u8>,
    _marker: PhantomData<&'de [u8]>,
}

impl<'de> Reader<'de> for SliceBuf<'de> {
    /// The type of a split off reader.
    type Split = SliceBuf<'de>;

    type Mut<'this>
        = &'this mut SliceBuf<'de>
    where
        Self: 'this;

    type Pos = Pos<'de>;

    #[inline]
    fn borrow_mut(&mut self) -> Self::Mut<'_> {
        self
    }

    /// Get the current reader position.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::Reader;
    ///
    /// let mut buf = pod::buf::slice(&[0; 32]);
    ///
    /// let pos = buf.pos();
    /// _ = buf.read::<u32>()?;
    /// assert_eq!(buf.distance_from(pos), 4);
    /// _ = buf.read::<u32>()?;
    /// assert_eq!(buf.distance_from(pos), 8);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    fn pos(&self) -> Self::Pos {
        Pos {
            ptr: self.ptr,
            _marker: PhantomData,
        }
    }

    /// Get the distance from a stored reader position to the current reader
    /// position.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::Reader;
    ///
    /// let mut buf = pod::buf::slice(&[0; 32]);
    ///
    /// let pos = buf.pos();
    /// _ = buf.read::<u32>()?;
    /// assert_eq!(buf.distance_from(pos), 4);
    /// _ = buf.read::<u32>()?;
    /// assert_eq!(buf.distance_from(pos), 8);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    fn distance_from(&self, pos: Self::Pos) -> usize {
        self.ptr.addr().get().wrapping_sub(pos.ptr.addr().get())
    }

    /// Skip the given number of bytes in the reader.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::Reader;
    ///
    /// let mut buf = pod::buf::slice(&[0; 32]);
    ///
    /// assert_eq!(buf.len(), 32);
    /// buf.skip(4)?;
    /// assert_eq!(buf.len(), 28);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    fn skip(&mut self, size: usize) -> Result<(), Error> {
        let Some((_, tail)) = self.split_at_checked(size) else {
            return Err(Error::new(ErrorKind::BufferUnderflow));
        };

        *self = tail;
        Ok(())
    }

    /// Unpad the given buffer to the specified alignment.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::Reader;
    ///
    /// let mut buf = pod::buf::slice(&[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x7b, 0x7b, 0x7b, 0x7b]);
    ///
    /// let pos = buf.pos();
    /// buf.skip(4)?;
    /// assert_eq!(buf.distance_from(pos), 4);
    /// buf.unpad(8)?;
    /// assert_eq!(buf.distance_from(pos), 8);
    /// buf.unpad(6)?;
    /// assert_eq!(buf.distance_from(pos), 12);
    /// assert_eq!(buf.read::<u32>()?, 0x7b7b7b7b);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    fn unpad(&mut self, align: usize) -> Result<(), Error> {
        debug_assert!(
            align <= u8::MAX as usize,
            "Alignments larger than 256 bytes are not supported"
        );

        let remaining = (self.off as usize) % align;

        if remaining == 0 {
            return Ok(());
        }

        let pad = align - remaining;

        if pad > self.len {
            return Err(Error::new(ErrorKind::BufferUnderflow));
        }

        self.len -= pad;
        self.ptr = unsafe { wrapping_add(self.ptr, pad) };
        self.offset(pad);
        Ok(())
    }

    /// Split the given buffer to the specified distance.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::Reader;
    ///
    /// let mut buf = pod::buf::slice(&[0xa8, 0xa8, 0xa8, 0xa8, 0x7b, 0x7b, 0x7b, 0x7b]);
    ///
    /// let mut buf1 = buf.split(4).unwrap();
    /// assert_eq!(buf1.read::<u32>()?, 0xa8a8a8a8);
    /// assert_eq!(buf.read::<u32>()?, 0x7b7b7b7b);
    /// # Ok::<_, pod::Error>(())
    /// ```
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
        self.offset(out.len());
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
        let ok = visitor.visit_borrowed(head.as_bytes())?;
        *self = tail;
        Ok(ok)
    }

    #[inline]
    fn as_bytes(&self) -> &[u8] {
        (*self).as_bytes()
    }

    #[inline]
    fn len(&self) -> usize {
        (*self).len()
    }

    #[inline]
    fn is_empty(&self) -> bool {
        (*self).is_empty()
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
        self.offset(len);
        out
    }
}

impl fmt::Debug for SliceBuf<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.as_bytes()).finish()
    }
}

unsafe fn wrapping_add<T>(ptr: NonNull<T>, addr: usize) -> NonNull<T> {
    unsafe { NonNull::new_unchecked(ptr.as_ptr().wrapping_add(addr)) }
}
