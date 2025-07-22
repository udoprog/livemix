use core::mem::MaybeUninit;
use core::slice;

use super::Error;
use super::utils::Align;
use super::visitor::Visitor;

mod sealed {
    use super::super::{ArrayBuf, Reader, Slice};

    pub trait Sealed {}
    impl Sealed for &Slice {}
    impl<const N: usize> Sealed for ArrayBuf<N> {}
    impl<'de, R> Sealed for &mut R where R: ?Sized + Reader<'de> {}
}

/// A type that u32 words can be read from.
pub trait Reader<'de>: self::sealed::Sealed {
    /// The mutable borrow of a reader.
    type Mut<'this>: Reader<'de>
    where
        Self: 'this;

    /// Borrow the current reader mutably.
    fn borrow_mut(&mut self) -> Self::Mut<'_>;

    /// Peek into the provided buffer without consuming the reader.
    fn peek_uninit_words(&self, out: &mut [MaybeUninit<u32>]) -> Result<(), Error>;

    /// Peek words into the provided buffer.
    fn read_words_uninit(&mut self, out: &mut [MaybeUninit<u32>]) -> Result<(), Error>;

    /// Peek words into the provided buffer.
    #[inline]
    fn read_words(&mut self, out: &mut [u32]) -> Result<(), Error> {
        let base = out.as_mut_ptr();
        let len = out.len();
        // SAFETY: An initialized slice can always be treated as an uninitialized slice of the same length.
        let out = unsafe { slice::from_raw_parts_mut(base.cast(), len) };
        self.read_words_uninit(out)
    }

    /// Read a `u32` value from the reader.
    #[inline]
    fn read_u32(&mut self) -> Result<u32, Error> {
        let mut out = Align::<u32, [_; 1]>::uninit();
        self.read_words_uninit(out.as_mut_slice())?;
        let a = unsafe { out.assume_init().read() };
        Ok(a)
    }

    /// Read a `u64` value from the reader.
    fn read_u64(&mut self) -> Result<u64, Error> {
        let mut out = Align::<u64, [_; 2]>::uninit();
        self.read_words_uninit(out.as_mut_slice())?;
        // SAFETY: The slice is guaranteed to be 2 elements u64 long.
        Ok(unsafe { out.assume_init().read() })
    }

    /// Read the given number of bytes from the input.
    fn read_bytes<V>(&mut self, len: usize, visitor: V) -> Result<V::Ok, Error>
    where
        V: Visitor<'de, [u8]>;

    /// Skip the given number of words.
    fn skip(&mut self, size: usize) -> Result<(), Error>;
}

impl<'de, R> Reader<'de> for &mut R
where
    R: ?Sized + Reader<'de>,
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
    fn peek_uninit_words(&self, out: &mut [MaybeUninit<u32>]) -> Result<(), Error> {
        (**self).peek_uninit_words(out)
    }

    #[inline]
    fn read_words_uninit(&mut self, out: &mut [MaybeUninit<u32>]) -> Result<(), Error> {
        (**self).read_words_uninit(out)
    }

    #[inline]
    fn read_words(&mut self, out: &mut [u32]) -> Result<(), Error> {
        (**self).read_words(out)
    }

    #[inline]
    fn read_u32(&mut self) -> Result<u32, Error> {
        (**self).read_u32()
    }

    #[inline]
    fn read_u64(&mut self) -> Result<u64, Error> {
        (**self).read_u64()
    }

    #[inline]
    fn read_bytes<V>(&mut self, len: usize, visitor: V) -> Result<V::Ok, Error>
    where
        V: Visitor<'de, [u8]>,
    {
        (**self).read_bytes(len, visitor)
    }

    #[inline]
    fn skip(&mut self, size: usize) -> Result<(), Error> {
        (**self).skip(size)
    }
}
