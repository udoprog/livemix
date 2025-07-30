use core::mem;

use crate::builder::{EnvelopePod, PodKind};
use crate::{Builder, Error, Type, Writer};

/// A control child for a sequence.
pub struct ControlChild {
    offset: u32,
    ty: u32,
}

impl ControlChild {
    #[inline]
    fn new() -> Self {
        Self { offset: 0, ty: 0 }
    }
}

impl<B> Builder<B, ControlChild> {
    /// Modify the offset of a control.
    pub fn offset(mut self, offset: u32) -> Self {
        self.kind.offset = offset;
        self
    }

    /// Modify the type of a control.
    pub fn ty(mut self, ty: u32) -> Self {
        self.kind.ty = ty;
        self
    }
}

impl crate::builder::builder::sealed::Sealed for ControlChild {}

impl PodKind for ControlChild {
    const ENVELOPE: bool = true;

    #[inline]
    fn header(&self, mut buf: impl Writer) -> Result<(), Error> {
        buf.write([self.offset, self.ty])
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

    #[inline]
    fn check(&self, _: Type, _: usize) -> Result<(), Error> {
        Ok(())
    }
}

/// An encoder for a sequence.
#[must_use = "Sequence encoders must be closed to ensure all elements are initialized"]
pub struct SequenceBuilder<W, K>
where
    W: Writer,
{
    writer: W,
    kind: K,
    header: W::Pos,
    unit: u32,
    pad: u32,
}

impl<W, K> SequenceBuilder<W, K>
where
    W: Writer,
    K: PodKind,
{
    #[inline]
    pub(crate) fn to_writer(mut writer: W, kind: K) -> Result<Self, Error> {
        // Reserve space for the header of the sequence which includes its size that will be determined later.
        let header = writer.reserve([
            mem::size_of::<[u32; 2]>() as u32,
            Type::SEQUENCE.into_u32(),
            0,
            0,
        ])?;

        Ok(Self {
            writer,
            kind,
            header,
            unit: 0,
            pad: 0,
        })
    }

    /// Write control into the sequence.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = pod::array();
    /// pod.as_mut().push_sequence(|seq| {
    ///     seq.control().push(1i32)?;
    ///     seq.control().push(2i32)?;
    ///     seq.control().push(3i32)?;
    ///     Ok(())
    /// })?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn control(&mut self) -> Builder<W::Mut<'_>, ControlChild> {
        Builder::new_with(self.writer.borrow_mut(), ControlChild::new())
    }

    #[inline]
    pub(crate) fn close(mut self) -> Result<(), Error> {
        let size = self
            .kind
            .check_size(Type::SEQUENCE, &self.writer, self.header)?;

        self.writer.write_at(
            self.header,
            [size, Type::SEQUENCE.into_u32(), self.unit, self.pad],
        )?;

        Ok(())
    }
}
