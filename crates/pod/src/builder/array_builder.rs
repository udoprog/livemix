use core::mem;

use crate::error::ErrorKind;
use crate::{BuildPod, Builder, ChildPod, Error, PADDING, Type, Writable, Writer};

/// An encoder for an array.
///
/// # Examples
///
/// ```
/// use pod::{Builder, Type};
///
/// let mut pod = Builder::array();
///
/// pod.as_mut().write_array(Type::INT, |array| {
///     array.child().write(1i32)?;
///     array.child().write(2i32)?;
///     array.child().write(3i32)?;
///     Ok(())
/// })?;
/// # Ok::<_, pod::Error>(())
/// ```
///
/// Encoding an array of unsized values:
///
/// ```
/// use pod::{Builder, Type};
///
/// let mut pod = Builder::array();
///
/// pod.write_unsized_array(Type::STRING, 4, |array| {
///     array.child().write_unsized("foo")?;
///     array.child().write_unsized("baz")?;
///     array.child().write_unsized("bar")?;
///     Ok(())
/// })?;
/// # Ok::<_, pod::Error>(())
/// ```
pub struct ArrayBuilder<W, P>
where
    W: Writer,
{
    writer: W,
    kind: P,
    header: W::Pos,
    child_size: usize,
    child_type: Type,
}

impl<W, P> ArrayBuilder<W, P>
where
    W: Writer,
    P: BuildPod,
{
    #[inline]
    pub(crate) fn to_writer(mut writer: W, kind: P, child_type: Type) -> Result<Self, Error> {
        let Some(child_size) = child_type.size() else {
            return Err(Error::new(ErrorKind::UnsizedTypeInArray { ty: child_type }));
        };

        let header = writer.reserve(&[
            mem::size_of::<[u32; 2]>() as u32,
            Type::ARRAY.into_u32(),
            child_size as u32,
            child_type.into_u32(),
        ])?;

        Ok(Self {
            writer,
            kind,
            header,
            child_size,
            child_type,
        })
    }

    #[inline]
    pub(crate) fn to_writer_unsized(
        mut writer: W,
        kind: P,
        child_size: usize,
        child_type: Type,
    ) -> Result<Self, Error> {
        if let Some(size) = child_type.size() {
            if size != child_size {
                return Err(Error::new(ErrorKind::ChildSizeMismatch {
                    actual: child_size,
                    expected: size,
                }));
            }
        };

        let header = {
            let Ok(child_size) = u32::try_from(child_size) else {
                return Err(Error::new(ErrorKind::SizeOverflow));
            };

            writer.reserve(&[
                mem::size_of::<[u32; 2]>() as u32,
                Type::ARRAY.into_u32(),
                child_size,
                child_type.into_u32(),
            ])?
        };

        Ok(Self {
            writer,
            kind,
            header,
            child_size,
            child_type,
        })
    }

    /// Write the given [`Writable`] to this [`ArrayBuilder`].
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ChoiceType, Builder, Type};
    ///
    /// let mut pod = Builder::array();
    /// pod.as_mut().write_array(Type::INT, |array| array.write((10, 0, 30)))?;
    ///
    /// let mut pod = pod.as_ref();
    /// let mut array = pod.read_array()?;
    /// assert_eq!(array.child_type(), Type::INT);
    /// assert_eq!(array.read::<(i32, u32, i32)>()?, (10, 0, 30));
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn write(&mut self, value: impl Writable) -> Result<(), Error> {
        let mut buf =
            Builder::new_child(self.writer.borrow_mut(), self.child_size, self.child_type);
        value.write_into(&mut buf)
    }

    /// Write control into the choice.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Builder, Type};
    ///
    /// let mut pod = Builder::array();
    /// pod.as_mut().write_array(Type::INT, |array| {
    ///     array.child().write(1i32)?;
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
            .check_size(Type::ARRAY, &self.writer, self.header)?;

        self.writer
            .write_at(self.header, &[size, Type::ARRAY.into_u32()])?;

        // Arrays are packed, so once we've finished writing all the items we
        // need to ensure it is correctly padded.
        self.writer.pad(PADDING)?;
        Ok(())
    }
}
