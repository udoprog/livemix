use crate::Error;
use crate::utils::{Align, AlignableWith, BytesInhabited};

mod sealed {
    use crate::Array;
    use crate::Writer;

    pub trait Sealed<T> {}
    impl<T, const N: usize> Sealed<T> for Array<T, N> {}
    impl<W, T> Sealed<T> for &mut W
    where
        W: ?Sized + Writer<T>,
        T: Copy,
    {
    }
}

/// A type that can have PODs written to it.
pub trait Writer<T>
where
    Self: self::sealed::Sealed<T>,
    T: Copy,
{
    /// The mutable borrow of a writer.
    type Mut<'this>: Writer<T>
    where
        Self: 'this;

    /// A position in the writer.
    type Pos: Copy;

    /// Borrow the current writer mutably.
    fn borrow_mut(&mut self) -> Self::Mut<'_>;

    /// Reserve space for the given number of words.
    fn reserve_words(&mut self, words: &[T]) -> Result<Self::Pos, Error>;

    /// Reserve space for a single value while writing.
    #[inline(always)]
    fn reserve(&mut self, value: impl AlignableWith<T>) -> Result<Self::Pos, Error> {
        self.reserve_words(Align(value).as_words())
    }

    /// Get the distance from the given position to the current writer position
    /// in bytes.
    fn distance_from(&self, pos: Self::Pos) -> Option<u32>;

    /// Write a slice of `u32` values to the writer.
    fn write_words(&mut self, words: &[T]) -> Result<(), Error>;

    /// Write a value to the writer.
    #[inline(always)]
    fn write(&mut self, value: impl AlignableWith<T>) -> Result<(), Error> {
        self.write_words(Align(value).as_words())
    }

    /// Write a slice of `u32` values to the writer at the given previously
    /// reserved `pos`.
    ///
    /// # Errors
    ///
    /// Returns an error if the given number of words written overflows the
    /// reserved space.
    fn write_words_at(&mut self, pos: Self::Pos, words: &[T]) -> Result<(), Error>;

    /// Write a value to the specified position in the writer.
    #[inline(always)]
    fn write_at(&mut self, pos: Self::Pos, value: impl AlignableWith<T>) -> Result<(), Error> {
        self.write_words_at(pos, Align(value).as_words())
    }

    /// Write bytes to the writer.
    fn write_bytes(&mut self, bytes: &[u8], pad: usize) -> Result<(), Error>
    where
        T: BytesInhabited;
}

impl<W, T> Writer<T> for &mut W
where
    W: ?Sized + Writer<T>,
    T: Copy,
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
    fn reserve_words(&mut self, words: &[T]) -> Result<Self::Pos, Error> {
        (**self).reserve_words(words)
    }

    #[inline]
    fn distance_from(&self, pos: Self::Pos) -> Option<u32> {
        (**self).distance_from(pos)
    }

    #[inline]
    fn write_words(&mut self, value: &[T]) -> Result<(), Error> {
        (**self).write_words(value)
    }

    #[inline]
    fn write_words_at(&mut self, pos: Self::Pos, value: &[T]) -> Result<(), Error> {
        (**self).write_words_at(pos, value)
    }

    #[inline]
    fn write_bytes(&mut self, bytes: &[u8], pad: usize) -> Result<(), Error>
    where
        T: BytesInhabited,
    {
        (**self).write_bytes(bytes, pad)
    }
}
