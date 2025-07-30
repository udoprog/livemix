use crate::Error;
use crate::utils::BytesInhabited;

mod sealed {
    #[cfg(feature = "alloc")]
    use crate::DynamicBuf;
    use crate::{ArrayBuf, Writer};

    pub trait Sealed {}
    impl<const N: usize> Sealed for ArrayBuf<N> {}
    #[cfg(feature = "alloc")]
    impl Sealed for DynamicBuf {}
    impl<W> Sealed for &mut W where W: ?Sized + Writer {}
}

/// A type that can have PODs written to it.
pub trait Writer
where
    Self: self::sealed::Sealed,
{
    /// The mutable borrow of a writer.
    type Mut<'this>: Writer
    where
        Self: 'this;

    /// A position in the writer.
    type Pos: Copy;

    /// Borrow the current writer mutably.
    fn borrow_mut(&mut self) -> Self::Mut<'_>;

    /// Reserve space for the given number of words.
    fn reserve<T>(&mut self, words: &[T]) -> Result<Self::Pos, Error>
    where
        T: BytesInhabited;

    /// Get the distance from the given position to the current writer position
    /// in bytes.
    fn distance_from(&self, pos: Self::Pos) -> usize;

    /// Write a slice of `u32` values to the writer.
    fn write<T>(&mut self, words: &[T]) -> Result<(), Error>
    where
        T: BytesInhabited;

    /// Write a slice of `u32` values to the writer at the given previously
    /// reserved `pos`.
    ///
    /// # Errors
    ///
    /// Returns an error if the given number of words written overflows the
    /// reserved space.
    fn write_at<T>(&mut self, pos: Self::Pos, words: &[T]) -> Result<(), Error>
    where
        T: BytesInhabited;

    /// Write bytes to the writer.
    fn write_bytes(&mut self, bytes: &[u8], pad: usize) -> Result<(), Error>;

    /// Pad the writer to the given alignment.
    fn pad(&mut self, align: usize) -> Result<(), Error>;
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
    fn reserve<T>(&mut self, words: &[T]) -> Result<Self::Pos, Error>
    where
        T: BytesInhabited,
    {
        (**self).reserve(words)
    }

    #[inline]
    fn distance_from(&self, pos: Self::Pos) -> usize {
        (**self).distance_from(pos)
    }

    #[inline]
    fn write<T>(&mut self, value: &[T]) -> Result<(), Error>
    where
        T: BytesInhabited,
    {
        (**self).write(value)
    }

    #[inline]
    fn write_at<T>(&mut self, pos: Self::Pos, value: &[T]) -> Result<(), Error>
    where
        T: BytesInhabited,
    {
        (**self).write_at(pos, value)
    }

    #[inline]
    fn write_bytes(&mut self, bytes: &[u8], pad: usize) -> Result<(), Error> {
        (**self).write_bytes(bytes, pad)
    }

    #[inline]
    fn pad(&mut self, align: usize) -> Result<(), Error> {
        (**self).pad(align)
    }
}
