use super::Error;
use super::ty::Type;

mod sealed {
    use crate::ArrayBuf;
    use crate::Writer;

    pub trait Sealed {}
    impl<const N: usize> Sealed for ArrayBuf<N> {}
    impl<W> Sealed for &mut W where W: ?Sized + Writer {}
}

/// A type that can have PODs written to it.
pub trait Writer: self::sealed::Sealed {
    /// Write a slice of `u32` values to the writer.
    fn write_words(&mut self, value: &[u32]) -> Result<(), Error>;

    /// Write a `u32` value to the writer.
    #[inline]
    fn write_u32(&mut self, value: u32) -> Result<(), Error> {
        self.write_words(&[value])
    }

    /// Write a `u64` value to the writer.
    #[inline]
    fn write_u64(&mut self, value: u64) -> Result<(), Error> {
        let [a, b] = unsafe { (&value as *const u64).cast::<[u32; 2]>().read() };
        self.write_words(&[a, b])
    }

    /// Write a type to the writer.
    #[inline]
    fn write_type(&mut self, ty: Type) -> Result<(), Error> {
        self.write_u32(ty.into_u32())
    }

    /// Write bytes to the writer.
    #[inline]
    fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), Error> {
        for chunk in bytes.chunks(4) {
            let value = if let &[a, b, c, d] = chunk {
                u32::from_ne_bytes([a, b, c, d])
            } else {
                let mut array = [0u8; 4];
                array[..chunk.len()].copy_from_slice(chunk);
                u32::from_ne_bytes(array)
            };

            self.write_u32(value)?;
        }

        Ok(())
    }

    /// Pad the writer to the next double word boundary.
    fn pad(&mut self) -> Result<(), Error>;
}

impl<W> Writer for &mut W
where
    W: ?Sized + Writer,
{
    #[inline]
    fn write_words(&mut self, value: &[u32]) -> Result<(), Error> {
        (**self).write_words(value)
    }

    #[inline]
    fn write_u32(&mut self, value: u32) -> Result<(), Error> {
        (**self).write_u32(value)
    }

    #[inline]
    fn write_u64(&mut self, value: u64) -> Result<(), Error> {
        (**self).write_u64(value)
    }

    #[inline]
    fn write_type(&mut self, ty: Type) -> Result<(), Error> {
        (**self).write_type(ty)
    }

    #[inline]
    fn pad(&mut self) -> Result<(), Error> {
        (**self).pad()
    }
}
