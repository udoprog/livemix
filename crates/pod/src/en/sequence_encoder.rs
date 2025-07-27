use crate::error::ErrorKind;
use crate::pod::PodKind;
use crate::{Error, Pod, Type, WORD_SIZE, Writer};

/// An encoder for a sequence.
#[must_use = "Sequence encoders must be closed to ensure all elements are initialized"]
pub struct SequenceEncoder<W, K>
where
    W: Writer<u64>,
{
    writer: W,
    kind: K,
    header: W::Pos,
    unit: u32,
    pad: u32,
}

impl<W, K> SequenceEncoder<W, K>
where
    W: Writer<u64>,
    K: PodKind,
{
    #[inline]
    pub(crate) fn to_writer(mut writer: W, kind: K) -> Result<Self, Error> {
        // Reserve space for the header of the sequence which includes its size that will be determined later.
        let header = writer.reserve([WORD_SIZE, Type::SEQUENCE.into_u32(), 0, 0])?;

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
    /// let mut pod = Pod::array();
    /// pod.as_mut().encode_sequence(|seq| {
    ///     seq.control(1, 10)?.push(1i32)?;
    ///     seq.control(2, 20)?.push(2i32)?;
    ///     seq.control(3, 30)?.push(3i32)?;
    ///     Ok(())
    /// })?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn control(&mut self, offset: u32, ty: u32) -> Result<Pod<W::Mut<'_>>, Error> {
        self.writer.write([offset, ty])?;
        Ok(Pod::new(self.writer.borrow_mut()))
    }

    #[inline]
    pub(crate) fn close(mut self) -> Result<(), Error> {
        let Some(size) = self
            .writer
            .distance_from(self.header)
            .and_then(|v| v.checked_sub(WORD_SIZE))
        else {
            return Err(Error::new(ErrorKind::SizeOverflow));
        };

        self.kind.check(Type::SEQUENCE, size)?;

        self.writer.write_at(
            self.header,
            [size, Type::SEQUENCE.into_u32(), self.unit, self.pad],
        )?;

        Ok(())
    }
}
