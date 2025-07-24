use crate::error::ErrorKind;
use crate::{Error, Pod, Type, WORD_SIZE, Writer};

/// An encoder for an object.
#[must_use = "Object encoders must be closed to ensure all elements are initialized"]
pub struct ObjectEncoder<W>
where
    W: Writer,
{
    writer: W,
    header: W::Pos,
    object_type: u32,
    object_id: u32,
}

impl<W> ObjectEncoder<W>
where
    W: Writer,
{
    pub(crate) fn to_writer(
        mut writer: W,
        object_type: u32,
        object_id: u32,
    ) -> Result<Self, Error> {
        // Reserve space for the header of the struct which includes its size
        // that will be determined later.
        let header = writer.reserve_words(&[0, 0])?;

        Ok(Self {
            writer,
            header,
            object_type,
            object_id,
        })
    }

    /// Encode a property into the object.
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
    pub fn property(&mut self, key: u32, flags: u32) -> Result<Pod<W::Mut<'_>>, Error> {
        self.writer.write([key, flags])?;
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

        self.writer.write_at(
            self.header,
            [
                size,
                Type::OBJECT.into_u32(),
                self.object_type,
                self.object_id,
            ],
        )?;
        Ok(())
    }
}
