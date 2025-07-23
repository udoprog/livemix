use core::mem::MaybeUninit;
use core::slice;

use crate::utils::Align;
use crate::visitor::Visitor;
use crate::{Error, Type};

mod sealed {
    use crate::{ArrayBuf, Reader, Slice};

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
    fn peek_words_uninit(&self, out: &mut [MaybeUninit<u32>]) -> Result<(), Error>;

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

    /// Read an array of words.
    #[inline]
    fn peek_array<const N: usize>(&mut self) -> Result<[u32; N], Error> {
        let mut out = Align::<[u32; N], [_; N]>::uninit();
        self.peek_words_uninit(out.as_mut_slice())?;
        // SAFETY: The slice must have been initialized by the reader.
        Ok(unsafe { out.assume_init().read() })
    }

    /// Read an array of words.
    #[inline]
    fn array<const N: usize>(&mut self) -> Result<[u32; N], Error> {
        let mut out = Align::<[u32; N], [_; N]>::uninit();
        self.read_words_uninit(out.as_mut_slice())?;
        // SAFETY: The slice must have been initialized by the reader.
        Ok(unsafe { out.assume_init().read() })
    }

    #[inline]
    fn header(&mut self) -> Result<(u32, Type), Error> {
        let [size, ty] = self.array()?;
        let ty = Type::new(ty);
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

    #[inline]
    fn borrow_mut(&mut self) -> Self::Mut<'_> {
        (**self).borrow_mut()
    }

    #[inline]
    fn peek_words_uninit(&self, out: &mut [MaybeUninit<u32>]) -> Result<(), Error> {
        (**self).peek_words_uninit(out)
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
    fn peek_array<const N: usize>(&mut self) -> Result<[u32; N], Error> {
        (**self).peek_array()
    }

    #[inline]
    fn array<const N: usize>(&mut self) -> Result<[u32; N], Error> {
        (**self).array()
    }
}
