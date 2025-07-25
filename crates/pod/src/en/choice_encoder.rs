use crate::error::ErrorKind;
use crate::pod::{ChildPod, PodKind};
use crate::{Choice, Error, Pod, Type, WORD_SIZE, Writer};

/// An encoder for a choice.
#[must_use = "Choice encoders must be closed to ensure all elements are initialized"]
pub struct ChoiceEncoder<W, K>
where
    W: Writer,
{
    writer: W,
    kind: K,
    header: W::Pos,
    #[allow(unused)]
    choice: Choice,
    #[allow(unused)]
    flags: u32,
    child_size: u32,
    child_type: Type,
}

impl<W, K> ChoiceEncoder<W, K>
where
    W: Writer,
    K: PodKind,
{
    #[inline]
    pub(crate) fn to_writer(
        mut writer: W,
        kind: K,
        choice: Choice,
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
    /// use pod::{Pod, Type, Choice};
    ///
    /// let mut pod = Pod::array();
    /// let mut choice = pod.encode_choice(Choice::NONE, Type::INT)?;
    ///
    /// choice.entry()?.encode(1i32)?;
    ///
    /// choice.close()?;
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

    /// Close the sequence encoder.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Choice, Pod, Type};
    ///
    /// let mut pod = Pod::array();
    /// let mut choice = pod.encode_choice(Choice::NONE, Type::INT)?;
    /// choice.entry()?.encode(1i32)?;
    /// choice.close()?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn close(mut self) -> Result<(), Error> {
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
