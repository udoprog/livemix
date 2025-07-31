use core::mem::MaybeUninit;

use crate::error::ErrorKind;
use crate::utils::UninitAlign;
use crate::{AsSlice, Error, Slice, Type, Visitor};

mod sealed {
    use crate::{ArrayBuf, Reader, Slice};

    pub trait Sealed {}

    impl Sealed for Slice<'_> {}
    impl<const N: usize> Sealed for ArrayBuf<N> {}
    impl<'de, R> Sealed for &mut R where R: ?Sized + Reader<'de> {}
}

/// A type that u32 words can be read from.
pub trait Reader<'de>
where
    Self: AsSlice + self::sealed::Sealed,
{
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

    /// Skip the given number of bytes.
    fn skip(&mut self, size: usize) -> Result<(), Error>;

    /// Split off the head of the current buffer.
    fn split(&mut self, at: usize) -> Option<Slice<'de>>;

    /// Peek into the provided buffer without consuming the reader.
    fn peek_words_uninit(&self, out: &mut [MaybeUninit<u8>]) -> Result<(), Error>;

    /// Peek words into the provided buffer.
    fn read_words_uninit(&mut self, out: &mut [MaybeUninit<u8>]) -> Result<(), Error>;

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
    /// assert_eq!(buf.len(), 0);
    /// buf.write(&[42u64])?;
    /// let expected = 42u64.to_ne_bytes();
    /// assert_eq!(buf.as_bytes(), &expected[..]);
    /// # Ok::<_, pod::Error>(())
    /// ```
    fn as_bytes(&self) -> &[u8];

    /// Returns the length of the bytes in the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Reader};
    ///
    /// let array = ArrayBuf::<128>::from_slice(&[1u64, 2, 3])?;
    /// let mut buf = pod::buf::slice(array.as_bytes());
    ///
    /// assert_eq!(buf.len(), 24);
    /// assert_eq!(buf.read::<[u64; 1]>(), Ok([1]));
    /// assert_eq!(buf.len(), 16);
    /// assert_eq!(buf.read::<[u64; 2]>(), Ok([2, 3]));
    /// assert_eq!(buf.len(), 0);
    /// # Ok::<_, pod::Error>(())
    /// ```
    fn len(&self) -> usize;

    /// Test if the reader is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, AsSlice, Reader};
    ///
    /// fn is_empty(buf: impl AsSlice) -> bool {
    ///    buf.as_slice().is_empty()
    /// }
    ///
    /// let mut buf = ArrayBuf::<128>::new();
    ///
    /// assert!(is_empty(buf.as_bytes()));
    /// buf.extend_from_words(&[42u64])?;
    /// assert!(!is_empty(buf.as_bytes()));
    /// # Ok::<_, pod::Error>(())
    /// ```
    fn is_empty(&self) -> bool;

    /// Unpad the current reader by advancing the position to align with the
    /// specified `align`.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Slice, Reader};
    ///
    /// let mut buf = Slice::new(&[1u8, 2, 3, 0, 4]);
    /// assert_eq!(buf.read::<[u8; 3]>(), Ok([1, 2, 3]));
    ///
    /// buf.unpad(4)?;
    /// assert_eq!(buf.as_bytes(), &[4]);
    /// # Ok::<_, pod::Error>(())
    /// ```
    fn unpad(&mut self, align: usize) -> Result<(), Error>;

    /// Read an array of words.
    #[inline]
    fn peek<T>(&self) -> Result<T, Error>
    where
        T: Copy,
    {
        let mut out = UninitAlign::<T>::uninit();
        self.peek_words_uninit(out.as_mut_slice())?;
        // SAFETY: The slice must have been initialized by the reader.
        Ok(unsafe { out.assume_init() })
    }

    /// Read type `T` from the reader.
    #[inline]
    fn read<T>(&mut self) -> Result<T, Error>
    where
        T: Copy,
    {
        let mut out = UninitAlign::<T>::uninit();
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
    fn skip(&mut self, size: usize) -> Result<(), Error> {
        (**self).skip(size)
    }

    #[inline]
    fn split(&mut self, at: usize) -> Option<Slice<'de>> {
        (**self).split(at)
    }

    #[inline]
    fn peek_words_uninit(&self, out: &mut [MaybeUninit<u8>]) -> Result<(), Error> {
        (**self).peek_words_uninit(out)
    }

    #[inline]
    fn read_words_uninit(&mut self, out: &mut [MaybeUninit<u8>]) -> Result<(), Error> {
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
    fn len(&self) -> usize {
        (**self).len()
    }

    #[inline]
    fn is_empty(&self) -> bool {
        (**self).is_empty()
    }

    #[inline]
    fn unpad(&mut self, padding: usize) -> Result<(), Error> {
        (**self).unpad(padding)
    }
}
