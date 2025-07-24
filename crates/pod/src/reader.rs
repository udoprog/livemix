use core::mem::MaybeUninit;

use crate::utils::{Align, WordAligned};
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

    /// Read the given number of bytes from the input.
    fn read_bytes<V>(&mut self, len: usize, visitor: V) -> Result<V::Ok, Error>
    where
        V: Visitor<'de, [u8]>;

    /// Read an array of words.
    #[inline]
    fn peek_array<const N: usize>(&mut self) -> Result<[u32; N], Error> {
        let mut out = Align::<[u32; N]>::uninit();
        self.peek_words_uninit(out.as_mut_slice())?;
        // SAFETY: The slice must have been initialized by the reader.
        Ok(unsafe { out.assume_init() })
    }

    /// Read an array of words.
    #[inline]
    fn array<const N: usize>(&mut self) -> Result<[u32; N], Error> {
        let mut out = Align::<[u32; N]>::uninit();
        self.read_words_uninit(out.as_mut_slice())?;
        // SAFETY: The slice must have been initialized by the reader.
        Ok(unsafe { out.assume_init() })
    }

    /// Read an array of words.
    #[inline]
    fn read<T>(&mut self) -> Result<T, Error>
    where
        T: WordAligned,
    {
        let mut out = Align::<T>::uninit();
        self.read_words_uninit(out.as_mut_slice())?;
        // SAFETY: The slice must have been initialized by the reader.
        Ok(unsafe { out.assume_init() })
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
