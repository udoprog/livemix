use crate::{BuildPod, Builder, Error, Type, Writable, Writer};

/// An encoder for a struct.
#[must_use = "Struct encoders must be closed to ensure all elements are initialized"]
pub struct StructBuilder<W, P>
where
    W: Writer,
{
    writer: W,
    kind: P,
    header: W::Pos,
}

impl<W, P> StructBuilder<W, P>
where
    W: Writer,
    P: BuildPod,
{
    #[inline]
    pub(crate) fn to_writer(mut writer: W, kind: P) -> Result<Self, Error> {
        // Reserve space for the header of the struct which includes its size that will be determined later.
        let header = writer.reserve(&[0, Type::STRUCT.into_u32()])?;

        Ok(Self {
            writer,
            header,
            kind,
        })
    }

    /// Write the given [`Writable`] to this [`StructBuilder`].
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = pod::array();
    /// pod.as_mut().write_struct(|st| st.write((1, 2, 3)))?;
    ///
    /// let mut pod = pod.as_ref();
    /// let mut st = pod.read_struct()?;
    /// assert_eq!(st.read::<(i32, i32, i32)>()?, (1, 2, 3));
    /// assert!(st.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn write(&mut self, value: impl Writable) -> Result<(), Error> {
        let mut buf = Builder::new(self.writer.borrow_mut());
        value.write_into(&mut buf)
    }

    /// Add a field into the struct.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = pod::array();
    /// pod.as_mut().write_struct(|st| {
    ///     st.field().write(1i32)?;
    ///     st.field().write(2i32)?;
    ///     st.field().write(3i32)?;
    ///     Ok(())
    /// })?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn field(&mut self) -> Builder<W::Mut<'_>> {
        Builder::new(self.writer.borrow_mut())
    }

    #[inline]
    pub(crate) fn close(mut self) -> Result<(), Error> {
        let size = self
            .kind
            .check_size(Type::STRUCT, &self.writer, self.header)?;

        self.writer
            .write_at(self.header, &[size, Type::STRUCT.into_u32()])?;
        Ok(())
    }
}
