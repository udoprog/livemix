use core::mem;

use crate::builder::{EnvelopePod, PodKind};
use crate::{Builder, Error, RawId, Type, Writer};

pub struct PropertyChild<K> {
    key: K,
    flags: u32,
}

impl<K> PropertyChild<K> {
    #[inline]
    fn new(key: K) -> Self {
        Self { key, flags: 0 }
    }
}

impl<B, K> Builder<B, PropertyChild<K>> {
    /// Modify the flags of a property.
    pub fn flags(mut self, flags: u32) -> Self {
        self.kind.flags = flags;
        self
    }
}

impl<K> crate::builder::builder::sealed::Sealed for PropertyChild<K> {}

impl<K> PodKind for PropertyChild<K>
where
    K: RawId,
{
    const ENVELOPE: bool = true;

    #[inline]
    fn header(&self, mut buf: impl Writer) -> Result<(), Error> {
        buf.write([self.key.into_id(), self.flags])
    }

    #[inline]
    fn push<T>(&self, value: T, buf: impl Writer) -> Result<(), Error>
    where
        T: crate::Encode,
    {
        EnvelopePod.push(value, buf)
    }

    #[inline]
    fn push_unsized<T>(&self, value: &T, buf: impl Writer) -> Result<(), Error>
    where
        T: ?Sized + crate::EncodeUnsized,
    {
        EnvelopePod.push_unsized(value, buf)
    }
}

/// An encoder for an object.
pub struct ObjectBuilder<W, P>
where
    W: Writer,
{
    writer: W,
    kind: P,
    header: W::Pos,
    #[allow(unused)]
    object_type: u32,
    #[allow(unused)]
    object_id: u32,
}

impl<W, P> ObjectBuilder<W, P>
where
    W: Writer,
    P: PodKind,
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
    /// let mut pod = pod::array();
    /// pod.as_mut().push_object(10, 20, |obj| {
    ///     obj.property(1).push(1i32)?;
    ///     obj.property(2).push(2i32)?;
    ///     obj.property(3).push(3i32)?;
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
    /// pod.as_mut().push_object(10, 20, |obj| {
    ///     obj.property(1).flags(0b1001).push(1i32)?;
    ///     obj.property(2).flags(0b1001).push(2i32)?;
    ///     obj.property(3).flags(0b1001).push(3i32)?;
    ///     Ok(())
    /// })?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn property<K>(&mut self, key: K) -> Builder<W::Mut<'_>, PropertyChild<K>>
    where
        K: RawId,
    {
        Builder::new_with(self.writer.borrow_mut(), PropertyChild::new(key))
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
