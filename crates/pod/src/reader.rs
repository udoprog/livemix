use core::marker::PhantomData;
use core::mem::{self, MaybeUninit};
use core::slice;

use crate::error::ErrorKind;
use crate::utils::{AlignableWith, UninitAlign};
use crate::{AsReader, Error, Type, Visitor};

mod sealed {
    use crate::{ArrayBuf, Reader};

    pub trait Sealed {}

    impl Sealed for &[u64] {}
    impl<const N: usize> Sealed for ArrayBuf<N> {}
    impl<'de, R> Sealed for &mut R where R: ?Sized + Reader<'de> {}
}

/// A type that u32 words can be read from.
pub trait Reader<'de>
where
    Self: AsReader + self::sealed::Sealed,
{
    /// The type of a split off reader.
    type Split: Reader<'de>;

    /// The mutable borrow of a reader.
    type Mut<'this>: Reader<'de>
    where
        Self: 'this;

    /// The position type used by the reader.
    type Pos: 'de + Copy;

    /// Borrow the current reader mutably.
    fn borrow_mut(&mut self) -> Self::Mut<'_>;

    /// Get the current position in the reader.
    fn pos(&self) -> Self::Pos;

    /// Get the position of the reader relative to the queried position.
    fn distance_from(&self, pos: Self::Pos) -> usize;

    /// Returns the size of the remaining buffer in bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Reader};
    ///
    /// let array = ArrayBuf::from_array([1, 2, 3]);
    /// let mut buf = array.as_slice();
    ///
    /// assert_eq!(buf.remaining(), 24);
    /// assert_eq!(buf.read::<[u64; 1]>()?, [1]);
    /// assert_eq!(buf.remaining(), 16);
    /// assert_eq!(buf.as_slice(), &[2, 3]);
    /// assert_eq!(buf.read::<[u64; 2]>()?, [2, 3]);
    /// assert_eq!(buf.remaining(), 0);
    /// # Ok::<_, pod::Error>(())
    /// ```
    fn remaining(&self) -> usize;

    /// Skip the given number of bytes.
    fn skip(&mut self, size: usize) -> Result<(), Error>;

    /// Split off the head of the current buffer.
    fn split(&mut self, at: usize) -> Option<Self::Split>;

    /// Peek into the provided buffer without consuming the reader.
    fn peek_words_uninit(&self, out: &mut [MaybeUninit<u64>]) -> Result<(), Error>;

    /// Peek words into the provided buffer.
    fn read_words_uninit(&mut self, out: &mut [MaybeUninit<u64>]) -> Result<(), Error>;

    /// Read the given number of bytes from the input.
    fn read_bytes<V>(&mut self, len: usize, visitor: V) -> Result<V::Ok, Error>
    where
        V: Visitor<'de, [u8]>;

    /// Returns the bytes of the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Writer};
    ///
    /// let mut buf = ArrayBuf::default();
    /// assert_eq!(buf.as_bytes().len(), 0);
    ///
    /// buf.write(42u64)?;
    /// let expected = 42u64.to_ne_bytes();
    /// assert_eq!(buf.as_bytes(), &expected[..]);
    /// # Ok::<_, pod::Error>(())
    /// ```
    fn as_bytes(&self) -> &[u8];

    /// Returns the length of the bytes in the buffer.
    fn bytes_len(&self) -> usize;

    /// Returns the slice of remaining data to be read.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Writer};
    ///
    /// let mut buf = ArrayBuf::default();
    /// assert_eq!(buf.as_slice().len(), 0);
    ///
    /// buf.write(42u64)?;
    /// assert_eq!(buf.as_slice(), &[42]);
    /// # Ok::<_, pod::Error>(())
    /// ```
    fn as_slice(&self) -> &'de [u64];

    /// Read an array of words.
    #[inline]
    fn peek<U>(&self) -> Result<U, Error>
    where
        U: AlignableWith,
    {
        let mut out = UninitAlign::<U>::uninit();
        self.peek_words_uninit(out.as_mut_slice())?;
        // SAFETY: The slice must have been initialized by the reader.
        Ok(unsafe { out.assume_init() })
    }

    /// Read an array of words.
    #[inline]
    fn read<U>(&mut self) -> Result<U, Error>
    where
        U: AlignableWith,
    {
        let mut out = UninitAlign::<U>::uninit();
        self.read_words_uninit(out.as_mut_slice())?;
        // SAFETY: The slice must have been initialized by the reader.
        Ok(unsafe { out.assume_init() })
    }

    #[inline]
    fn header(&mut self) -> Result<(usize, Type), Error> {
        let [size, ty] = self.read::<[u32; 2]>()?;
        let ty = Type::new(ty);

        let Ok(size) = usize::try_from(size) else {
            return Err(Error::new(ErrorKind::SizeOverflow));
        };

        Ok((size, ty))
    }
}

impl<'de, R> Reader<'de> for &mut R
where
    R: ?Sized + Reader<'de>,
{
    type Split = R::Split;

    type Mut<'this>
        = R::Mut<'this>
    where
        Self: 'this;

    type Pos = R::Pos;

    #[inline]
    fn borrow_mut(&mut self) -> Self::Mut<'_> {
        (**self).borrow_mut()
    }

    #[inline]
    fn pos(&self) -> Self::Pos {
        (**self).pos()
    }

    #[inline]
    fn distance_from(&self, pos: Self::Pos) -> usize {
        (**self).distance_from(pos)
    }

    #[inline]
    fn remaining(&self) -> usize {
        (**self).remaining()
    }

    #[inline]
    fn skip(&mut self, size: usize) -> Result<(), Error> {
        (**self).skip(size)
    }

    #[inline]
    fn split(&mut self, at: usize) -> Option<Self::Split> {
        (**self).split(at)
    }

    #[inline]
    fn peek_words_uninit(&self, out: &mut [MaybeUninit<u64>]) -> Result<(), Error> {
        (**self).peek_words_uninit(out)
    }

    #[inline]
    fn read_words_uninit(&mut self, out: &mut [MaybeUninit<u64>]) -> Result<(), Error> {
        (**self).read_words_uninit(out)
    }

    #[inline]
    fn read_bytes<V>(&mut self, len: usize, visitor: V) -> Result<V::Ok, Error>
    where
        V: Visitor<'de, [u8]>,
    {
        (**self).read_bytes(len, visitor)
    }

    #[inline]
    fn as_bytes(&self) -> &[u8] {
        (**self).as_bytes()
    }

    #[inline]
    fn bytes_len(&self) -> usize {
        (**self).bytes_len()
    }

    #[inline]
    fn as_slice(&self) -> &'de [u64] {
        (**self).as_slice()
    }
}

/// A stored slice position.
pub struct SlicePos<'de> {
    ptr: *const u64,
    _marker: core::marker::PhantomData<&'de u64>,
}

impl Clone for SlicePos<'_> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl Copy for SlicePos<'_> {}

impl<'de> Reader<'de> for &'de [u64] {
    /// The type of a split off reader.
    type Split = &'de [u64];

    type Mut<'this>
        = &'this mut &'de [u64]
    where
        Self: 'this;

    type Pos = SlicePos<'de>;

    #[inline]
    fn borrow_mut(&mut self) -> Self::Mut<'_> {
        self
    }

    #[inline]
    fn pos(&self) -> Self::Pos {
        SlicePos {
            ptr: self.as_ptr(),
            _marker: PhantomData,
        }
    }

    #[inline]
    fn distance_from(&self, pos: Self::Pos) -> usize {
        // SAFETY: In principle, the stored position includes the lifetime of
        // `'de` which should prevent the buffer from being invalidated.
        let offset = unsafe { self.as_ptr().offset_from_unsigned(pos.ptr) };
        offset.wrapping_mul(mem::size_of::<u64>())
    }

    #[inline]
    fn remaining(&self) -> usize {
        self.len() * mem::size_of::<u64>()
    }

    #[inline]
    fn skip(&mut self, size: usize) -> Result<(), Error> {
        let size = size.div_ceil(mem::size_of::<u64>());

        let Some((_, tail)) = self.split_at_checked(size) else {
            return Err(Error::new(ErrorKind::BufferUnderflow));
        };

        *self = tail;
        Ok(())
    }

    #[inline]
    fn split(&mut self, at: usize) -> Option<Self::Split> {
        let at = at.div_ceil(mem::size_of::<u64>());
        let (head, tail) = self.split_at_checked(at)?;
        *self = tail;
        Some(head)
    }

    #[inline]
    fn peek_words_uninit(&self, out: &mut [MaybeUninit<u64>]) -> Result<(), Error> {
        if out.len() > self.len() {
            return Err(Error::new(ErrorKind::BufferUnderflow));
        }

        // SAFETY: The start pointer is valid since it hasn't reached the end yet.
        unsafe {
            self.as_ptr()
                .cast::<MaybeUninit<u64>>()
                .copy_to_nonoverlapping(out.as_mut_ptr(), out.len());
        }

        Ok(())
    }

    #[inline]
    fn read_words_uninit(&mut self, out: &mut [MaybeUninit<u64>]) -> Result<(), Error> {
        if out.len() > self.len() {
            return Err(Error::new(ErrorKind::BufferUnderflow));
        }

        // SAFETY: The start pointer is valid since it hasn't reached the end yet.
        unsafe {
            self.as_ptr()
                .cast::<MaybeUninit<u64>>()
                .copy_to_nonoverlapping(out.as_mut_ptr(), out.len());
        }

        *self = &self[out.len()..];
        Ok(())
    }

    #[inline]
    fn read_bytes<V>(&mut self, len: usize, visitor: V) -> Result<V::Ok, Error>
    where
        V: Visitor<'de, [u8]>,
    {
        let req = len.div_ceil(mem::size_of::<u64>());

        let Some((head, tail)) = self.split_at_checked(req) else {
            return Err(Error::new(ErrorKind::BufferUnderflow));
        };

        // SAFETY: The head is guaranteed to be valid since it was split from the original slice.
        let value = unsafe { slice::from_raw_parts(head.as_ptr().cast::<u8>(), len) };
        let ok = visitor.visit_borrowed(value)?;
        *self = tail;
        Ok(ok)
    }

    #[inline]
    fn as_bytes(&self) -> &[u8] {
        // SAFETY: The slice is guaranteed to be valid since it was created from
        // a slice of words.
        unsafe { slice::from_raw_parts(self.as_ptr().cast::<u8>(), self.bytes_len()) }
    }

    #[inline]
    fn bytes_len(&self) -> usize {
        self.len().wrapping_mul(mem::size_of::<u64>())
    }

    #[inline]
    fn as_slice(&self) -> &'de [u64] {
        self
    }
}
