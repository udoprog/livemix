use core::mem;

use crate::{BuildPod, Builder, Error, PropertyPod, RawId, Type, Writer, WriterSlice};

/// An encoder for an object.
pub struct ObjectBuilder<W, P>
where
    W: Writer,
{
    writer: W,
    kind: P,
    pub(crate) header: W::Pos,
}

impl<W, P> ObjectBuilder<W, P>
where
    W: Writer,
    P: BuildPod,
{
    #[inline]
    pub(crate) fn to_writer(
        mut writer: W,
        kind: P,
        object_type: u32,
        object_id: u32,
    ) -> Result<Self, Error> {
        // Reserve space for the header of the struct which includes its size
        // that will be determined later.
        let header = writer.reserve(&[
            mem::size_of::<[u32; 2]>() as u32,
            Type::OBJECT.into_u32(),
            object_type,
            object_id,
        ])?;

        Ok(Self {
            writer,
            kind,
            header,
        })
    }

    /// Write a property into the object.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = pod::array();
    /// pod.as_mut().write_object(10, 20, |obj| {
    ///     obj.property(1).write(1i32)?;
    ///     obj.property(2).write(2i32)?;
    ///     obj.property(3).write(3i32)?;
    ///     Ok(())
    /// })?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    ///
    /// With flags:
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = pod::array();
    /// pod.as_mut().write_object(10, 20, |obj| {
    ///     obj.property(1).flags(0b1001).write(1i32)?;
    ///     obj.property(2).flags(0b1001).write(2i32)?;
    ///     obj.property(3).flags(0b1001).write(3i32)?;
    ///     Ok(())
    /// })?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn property<K>(&mut self, key: K) -> Builder<W::Mut<'_>, PropertyPod<K>>
    where
        K: RawId,
    {
        Builder::new_with(self.writer.borrow_mut(), PropertyPod::new(key))
    }

    #[inline]
    pub(crate) fn close(mut self) -> Result<WriterSlice<W, 16>, Error> {
        let size = self
            .kind
            .check_size(Type::OBJECT, &self.writer, self.header)?;

        self.writer
            .write_at(self.header, &[size, Type::OBJECT.into_u32()])?;

        Ok(WriterSlice::new(self.writer, self.header))
    }
}
