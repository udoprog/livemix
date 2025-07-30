use core::mem;

use crate::error::ErrorKind;
use crate::{Encode, EncodeUnsized, Error, RawId, Type, Writer};

use super::Builder;

mod sealed {
    use super::{ChildPod, ControlChild, EnvelopePod, PropertyChild};

    pub trait Sealed {}
    impl Sealed for EnvelopePod {}
    impl Sealed for ChildPod {}
    impl<K> Sealed for PropertyChild<K> {}
    impl Sealed for ControlChild {}
}

pub trait PodKind
where
    Self: self::sealed::Sealed,
{
    const ENVELOPE: bool;

    #[inline]
    fn header(&self, _: impl Writer) -> Result<(), Error> {
        Ok(())
    }

    fn push<T>(&self, value: T, buf: impl Writer) -> Result<(), Error>
    where
        T: Encode;

    fn push_unsized<T>(&self, value: &T, buf: impl Writer) -> Result<(), Error>
    where
        T: ?Sized + EncodeUnsized;

    #[inline]
    fn check(&self, _: Type, _: usize) -> Result<(), Error> {
        Ok(())
    }

    #[inline]
    fn check_size<W>(&self, ty: Type, writer: &W, header: W::Pos) -> Result<u32, Error>
    where
        W: ?Sized + Writer,
    {
        // This should always hold, since when we reserve space, we always
        // reserve space for the header, which is 64 bits wide.
        debug_assert!(writer.distance_from(header) >= mem::size_of::<[u32; 2]>());

        // Calculate the size of the struct at the header position.
        //
        // Every header is exactly 64-bits wide and this is not included in the
        // size of the objects, so we have to subtract it here.
        let size = writer
            .distance_from(header)
            .wrapping_sub(mem::size_of::<[u32; 2]>());

        self.check(ty, size)?;

        let Ok(size) = u32::try_from(size) else {
            return Err(Error::new(ErrorKind::SizeOverflow));
        };

        Ok(size)
    }
}

/// An unlimited pod.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct EnvelopePod;

impl PodKind for EnvelopePod {
    const ENVELOPE: bool = true;

    #[inline]
    fn push<T>(&self, value: T, mut buf: impl Writer) -> Result<(), Error>
    where
        T: Encode,
    {
        let Ok(size) = u32::try_from(T::SIZE) else {
            return Err(Error::new(ErrorKind::SizeOverflow));
        };

        buf.write(&[size, T::TYPE.into_u32()])?;
        value.write_content(buf)
    }

    #[inline]
    fn push_unsized<T>(&self, value: &T, mut buf: impl Writer) -> Result<(), Error>
    where
        T: ?Sized + EncodeUnsized,
    {
        let Ok(size) = u32::try_from(value.size()) else {
            return Err(Error::new(ErrorKind::SizeOverflow));
        };

        buf.write(&[size, T::TYPE.into_u32()])?;
        value.write_content(buf)
    }

    #[inline]
    fn check(&self, _: Type, _: usize) -> Result<(), Error> {
        Ok(())
    }
}

/// A pod limited for a specific child type and size.
#[derive(Clone, Copy, Debug)]
pub struct ChildPod {
    pub(crate) size: usize,
    pub(crate) ty: Type,
    pub(crate) padded: bool,
}

impl PodKind for ChildPod {
    const ENVELOPE: bool = false;

    #[inline]
    fn push<T>(&self, value: T, mut buf: impl Writer) -> Result<(), Error>
    where
        T: Encode,
    {
        self.check(T::TYPE, T::SIZE)?;
        value.write_content(buf.borrow_mut())?;

        if self.padded {
            buf.pad(8)?;
        }

        Ok(())
    }

    #[inline]
    fn push_unsized<T>(&self, value: &T, mut buf: impl Writer) -> Result<(), Error>
    where
        T: ?Sized + EncodeUnsized,
    {
        self.check(T::TYPE, value.size())?;
        value.write_content(buf.borrow_mut())?;

        if self.padded {
            buf.pad(8)?;
        }

        Ok(())
    }

    #[inline]
    fn check(&self, ty: Type, size: usize) -> Result<(), Error> {
        if self.ty != ty {
            return Err(Error::new(ErrorKind::Expected {
                expected: self.ty,
                actual: ty,
            }));
        }

        if size > self.size {
            return Err(Error::new(ErrorKind::ChildSizeMismatch {
                expected: self.size,
                actual: size,
            }));
        }

        Ok(())
    }
}

pub struct PropertyChild<K> {
    key: K,
    flags: u32,
}

impl<K> PropertyChild<K> {
    #[inline]
    pub(crate) fn new(key: K) -> Self {
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

impl<K> PodKind for PropertyChild<K>
where
    K: RawId,
{
    const ENVELOPE: bool = true;

    #[inline]
    fn header(&self, mut buf: impl Writer) -> Result<(), Error> {
        buf.write(&[self.key.into_id(), self.flags])
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

/// A control child for a sequence.
pub struct ControlChild {
    offset: u32,
    ty: u32,
}

impl ControlChild {
    #[inline]
    pub(crate) fn new() -> Self {
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

impl PodKind for ControlChild {
    const ENVELOPE: bool = true;

    #[inline]
    fn header(&self, mut buf: impl Writer) -> Result<(), Error> {
        buf.write(&[self.offset, self.ty])
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
