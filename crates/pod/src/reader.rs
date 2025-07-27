use core::mem::{self, MaybeUninit};
use core::slice;

use crate::error::ErrorKind;
use crate::utils::{AlignableWith, BytesInhabited, UninitAlign};
use crate::{AsReader, Error, Type, Visitor};

mod sealed {
    use crate::{Buf, Reader};

    pub trait Sealed<T> {}

    impl<T> Sealed<T> for &[T] {}
    impl<T, const N: usize> Sealed<T> for Buf<T, N> {}
    impl<'de, R, T> Sealed<T> for &mut R where R: ?Sized + Reader<'de, T> {}
}

/// A type that u32 words can be read from.
pub trait Reader<'de, T>
where
    Self: AsReader<T> + self::sealed::Sealed<T>,
{
    /// The mutable borrow of a reader.
    type Mut<'this>: Reader<'de, T>
    where
        Self: 'this;

    /// Borrow the current reader mutably.
    fn borrow_mut(&mut self) -> Self::Mut<'_>;

    /// Returns the size of the remaining buffer in bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Buf, Reader};
    ///
    /// let mut array = Buf::from_array([1u32, 2, 3]);
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
    fn remaining_bytes(&self) -> usize;

    /// Skip the given number of bytes.
    fn skip(&mut self, size: u32) -> Result<(), Error>;

    /// Split off the head of the current buffer.
    fn split(&mut self, at: u32) -> Result<Self::Reader<'_>, Error>;

    /// Peek into the provided buffer without consuming the reader.
    fn peek_words_uninit(&self, out: &mut [MaybeUninit<T>]) -> Result<(), Error>;

    /// Peek words into the provided buffer.
    fn read_words_uninit(&mut self, out: &mut [MaybeUninit<T>]) -> Result<(), Error>;

    /// Read the given number of bytes from the input.
    fn read_bytes<V>(&mut self, len: u32, visitor: V) -> Result<V::Ok, Error>
    where
        T: BytesInhabited,
        V: Visitor<'de, [u8]>;

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
    fn as_bytes(&self) -> &[u8];

    /// Returns the slice of remaining data to be read.
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
    fn as_slice(&self) -> &[T];

    /// Read an array of words.
    #[inline]
    fn peek<U>(&mut self) -> Result<U, Error>
    where
        U: AlignableWith<T>,
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
        U: AlignableWith<T>,
    {
        let mut out = UninitAlign::<U>::uninit();
        self.read_words_uninit(out.as_mut_slice())?;
        // SAFETY: The slice must have been initialized by the reader.
        Ok(unsafe { out.assume_init() })
    }

    #[inline]
    fn header(&mut self) -> Result<(u32, Type), Error>
    where
        [u32; 2]: AlignableWith<T>,
    {
        let [size, ty] = self.read::<[u32; 2]>()?;
        let ty = Type::new(ty);
        Ok((size, ty))
    }
}

impl<'de, R, T> Reader<'de, T> for &mut R
where
    R: ?Sized + Reader<'de, T>,
{
    type Mut<'this>
        = R::Mut<'this>
    where
        Self: 'this;

    #[inline]
    fn borrow_mut(&mut self) -> Self::Mut<'_> {
        (**self).borrow_mut()
    }

    #[inline]
    fn remaining_bytes(&self) -> usize {
        (**self).remaining_bytes()
    }

    #[inline]
    fn skip(&mut self, size: u32) -> Result<(), Error> {
        (**self).skip(size)
    }

    #[inline]
    fn split(&mut self, at: u32) -> Result<Self::Reader<'_>, Error> {
        (**self).split(at)
    }

    #[inline]
    fn peek_words_uninit(&self, out: &mut [MaybeUninit<T>]) -> Result<(), Error> {
        (**self).peek_words_uninit(out)
    }

    #[inline]
    fn read_words_uninit(&mut self, out: &mut [MaybeUninit<T>]) -> Result<(), Error> {
        (**self).read_words_uninit(out)
    }

    #[inline]
    fn read_bytes<V>(&mut self, len: u32, visitor: V) -> Result<V::Ok, Error>
    where
        T: BytesInhabited,
        V: Visitor<'de, [u8]>,
    {
        (**self).read_bytes(len, visitor)
    }

    #[inline]
    fn as_bytes(&self) -> &[u8] {
        (**self).as_bytes()
    }

    #[inline]
    fn as_slice(&self) -> &[T] {
        (**self).as_slice()
    }
}

impl<'de, T> Reader<'de, T> for &'de [T]
where
    T: 'static,
{
    type Mut<'this>
        = &'this mut &'de [T]
    where
        Self: 'this;

    #[inline]
    fn borrow_mut(&mut self) -> Self::Mut<'_> {
        self
    }

    #[inline]
    fn remaining_bytes(&self) -> usize {
        self.len() * mem::size_of::<T>()
    }

    #[inline]
    fn skip(&mut self, size: u32) -> Result<(), Error> {
        let size = size.div_ceil(mem::size_of::<T>() as u32);

        let Ok(size) = usize::try_from(size) else {
            return Err(Error::new(ErrorKind::SizeOverflow));
        };

        let Some((_, tail)) = self.split_at_checked(size) else {
            return Err(Error::new(ErrorKind::BufferUnderflow));
        };

        *self = tail;
        Ok(())
    }

    #[inline]
    fn split(&mut self, at: u32) -> Result<Self::Reader<'_>, Error> {
        let Ok(at) = usize::try_from(at) else {
            return Err(Error::new(ErrorKind::SizeOverflow));
        };

        let at = at.div_ceil(mem::size_of::<T>());

        let Some((head, tail)) = self.split_at_checked(at) else {
            return Err(Error::new(ErrorKind::BufferUnderflow));
        };

        *self = tail;
        Ok(head)
    }

    #[inline]
    fn peek_words_uninit(&self, out: &mut [MaybeUninit<T>]) -> Result<(), Error> {
        if out.len() > self.len() {
            return Err(Error::new(ErrorKind::BufferUnderflow));
        }

        // SAFETY: The start pointer is valid since it hasn't reached the end yet.
        unsafe {
            self.as_ptr()
                .cast::<MaybeUninit<T>>()
                .copy_to_nonoverlapping(out.as_mut_ptr(), out.len());
        }

        Ok(())
    }

    #[inline]
    fn read_words_uninit(&mut self, out: &mut [MaybeUninit<T>]) -> Result<(), Error> {
        if out.len() > self.len() {
            return Err(Error::new(ErrorKind::BufferUnderflow));
        }

        // SAFETY: The start pointer is valid since it hasn't reached the end yet.
        unsafe {
            self.as_ptr()
                .cast::<MaybeUninit<T>>()
                .copy_to_nonoverlapping(out.as_mut_ptr(), out.len());
        }

        *self = &self[out.len()..];
        Ok(())
    }

    #[inline]
    fn read_bytes<V>(&mut self, len: u32, visitor: V) -> Result<V::Ok, Error>
    where
        V: Visitor<'de, [u8]>,
    {
        let Ok(len) = usize::try_from(len) else {
            return Err(Error::new(ErrorKind::SizeOverflow));
        };

        let req = len.div_ceil(mem::size_of::<T>());

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
        unsafe {
            slice::from_raw_parts(
                self.as_ptr().cast::<u8>(),
                self.len().wrapping_mul(mem::size_of::<T>()),
            )
        }
    }

    #[inline]
    fn as_slice(&self) -> &[T] {
        self
    }
}
