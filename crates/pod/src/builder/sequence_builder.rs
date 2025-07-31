use core::mem;

use crate::{BuildPodKind, Builder, ControlChild, Error, Type, Writer};

/// An encoder for a sequence.
#[must_use = "Sequence encoders must be closed to ensure all elements are initialized"]
pub struct SequenceBuilder<W, P>
where
    W: Writer,
{
    writer: W,
    kind: P,
    header: W::Pos,
    unit: u32,
    pad: u32,
}

impl<W, P> SequenceBuilder<W, P>
where
    W: Writer,
    P: BuildPodKind,
{
    #[inline]
    pub(crate) fn to_writer(mut writer: W, kind: P) -> Result<Self, Error> {
        // Reserve space for the header of the sequence which includes its size that will be determined later.
        let header = writer.reserve(&[
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
            &[size, Type::SEQUENCE.into_u32(), self.unit, self.pad],
        )?;

        Ok(())
    }
}
