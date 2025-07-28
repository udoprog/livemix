use core::mem;

use crate::error::ErrorKind;
use crate::pod::{ChildPod, PodKind};
use crate::{Error, Pod, Type, Writer};

/// An encoder for an array.
///
/// # Examples
///
/// ```
/// use pod::{Pod, Type};
///
/// let mut pod = Pod::array();
///
/// pod.as_mut().push_array(Type::INT, |array| {
///     array.child()?.push(1i32)?;
///     array.child()?.push(2i32)?;
///     array.child()?.push(3i32)?;
///     Ok(())
/// })?;
/// # Ok::<_, pod::Error>(())
/// ```
///
/// Encoding an array of unsized values:
///
/// ```
/// use pod::{Pod, Type};
///
/// let mut pod = Pod::array();
///
/// pod.push_unsized_array(Type::STRING, 4, |array| {
///     array.child()?.push_unsized("foo")?;
///     array.child()?.push_unsized("baz")?;
///     array.child()?.push_unsized("bar")?;
///     Ok(())
/// })?;
/// # Ok::<_, pod::Error>(())
/// ```
pub struct ArrayEncoder<W, K>
where
    W: Writer<u64>,
{
    writer: W,
    kind: K,
    header: W::Pos,
    child_size: usize,
    child_type: Type,
}

impl<W, K> ArrayEncoder<W, K>
where
    W: Writer<u64>,
    K: PodKind,
{
    #[inline]
    pub(crate) fn to_writer(mut writer: W, kind: K, child_type: Type) -> Result<Self, Error> {
        let Some(child_size) = child_type.size() else {
            return Err(Error::new(ErrorKind::UnsizedTypeInArray { ty: child_type }));
        };

        let header = writer.reserve([
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
        kind: K,
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

            writer.reserve([
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

    /// Write control into the choice.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = Pod::array();
    /// pod.as_mut().push_array(Type::INT, |array| {
    ///     array.child()?.push(1i32)?;
    ///     Ok(())
    /// })?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn child(&mut self) -> Result<Pod<W::Mut<'_>, ChildPod>, Error> {
        Ok(Pod::new_child(
            self.writer.borrow_mut(),
            self.child_size,
            self.child_type,
        ))
    }

    #[inline]
    pub(crate) fn close(mut self) -> Result<(), Error> {
        let size = self
            .kind
            .check_size(Type::ARRAY, &self.writer, self.header)?;

        self.writer
            .write_at(self.header, [size, Type::ARRAY.into_u32()])?;
        Ok(())
    }
}
