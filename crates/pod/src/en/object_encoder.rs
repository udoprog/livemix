use crate::error::ErrorKind;
use crate::pod::PodKind;
use crate::{Error, Pod, Type, WORD_SIZE, Writer};

/// An encoder for an object.
pub struct ObjectEncoder<W, K>
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

impl<W, K> ObjectEncoder<W, K>
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
        let header =
            writer.reserve([WORD_SIZE, Type::OBJECT.into_u32(), object_type, object_id])?;

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
    /// pod.as_mut().encode_object(10, 20, |obj| {
    ///     obj.property(1, 0)?.encode(1i32)?;
    ///     obj.property(2, 0)?.encode(2i32)?;
    ///     obj.property(3, 0)?.encode(3i32)?;
    ///     Ok(())
    /// })?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn property(&mut self, key: u32, flags: u32) -> Result<Pod<W::Mut<'_>>, Error> {
        self.writer.write([key, flags])?;
        Ok(Pod::new(self.writer.borrow_mut()))
    }

    #[inline]
    pub(crate) fn close(mut self) -> Result<(), Error> {
        // Write the size of the struct at the header position.
        let Some(size) = self
            .writer
            .distance_from(self.header)
            .and_then(|v| v.checked_sub(WORD_SIZE))
        else {
            return Err(Error::new(ErrorKind::SizeOverflow));
        };

        self.kind.check(Type::OBJECT, size)?;

        self.writer
            .write_at(self.header, [size, Type::OBJECT.into_u32()])?;

        Ok(())
    }
}
