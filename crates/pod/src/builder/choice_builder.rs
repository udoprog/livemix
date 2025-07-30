use core::mem;

use crate::builder::{ChildPod, PodKind};
use crate::error::ErrorKind;
use crate::{Builder, ChoiceType, Error, Type, Writer};

/// An encoder for a choice.
pub struct ChoiceBuilder<W, P>
where
    W: Writer,
{
    writer: W,
    kind: P,
    header: W::Pos,
    #[allow(unused)]
    choice: ChoiceType,
    #[allow(unused)]
    flags: u32,
    child_size: usize,
    child_type: Type,
}

impl<W, P> ChoiceBuilder<W, P>
where
    W: Writer,
    P: PodKind,
{
    #[inline]
    pub(crate) fn to_writer(
        mut writer: W,
        kind: P,
        choice: ChoiceType,
        child_type: Type,
    ) -> Result<Self, Error> {
        let Some(child_size) = child_type.size() else {
            return Err(Error::new(ErrorKind::UnsizedTypeInArray { ty: child_type }));
        };

        // Reserve space for the header of the choice which includes its size
        // that will be determined later.
        let header = {
            let Ok(child_size) = u32::try_from(child_size) else {
                return Err(Error::new(ErrorKind::SizeOverflow));
            };

            writer.reserve(&[
                mem::size_of::<[u32; 4]>() as u32,
                Type::CHOICE.into_u32(),
                choice.into_u32(),
                0,
                child_size,
                child_type.into_u32(),
            ])?
        };

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
    /// use pod::{ChoiceType, Builder, Type};
    ///
    /// let mut pod = Builder::array();
    /// pod.push_choice(ChoiceType::NONE, Type::INT, |choice| {
    ///     choice.child().push(1i32)?;
    ///     Ok(())
    /// })?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn child(&mut self) -> Builder<W::Mut<'_>, ChildPod> {
        Builder::new_child(self.writer.borrow_mut(), self.child_size, self.child_type)
    }

    #[inline]
    pub(crate) fn close(mut self) -> Result<(), Error> {
        let size = self
            .kind
            .check_size(Type::CHOICE, &self.writer, self.header)?;

        self.writer
            .write_at(self.header, &[size, Type::CHOICE.into_u32()])?;

        Ok(())
    }
}
