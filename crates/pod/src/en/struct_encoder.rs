use crate::error::ErrorKind;
use crate::{Error, Pod, Type, WORD_SIZE, Writer};

/// An encoder for a struct.
#[must_use = "Struct encoders must be closed to ensure all elements are initialized"]
pub struct StructEncoder<W>
where
    W: Writer,
{
    writer: W,
    header: W::Pos,
}

impl<W> StructEncoder<W>
where
    W: Writer,
{
    pub(crate) fn to_writer(mut writer: W) -> Result<Self, Error> {
        // Reserve space for the header of the struct which includes its size that will be determined later.
        let header = writer.reserve_words(&[0])?;
        Ok(Self { writer, header })
    }

    /// Add a field into the struct.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = Pod::array();
    /// let mut st = pod.encode_struct()?;
    ///
    /// st.field()?.encode(1i32)?;
    /// st.field()?.encode(2i32)?;
    /// st.field()?.encode(3i32)?;
    ///
    /// st.close()?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn field(&mut self) -> Result<Pod<W::Mut<'_>>, Error> {
        Ok(Pod::new(self.writer.borrow_mut()))
    }

    /// Close the struct encoder.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = Pod::array();
    /// let mut st = pod.encode_struct()?;
    ///
    /// st.field()?.encode(1i32)?;
    /// st.field()?.encode(2i32)?;
    /// st.field()?.encode(3i32)?;
    ///
    /// st.close()?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    pub fn close(mut self) -> Result<(), Error> {
        // Write the size of the struct at the header position.
        let Some(size) = self
            .writer
            .distance_from(self.header)
            .and_then(|v| v.checked_sub(WORD_SIZE))
        else {
            return Err(Error::new(ErrorKind::SizeOverflow));
        };

        self.writer
            .write_at(self.header, [size, Type::STRUCT.into_u32()])?;
        Ok(())
    }
}
