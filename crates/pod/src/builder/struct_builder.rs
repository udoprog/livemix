use crate::{EncodeInto, Error, Pod, PodKind, Type, Writer};

/// An encoder for a struct.
#[must_use = "Struct encoders must be closed to ensure all elements are initialized"]
pub struct StructBuilder<W, K>
where
    W: Writer<u64>,
{
    writer: W,
    kind: K,
    header: W::Pos,
}

impl<W, K> StructBuilder<W, K>
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

    /// Apply the given [`EncodeInto`] implementation to the contents of this
    /// struct.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = Pod::array();
    /// pod.as_mut().push_struct(|st| st.encode((1, 2, 3)))?;
    ///
    /// let mut pod = pod.as_ref();
    /// let mut st = pod.next_struct()?;
    /// assert_eq!(st.decode::<(i32, i32, i32)>()?, (1, 2, 3));
    /// assert!(st.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn encode(&mut self, value: impl EncodeInto) -> Result<(), Error> {
        value.encode_into(Pod::new(self.writer.borrow_mut()))
    }

    /// Add a field into the struct.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = Pod::array();
    /// pod.as_mut().push_struct(|st| {
    ///     st.field().push(1i32)?;
    ///     st.field().push(2i32)?;
    ///     st.field().push(3i32)?;
    ///     Ok(())
    /// })?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn field(&mut self) -> Pod<W::Mut<'_>> {
        Pod::new(self.writer.borrow_mut())
    }

    #[inline]
    pub(crate) fn close(mut self) -> Result<(), Error> {
        let size = self
            .kind
            .check_size(Type::STRUCT, &self.writer, self.header)?;

        self.writer
            .write_at(self.header, [size, Type::STRUCT.into_u32()])?;
        Ok(())
    }
}
