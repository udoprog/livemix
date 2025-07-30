use core::mem;

use crate::{Error, Pod, PodKind, RawId, Type, Writer};

/// An encoder for an object.
pub struct ObjectBuilder<W, K>
where
    W: Writer<u64>,
{
    writer: W,
    kind: K,
    header: W::Pos,
    #[allow(unused)]
    object_type: u32,
    #[allow(unused)]
    object_id: u32,
}

impl<W, K> ObjectBuilder<W, K>
where
    W: Writer<u64>,
    K: PodKind,
{
    #[inline]
    pub(crate) fn to_writer(
        mut writer: W,
        kind: K,
        object_type: u32,
        object_id: u32,
    ) -> Result<Self, Error> {
        // Reserve space for the header of the struct which includes its size
        // that will be determined later.
        let header = writer.reserve([
            mem::size_of::<[u32; 2]>() as u32,
            Type::OBJECT.into_u32(),
            object_type,
            object_id,
        ])?;

        Ok(Self {
            writer,
            kind,
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
    /// pod.as_mut().push_object(10, 20, |obj| {
    ///     obj.property(1)?.push(1i32)?;
    ///     obj.property(2)?.push(2i32)?;
    ///     obj.property(3)?.push(3i32)?;
    ///     Ok(())
    /// })?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn property(&mut self, key: impl RawId) -> Result<Pod<W::Mut<'_>>, Error> {
        self.property_with_flags(key, 0)
    }

    /// Encode a property with flags into the object.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = Pod::array();
    /// pod.as_mut().push_object(10, 20, |obj| {
    ///     obj.property_with_flags(1, 0b1001)?.push(1i32)?;
    ///     obj.property_with_flags(2, 0b1001)?.push(2i32)?;
    ///     obj.property_with_flags(3, 0b1001)?.push(3i32)?;
    ///     Ok(())
    /// })?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn property_with_flags(
        &mut self,
        key: impl RawId,
        flags: u32,
    ) -> Result<Pod<W::Mut<'_>>, Error> {
        self.writer.write([key.into_id(), flags])?;
        Ok(Pod::new(self.writer.borrow_mut()))
    }

    #[inline]
    pub(crate) fn close(mut self) -> Result<(), Error> {
        let size = self
            .kind
            .check_size(Type::OBJECT, &self.writer, self.header)?;

        self.writer
            .write_at(self.header, [size, Type::OBJECT.into_u32()])?;

        Ok(())
    }
}
