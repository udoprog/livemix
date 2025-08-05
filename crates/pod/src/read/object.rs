use core::fmt;
use core::mem;

#[cfg(feature = "alloc")]
use crate::buf::{AllocError, DynamicBuf};
use crate::error::ErrorKind;
use crate::{
    AsSlice, Error, PADDING, PodItem, PodStream, Property, Readable, Reader, SizedReadable, Slice,
    Type, TypedPod, UnsizedReadable, UnsizedWritable, Writer,
};

use super::Struct;

/// A decoder for a struct.
pub struct Object<B> {
    buf: B,
    object_type: u32,
    object_id: u32,
}

impl<B> Object<B> {
    #[inline]
    pub(crate) fn new(buf: B, object_type: u32, object_id: u32) -> Self {
        Self {
            buf,
            object_type,
            object_id,
        }
    }

    /// Get the type of the object.
    #[inline]
    pub const fn object_type(&self) -> u32 {
        self.object_type
    }

    /// Get the id of the object.
    #[inline]
    pub const fn object_id(&self) -> u32 {
        self.object_id
    }

    /// Get a reference to the underlying buffer.
    #[inline]
    pub fn as_buf(&self) -> &B {
        &self.buf
    }
}

impl<'de, B> Object<B>
where
    B: Reader<'de>,
{
    #[inline]
    pub(crate) fn from_reader(mut buf: B) -> Result<Self, Error> {
        let [object_type, object_id] = buf.read::<[u32; 2]>()?;

        Ok(Self {
            buf,
            object_type,
            object_id,
        })
    }

    /// Read a value from the [`Object`] using the [`Readable`] trait.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::Readable;
    ///
    /// #[derive(Readable)]
    /// #[pod(object(type = 10u32, id = 20u32))]
    /// struct Contents {
    ///     #[pod(property = 100u32)]
    ///     value: u32,
    /// }
    ///
    /// let mut pod = pod::array();
    /// let mut obj = pod.as_mut().embed_object(10u32, 20u32, |obj| {
    ///     obj.property(100u32).write(200)
    /// })?;
    ///
    /// let c = obj.as_ref().read::<Contents>()?;
    /// assert_eq!(c.value, 200);
    /// # Ok::<_, pod::Error>(())
    /// ```
    pub fn read<T>(mut self) -> Result<T, Error>
    where
        T: Readable<'de>,
    {
        T::read_from(&mut self)
    }

    /// Test if the decoder is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = pod::array();
    /// pod.as_mut().write_object(10, 20, |obj| {
    ///     obj.property(1).flags(0b001).write(1i32)?;
    ///     obj.property(2).flags(0b010).write(2i32)?;
    ///     obj.property(3).flags(0b100).write(3i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let mut obj = pod.as_ref().read_object()?;
    /// assert!(!obj.is_empty());
    ///
    /// let p = obj.property()?;
    /// assert_eq!(p.key(), 1);
    /// assert_eq!(p.flags(), 0b001);
    /// assert_eq!(p.value().read_sized::<i32>()?, 1);
    ///
    /// let p = obj.property()?;
    /// assert_eq!(p.key(), 2);
    /// assert_eq!(p.flags(), 0b010);
    /// assert_eq!(p.value().read_sized::<i32>()?, 2);
    ///
    /// let p = obj.property()?;
    /// assert_eq!(p.key(), 3);
    /// assert_eq!(p.flags(), 0b100);
    /// assert_eq!(p.value().read_sized::<i32>()?, 3);
    ///
    /// assert!(obj.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    /// Read the next field in the struct.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = pod::array();
    /// pod.as_mut().write_object(10, 20, |obj| {
    ///     obj.property(1).flags(0b001).write(1i32)?;
    ///     obj.property(2).flags(0b010).write(2i32)?;
    ///     obj.property(3).flags(0b100).write(3i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let mut obj = pod.as_ref().read_object()?;
    /// assert!(!obj.is_empty());
    ///
    /// let p = obj.property()?;
    /// assert_eq!(p.key(), 1);
    /// assert_eq!(p.flags(), 0b001);
    /// assert_eq!(p.value().read_sized::<i32>()?, 1);
    ///
    /// let p = obj.property()?;
    /// assert_eq!(p.key(), 2);
    /// assert_eq!(p.flags(), 0b010);
    /// assert_eq!(p.value().read_sized::<i32>()?, 2);
    ///
    /// let p = obj.property()?;
    /// assert_eq!(p.key(), 3);
    /// assert_eq!(p.flags(), 0b100);
    /// assert_eq!(p.value().read_sized::<i32>()?, 3);
    ///
    /// assert!(obj.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn property(&mut self) -> Result<Property<Slice<'de>>, Error> {
        if self.buf.is_empty() {
            return Err(Error::new(ErrorKind::ObjectUnderflow));
        }

        let [key, flags] = self.buf.read::<[u32; 2]>()?;
        let (size, ty) = self.buf.header()?;

        let Some(head) = self.buf.split(size) else {
            return Err(Error::new(ErrorKind::BufferUnderflow));
        };

        let pod = TypedPod::packed(head, size, ty);
        self.buf.unpad(PADDING)?;
        Ok(Property::new(key, flags, pod))
    }

    /// Coerce into an owned [`Object`].
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = pod::array();
    /// pod.as_mut().write_object(10, 20, |obj| {
    ///     obj.property(1).flags(0b001).write(1i32)?;
    ///     obj.property(2).flags(0b010).write(2i32)?;
    ///     obj.property(3).flags(0b100).write(3i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let obj = pod.as_ref().read_object()?.to_owned()?;
    ///
    /// let mut obj = obj.as_ref();
    /// assert!(!obj.is_empty());
    ///
    /// let p = obj.property()?;
    /// assert_eq!(p.key(), 1);
    /// assert_eq!(p.flags(), 0b001);
    /// assert_eq!(p.value().read_sized::<i32>()?, 1);
    ///
    /// let p = obj.property()?;
    /// assert_eq!(p.key(), 2);
    /// assert_eq!(p.flags(), 0b010);
    /// assert_eq!(p.value().read_sized::<i32>()?, 2);
    ///
    /// let p = obj.property()?;
    /// assert_eq!(p.key(), 3);
    /// assert_eq!(p.flags(), 0b100);
    /// assert_eq!(p.value().read_sized::<i32>()?, 3);
    ///
    /// assert!(obj.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[cfg(feature = "alloc")]
    #[inline]
    pub fn to_owned(&self) -> Result<Object<DynamicBuf>, AllocError> {
        Ok(Object {
            buf: DynamicBuf::from_slice(self.buf.as_bytes())?,
            object_type: self.object_type,
            object_id: self.object_id,
        })
    }
}

impl<B> Object<B>
where
    B: AsSlice,
{
    /// Coerce into a borrowed [`Object`].
    ///
    /// Decoding this object does not affect the original object.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = pod::array();
    /// pod.as_mut().write_object(10, 20, |obj| {
    ///     obj.property(1).flags(0b001).write(1i32)?;
    ///     obj.property(2).flags(0b010).write(2i32)?;
    ///     obj.property(3).flags(0b100).write(3i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let obj = pod.as_ref().read_object()?.to_owned()?;
    ///
    /// let mut obj = obj.as_ref();
    /// assert!(!obj.is_empty());
    ///
    /// let p = obj.property()?;
    /// assert_eq!(p.key(), 1);
    /// assert_eq!(p.flags(), 0b001);
    /// assert_eq!(p.value().read_sized::<i32>()?, 1);
    ///
    /// let p = obj.property()?;
    /// assert_eq!(p.key(), 2);
    /// assert_eq!(p.flags(), 0b010);
    /// assert_eq!(p.value().read_sized::<i32>()?, 2);
    ///
    /// let p = obj.property()?;
    /// assert_eq!(p.key(), 3);
    /// assert_eq!(p.flags(), 0b100);
    /// assert_eq!(p.value().read_sized::<i32>()?, 3);
    ///
    /// assert!(obj.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn as_ref(&self) -> Object<Slice<'_>> {
        Object::new(self.buf.as_slice(), self.object_type, self.object_id)
    }
}

/// [`UnsizedWritable`] implementation for [`Object`].
///
/// # Examples
///
/// ```
/// use pod::{Pod, Type};
///
/// let mut pod = pod::array();
/// pod.as_mut().write_object(10, 20, |obj| {
///     obj.property(1).flags(0b001).write(1i32)?;
///     obj.property(2).flags(0b010).write(2i32)?;
///     obj.property(3).flags(0b100).write(3i32)?;
///     Ok(())
/// })?;
///
/// let obj = pod.as_ref().read_object()?.to_owned()?;
///
/// let mut pod2 = pod::array();
/// pod2.as_mut().write(obj)?;
///
/// let obj = pod2.as_ref().read_object()?;
///
/// let mut obj = obj.as_ref();
/// assert!(!obj.is_empty());
///
/// let p = obj.property()?;
/// assert_eq!(p.key(), 1);
/// assert_eq!(p.flags(), 0b001);
/// assert_eq!(p.value().read_sized::<i32>()?, 1);
///
/// let p = obj.property()?;
/// assert_eq!(p.key(), 2);
/// assert_eq!(p.flags(), 0b010);
/// assert_eq!(p.value().read_sized::<i32>()?, 2);
///
/// let p = obj.property()?;
/// assert_eq!(p.key(), 3);
/// assert_eq!(p.flags(), 0b100);
/// assert_eq!(p.value().read_sized::<i32>()?, 3);
///
/// assert!(obj.is_empty());
/// # Ok::<_, pod::Error>(())
/// ```
impl<B> UnsizedWritable for Object<B>
where
    B: AsSlice,
{
    const TYPE: Type = Type::OBJECT;

    #[inline]
    fn size(&self) -> Option<usize> {
        let len = self.buf.as_slice().len();
        len.checked_add(mem::size_of::<[u32; 2]>())
    }

    #[inline]
    fn write_unsized(&self, mut writer: impl Writer) -> Result<(), Error> {
        writer.write(&[self.object_type, self.object_id])?;
        writer.write(self.buf.as_slice().as_bytes())
    }
}

crate::macros::encode_into_unsized!(impl [B] Object<B> where B: AsSlice);

impl<'de> PodItem<'de> for Object<Slice<'de>> {
    #[inline]
    fn read<T>(self) -> Result<T, Error>
    where
        T: Readable<'de>,
    {
        Err(Error::new(ErrorKind::ReadNotSupported { ty: Type::OBJECT }))
    }

    #[inline]
    fn read_sized<T>(self) -> Result<T, Error>
    where
        T: SizedReadable<'de>,
    {
        Err(Error::new(ErrorKind::ReadSizedNotSupported {
            ty: Type::OBJECT,
        }))
    }

    #[inline]
    fn read_unsized<T>(self) -> Result<&'de T, Error>
    where
        T: ?Sized + UnsizedReadable<'de>,
    {
        Err(Error::new(ErrorKind::ReadUnsizedNotSupported {
            ty: Type::OBJECT,
        }))
    }

    #[inline]
    fn read_struct(self) -> Result<Struct<Slice<'de>>, Error> {
        Err(Error::expected(Type::STRUCT, Type::OBJECT, self.buf.len()))
    }

    #[inline]
    fn read_object(self) -> Result<Object<Slice<'de>>, Error> {
        Ok(self)
    }

    #[inline]
    fn read_option(self) -> Result<Option<Self>, Error> {
        Ok(Some(self))
    }
}

/// Read from the [`Object`] as a [`PodStream`].
///
/// # Examples
///
/// ```
/// use pod::{Readable, Writable};
///
/// #[derive(Readable, Writable)]
/// #[pod(object(type = 10u32, id = 20u32))]
/// struct Contents {
///     #[pod(property = 100u32)]
///     value: u32,
/// }
///
/// let mut pod = pod::array();
/// pod.as_mut().write(Contents { value: 200 })?;
///
/// let c = pod.as_ref().read::<Contents>()?;
/// assert_eq!(c.value, 200);
/// # Ok::<_, pod::Error>(())
/// ```
impl<'de, B> PodStream<'de> for Object<B>
where
    B: Reader<'de>,
{
    type Item = Object<Slice<'de>>;

    #[inline]
    fn next(&mut self) -> Result<Self::Item, Error> {
        let Some(buf) = self.buf.split(self.buf.len()) else {
            return Err(Error::new(ErrorKind::BufferUnderflow));
        };

        Ok(Object::new(buf, self.object_type, self.object_id))
    }
}

impl<'de, B> fmt::Debug for Object<B>
where
    B: AsSlice,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct Properties<'a, B>(&'a Object<B>);

        impl<B> fmt::Debug for Properties<'_, B>
        where
            B: AsSlice,
        {
            #[inline]
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let mut this = self.0.as_ref();

                let mut f = f.debug_list();

                while !this.is_empty() {
                    match this.property() {
                        Ok(prop) => {
                            f.entry(&prop);
                        }
                        Err(e) => {
                            f.entry(&e);
                        }
                    }
                }

                f.finish()
            }
        }

        let mut f = f.debug_struct("Object");
        f.field("object_type", &self.object_type());
        f.field("object_id", &self.object_id());
        f.field("properties", &Properties(&self));
        f.finish()
    }
}
