use crate::error::ErrorKind;
use crate::pod::PodKind;
use crate::{Error, Pod, Type, WORD_SIZE, Writer};

/// An encoder for a struct.
#[must_use = "Struct encoders must be closed to ensure all elements are initialized"]
pub struct StructEncoder<W, K>
where
    W: Writer<u64>,
{
    writer: W,
    kind: K,
    header: W::Pos,
}

impl<W, K> StructEncoder<W, K>
where
    W: Writer<u64>,
    K: PodKind,
{
    #[inline]
    pub(crate) fn to_writer(mut writer: W, kind: K) -> Result<Self, Error> {
        // Reserve space for the header of the struct which includes its size that will be determined later.
        let header = writer.reserve([0, Type::STRUCT.into_u32()])?;

        Ok(Self {
            writer,
            header,
            kind,
        })
    }

    /// Add a field into the struct.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = Pod::array();
    /// let mut st = pod.as_mut().encode_struct()?;
    /// st.field()?.encode(1i32)?;
    /// st.field()?.encode(2i32)?;
    /// st.field()?.encode(3i32)?;
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
    /// let mut st = pod.as_mut().encode_struct()?;
    /// st.field()?.encode(1i32)?;
    /// st.field()?.encode(2i32)?;
    /// st.field()?.encode(3i32)?;
    /// st.close()?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn close(mut self) -> Result<(), Error> {
        // Write the size of the struct at the header position.
        let Some(size) = self
            .writer
            .distance_from(self.header)
            .and_then(|v| v.checked_sub(WORD_SIZE))
        else {
            return Err(Error::new(ErrorKind::SizeOverflow));
        };

        self.kind.check(Type::STRUCT, size)?;

        self.writer
            .write_at(self.header, [size, Type::STRUCT.into_u32()])?;
        Ok(())
    }
}
