use crate::error::ErrorKind;
use crate::{Error, Pod, Type, WORD_SIZE, Writer};

/// An encoder for a struct.
#[must_use = "Struct encoders must be closed to ensure all elements are initialized"]
pub struct EncodeStruct<W>
where
    W: Writer,
{
    writer: W,
    header: W::Pos,
}

impl<W> EncodeStruct<W>
where
    W: Writer,
{
    pub(crate) fn new(writer: W, header: W::Pos) -> Self {
        Self { writer, header }
    }

    /// Add a field into the struct.
    #[inline]
    pub fn add(&mut self) -> Result<Pod<W::Mut<'_>>, Error> {
        Ok(Pod::new(self.writer.borrow_mut()))
    }

    /// Close the struct encoder.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Pod, Type};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let pod = Pod::new(&mut buf);
    /// let mut st = pod.encode_struct()?;
    ///
    /// st.add()?.encode(1i32)?;
    /// st.add()?.encode(2i32)?;
    /// st.add()?.encode(3i32)?;
    ///
    /// st.close()?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    pub fn close(mut self) -> Result<(), Error> {
        // Write the size of the struct at the header position.
        let size = self.writer.distance_from(self.header) - WORD_SIZE;

        let Ok(size) = u32::try_from(size) else {
            return Err(Error::new(ErrorKind::SizeOverflow));
        };

        self.writer
            .write_at(self.header, [size, Type::STRUCT.into_u32()])?;
        Ok(())
    }
}
