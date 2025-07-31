use core::mem;

use crate::error::ErrorKind;
use crate::{Encode, EncodeUnsized, Error, PADDING, RawId, Reader, Type, Writer};

use super::Builder;

mod sealed {
    use super::{ChildPod, ControlPod, PackedPod, PaddedPod, PropertyPod};

    pub trait Sealed {}
    impl Sealed for PaddedPod {}
    impl Sealed for ChildPod {}
    impl<K> Sealed for PropertyPod<K> where K: Copy {}
    impl Sealed for ControlPod {}
    impl Sealed for PackedPod {}
}

pub trait BuildPod
where
    Self: Sized + self::sealed::Sealed,
{
    #[inline]
    fn header(&self, _: impl Writer) -> Result<(), Error> {
        Ok(())
    }

    fn push<T>(self, value: T, buf: impl Writer) -> Result<(), Error>
    where
        T: Encode;

    fn push_unsized<T>(self, value: &T, buf: impl Writer) -> Result<(), Error>
    where
        T: ?Sized + EncodeUnsized;

    #[inline]
    fn check(self, _: Type, _: usize) -> Result<(), Error> {
        Ok(())
    }

    #[inline]
    fn check_size<W>(self, ty: Type, writer: &W, header: W::Pos) -> Result<u32, Error>
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

/// A padded pod.
///
/// This is the default.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct PaddedPod;

impl BuildPod for PaddedPod {
    #[inline]
    fn push<T>(self, value: T, mut buf: impl Writer) -> Result<(), Error>
    where
        T: Encode,
    {
        let Ok(size) = u32::try_from(T::SIZE) else {
            return Err(Error::new(ErrorKind::SizeOverflow));
        };

        buf.write(&[size, T::TYPE.into_u32()])?;
        value.write_content(buf.borrow_mut())?;
        buf.pad(PADDING)?;
        Ok(())
    }

    #[inline]
    fn push_unsized<T>(self, value: &T, mut buf: impl Writer) -> Result<(), Error>
    where
        T: ?Sized + EncodeUnsized,
    {
        let Some(size) = value.size() else {
            return Err(Error::new(ErrorKind::SizeOverflow));
        };

        let Ok(size) = u32::try_from(size) else {
            return Err(Error::new(ErrorKind::SizeOverflow));
        };

        buf.write(&[size, T::TYPE.into_u32()])?;
        value.write_content(buf.borrow_mut())?;
        buf.pad(PADDING)?;
        Ok(())
    }

    #[inline]
    fn check(self, _: Type, _: usize) -> Result<(), Error> {
        Ok(())
    }
}

/// A pod limited for a specific child type and size.
#[derive(Clone, Copy, Debug)]
pub struct ChildPod {
    pub(crate) size: usize,
    pub(crate) ty: Type,
}

impl BuildPod for ChildPod {
    #[inline]
    fn push<T>(self, value: T, buf: impl Writer) -> Result<(), Error>
    where
        T: Encode,
    {
        self.check(T::TYPE, T::SIZE)?;
        value.write_content(buf)
    }

    #[inline]
    fn push_unsized<T>(self, value: &T, buf: impl Writer) -> Result<(), Error>
    where
        T: ?Sized + EncodeUnsized,
    {
        let Some(size) = value.size() else {
            return Err(Error::new(ErrorKind::SizeOverflow));
        };

        self.check(T::TYPE, size)?;
        value.write_content(buf)
    }

    #[inline]
    fn check(self, ty: Type, size: usize) -> Result<(), Error> {
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

#[derive(Clone, Copy)]
pub struct PropertyPod<K>
where
    K: Copy,
{
    key: K,
    flags: u32,
}

impl<K> PropertyPod<K>
where
    K: Copy,
{
    #[inline]
    pub(crate) fn new(key: K) -> Self {
        Self { key, flags: 0 }
    }
}

impl<B, K> Builder<B, PropertyPod<K>>
where
    K: RawId,
{
    /// Modify the flags of a property.
    pub fn flags(mut self, flags: u32) -> Self {
        self.as_kind_mut().flags = flags;
        self
    }
}

impl<K> BuildPod for PropertyPod<K>
where
    K: RawId,
{
    #[inline]
    fn header(&self, mut buf: impl Writer) -> Result<(), Error> {
        buf.write(&[self.key.into_id(), self.flags])
    }

    #[inline]
    fn push<T>(self, value: T, buf: impl Writer) -> Result<(), Error>
    where
        T: crate::Encode,
    {
        PaddedPod.push(value, buf)
    }

    #[inline]
    fn push_unsized<T>(self, value: &T, buf: impl Writer) -> Result<(), Error>
    where
        T: ?Sized + crate::EncodeUnsized,
    {
        PaddedPod.push_unsized(value, buf)
    }
}

/// A control child for a sequence.
#[derive(Debug)]
pub struct ControlPod {
    offset: u32,
    ty: u32,
}

impl ControlPod {
    #[inline]
    pub(crate) fn new() -> Self {
        Self { offset: 0, ty: 0 }
    }
}

impl<B> Builder<B, ControlPod> {
    /// Modify the offset of a control.
    pub fn offset(mut self, offset: u32) -> Self {
        self.as_kind_mut().offset = offset;
        self
    }

    /// Modify the type of a control.
    pub fn ty(mut self, ty: u32) -> Self {
        self.as_kind_mut().ty = ty;
        self
    }
}

impl BuildPod for ControlPod {
    #[inline]
    fn header(&self, mut buf: impl Writer) -> Result<(), Error> {
        buf.write(&[self.offset, self.ty])
    }

    #[inline]
    fn push<T>(self, value: T, buf: impl Writer) -> Result<(), Error>
    where
        T: crate::Encode,
    {
        PaddedPod.push(value, buf)
    }

    #[inline]
    fn push_unsized<T>(self, value: &T, buf: impl Writer) -> Result<(), Error>
    where
        T: ?Sized + crate::EncodeUnsized,
    {
        PaddedPod.push_unsized(value, buf)
    }

    #[inline]
    fn check(self, _: Type, _: usize) -> Result<(), Error> {
        Ok(())
    }
}

/// Trait for specializing a pod for reading.
pub trait ReadPod
where
    Self: Copy + self::sealed::Sealed,
{
    #[doc(hidden)]
    fn unpad<'de>(&self, buf: impl Reader<'de>) -> Result<(), Error>;
}

/// A packed pod. This is used when unpacking packed fields, like those of
/// choices or arrays.
#[derive(Clone, Copy)]
#[non_exhaustive]
pub struct PackedPod;

impl ReadPod for PaddedPod {
    #[inline]
    fn unpad<'de>(&self, mut buf: impl Reader<'de>) -> Result<(), Error> {
        buf.unpad(PADDING)
    }
}

impl ReadPod for PackedPod {
    #[inline]
    fn unpad<'de>(&self, _: impl Reader<'de>) -> Result<(), Error> {
        Ok(())
    }
}
