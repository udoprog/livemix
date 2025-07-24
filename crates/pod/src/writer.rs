use crate::Error;
use crate::utils::{WordAligned, as_words};

mod sealed {
    use crate::ArrayBuf;
    use crate::Writer;

    pub trait Sealed {}
    impl<const N: usize> Sealed for ArrayBuf<N> {}
    impl<W> Sealed for &mut W where W: ?Sized + Writer {}
}

/// A type that can have PODs written to it.
pub trait Writer: self::sealed::Sealed {
    /// The mutable borrow of a writer.
    type Mut<'this>: Writer
    where
        Self: 'this;

    /// A position in the writer.
    type Pos: Copy;

    /// Borrow the current writer mutably.
    fn borrow_mut(&mut self) -> Self::Mut<'_>;

    /// Reserve space for the given number of words.
    fn reserve_words(&mut self, words: &[u32]) -> Result<Self::Pos, Error>;

    /// Get the distance from the given position to the current writer position
    /// in words.
    fn distance_from(&self, pos: Self::Pos) -> usize;

    /// Write `words` number of zeros the writer.
    fn write_zeros(&mut self, words: usize) -> Result<(), Error>;

    /// Write a slice of `u32` values to the writer.
    fn write_words(&mut self, words: &[u32]) -> Result<(), Error>;

    /// Write a slice of `u32` values to the writer at the given previously
    /// reserved `pos`.
    ///
    /// # Errors
    ///
    /// Returns an error if the given number of words written overflows the
    /// reserved space.
    fn write_words_at(&mut self, pos: Self::Pos, words: &[u32]) -> Result<(), Error>;

    /// Write a `u64` value to the writer.
    #[inline]
    fn write<T>(&mut self, value: &T) -> Result<(), Error>
    where
        T: WordAligned,
    {
        self.write_words(as_words(value))
    }

    /// Write bytes to the writer.
    fn write_bytes(&mut self, bytes: &[u8], pad: usize) -> Result<(), Error>;
}

impl<W> Writer for &mut W
where
    W: ?Sized + Writer,
{
    type Mut<'this>
        = W::Mut<'this>
    where
        Self: 'this;

    type Pos = W::Pos;

    #[inline]
    fn borrow_mut(&mut self) -> Self::Mut<'_> {
        (**self).borrow_mut()
    }

    #[inline]
    fn reserve_words(&mut self, words: &[u32]) -> Result<Self::Pos, Error> {
        (**self).reserve_words(words)
    }

    #[inline]
    fn distance_from(&self, pos: Self::Pos) -> usize {
        (**self).distance_from(pos)
    }

    #[inline]
    fn write_zeros(&mut self, words: usize) -> Result<(), Error> {
        (**self).write_zeros(words)
    }

    #[inline]
    fn write_words(&mut self, value: &[u32]) -> Result<(), Error> {
        (**self).write_words(value)
    }

    #[inline]
    fn write_words_at(&mut self, pos: Self::Pos, value: &[u32]) -> Result<(), Error> {
        (**self).write_words_at(pos, value)
    }

    #[inline]
    fn write_bytes(&mut self, bytes: &[u8], pad: usize) -> Result<(), Error> {
        (**self).write_bytes(bytes, pad)
    }
}
