use crate::error::ErrorKind;
use crate::pod::{ChildPod, PodKind};
use crate::{ChoiceType, Error, Pod, Type, WORD_SIZE, Writer};

/// An encoder for a choice.
pub struct ChoiceEncoder<W, K>
where
    W: Writer<u64>,
{
    writer: W,
    kind: K,
    header: W::Pos,
    #[allow(unused)]
    choice: ChoiceType,
    #[allow(unused)]
    flags: u32,
    child_size: u32,
    child_type: Type,
}

impl<W, K> ChoiceEncoder<W, K>
where
    W: Writer<u64>,
    K: PodKind,
{
    #[inline]
    pub(crate) fn to_writer(
        mut writer: W,
        kind: K,
        choice: ChoiceType,
        child_type: Type,
    ) -> Result<Self, Error> {
        let Some(child_size) = child_type.size() else {
            return Err(Error::new(ErrorKind::UnsizedTypeInArray { ty: child_type }));
        };

        // Reserve space for the header of the choice which includes its size
        // that will be determined later.
        let header = writer.reserve([
            WORD_SIZE * 2,
            Type::CHOICE.into_u32(),
            choice.into_u32(),
            0,
            child_size,
            child_type.into_u32(),
        ])?;

        Ok(Self {
            writer,
            kind,
            header,
            choice,
            flags: 0,
            child_size,
            child_type,
        })
    }

    /// Write control into the choice.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ChoiceType, Pod, Type};
    ///
    /// let mut pod = Pod::array();
    /// pod.encode_choice(ChoiceType::NONE, Type::INT, |choice| {
    ///     choice.entry()?.encode(1i32)?;
    ///     Ok(())
    /// })?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn entry(&mut self) -> Result<Pod<W::Mut<'_>, ChildPod>, Error> {
        Ok(Pod::new_child(
            self.writer.borrow_mut(),
            self.child_size,
            self.child_type,
        ))
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

        self.kind.check(Type::CHOICE, size)?;

        self.writer
            .write_at(self.header, [size, Type::CHOICE.into_u32()])?;

        Ok(())
    }
}
