use core::mem::MaybeUninit;
use core::slice;

use crate::error::ErrorKind;
use crate::utils::{UninitAlign, WordSized};
use crate::visitor::Visitor;
use crate::{Error, Type, WORD_SIZE};

mod sealed {
    use crate::{ArrayBuf, Reader};

    pub trait Sealed {}
    impl Sealed for &[u64] {}
    impl<const N: usize> Sealed for ArrayBuf<N> {}
    impl<'de, R> Sealed for &mut R where R: ?Sized + Reader<'de> {}
}

/// A type that u32 words can be read from.
pub trait Reader<'de>: self::sealed::Sealed {
    /// The mutable borrow of a reader.
    type Mut<'this>: Reader<'de>
    where
        Self: 'this;

    /// A clone of the reader.
    type Clone<'this>: Reader<'this>
    where
        Self: 'this;

    /// Borrow the current reader mutably.
    fn borrow_mut(&mut self) -> Self::Mut<'_>;

    /// Clone the reader.
    fn clone_reader(&self) -> Self::Clone<'_>;

    /// Split off the head of the current buffer.
    fn split(&mut self, at: usize) -> Result<Self::Clone<'_>, Error>;

    /// Peek into the provided buffer without consuming the reader.
    fn peek_words_uninit(&self, out: &mut [MaybeUninit<u64>]) -> Result<(), Error>;

    /// Peek words into the provided buffer.
    fn read_words_uninit(&mut self, out: &mut [MaybeUninit<u64>]) -> Result<(), Error>;

    /// Read the given number of bytes from the input.
    fn read_bytes<V>(&mut self, len: usize, visitor: V) -> Result<V::Ok, Error>
    where
        V: Visitor<'de, [u8]>;

    /// Read an array of words.
    #[inline]
    fn peek<T>(&mut self) -> Result<T, Error>
    where
        T: WordSized,
    {
        let mut out = UninitAlign::<T>::uninit();
        self.peek_words_uninit(out.as_mut_slice())?;
        // SAFETY: The slice must have been initialized by the reader.
        Ok(unsafe { out.assume_init() })
    }

    /// Read an array of words.
    #[inline]
    fn read<T>(&mut self) -> Result<T, Error>
    where
        T: WordSized,
    {
        let mut out = UninitAlign::<T>::uninit();
        self.read_words_uninit(out.as_mut_slice())?;
        // SAFETY: The slice must have been initialized by the reader.
        Ok(unsafe { out.assume_init() })
    }

    #[inline]
    fn header(&mut self) -> Result<(u32, Type), Error> {
        let [size, ty] = self.read::<[u32; 2]>()?;
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

    type Clone<'this>
        = R::Clone<'this>
    where
        Self: 'this;

    #[inline]
    fn borrow_mut(&mut self) -> Self::Mut<'_> {
        (**self).borrow_mut()
    }

    #[inline]
    fn clone_reader(&self) -> Self::Clone<'_> {
        (**self).clone_reader()
    }

    #[inline]
    fn split(&mut self, at: usize) -> Result<Self::Clone<'_>, Error> {
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
}

impl<'de> Reader<'de> for &'de [u64] {
    type Mut<'this>
        = &'this mut &'de [u64]
    where
        Self: 'this;

    type Clone<'this>
        = &'this [u64]
    where
        Self: 'this;

    #[inline]
    fn borrow_mut(&mut self) -> Self::Mut<'_> {
        self
    }

    #[inline]
    fn clone_reader(&self) -> Self::Clone<'_> {
        *self
    }

    #[inline]
    fn split(&mut self, at: usize) -> Result<Self::Clone<'_>, Error> {
        let at = at.div_ceil(WORD_SIZE);

        let Some((head, tail)) = self.split_at_checked(at) else {
            return Err(Error::new(ErrorKind::BufferUnderflow));
        };

        *self = tail;
        Ok(head)
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
        let req = len.div_ceil(WORD_SIZE);

        let Some((head, tail)) = self.split_at_checked(req) else {
            return Err(Error::new(ErrorKind::BufferUnderflow));
        };

        let value = unsafe { slice::from_raw_parts(head.as_ptr().cast::<u8>(), len) };
        let ok = visitor.visit_borrowed(value)?;
        *self = tail;
        Ok(ok)
    }
}
