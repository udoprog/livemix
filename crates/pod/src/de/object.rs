use core::fmt;
use core::mem;

#[cfg(feature = "alloc")]
use alloc::boxed::Box;

use crate::error::ErrorKind;
use crate::{AsReader, Encode, Error, Property, Reader, Type, TypedPod, Writer};

/// A decoder for a struct.
pub struct Object<B> {
    buf: B,
    size: usize,
    object_type: u32,
    object_id: u32,
}

impl<B> Object<B> {
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
    B: Reader<'de, u64>,
{
    #[inline]
    fn new(buf: B, size: usize, object_type: u32, object_id: u32) -> Self {
        Self {
            buf,
            size,
            object_type,
            object_id,
        }
    }

    #[inline]
    pub(crate) fn from_reader(mut buf: B, size: usize) -> Result<Self, Error> {
        let [object_type, object_id] = buf.read::<[u32; 2]>()?;

        // Remove the size of the object header.
        let Some(size) = size.checked_sub(mem::size_of::<[u32; 2]>()) else {
            return Err(Error::new(ErrorKind::SizeUnderflow {
                size,
                sub: mem::size_of::<[u32; 2]>(),
            }));
        };

        Ok(Self {
            buf,
            size,
            object_type,
            object_id,
        })
    }

    /// Test if the decoder is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = Pod::array();
    /// pod.as_mut().push_object(10, 20, |obj| {
    ///     obj.property(1, 10)?.push(1i32)?;
    ///     obj.property(2, 20)?.push(2i32)?;
    ///     obj.property(3, 30)?.push(3i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let mut obj = pod.as_ref().next_object()?;
    /// assert!(!obj.is_empty());
    ///
    /// let p = obj.property()?;
    /// assert_eq!(p.key(), 1);
    /// assert_eq!(p.flags(), 10);
    /// assert_eq!(p.value().next::<i32>()?, 1);
    ///
    /// let p = obj.property()?;
    /// assert_eq!(p.key(), 2);
    /// assert_eq!(p.flags(), 20);
    /// assert_eq!(p.value().next::<i32>()?, 2);
    ///
    /// let p = obj.property()?;
    /// assert_eq!(p.key(), 3);
    /// assert_eq!(p.flags(), 30);
    /// assert_eq!(p.value().next::<i32>()?, 3);
    ///
    /// assert!(obj.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.size == 0
    }

    /// Decode the next field in the struct.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = Pod::array();
    /// pod.as_mut().push_object(10, 20, |obj| {
    ///     obj.property(1, 10)?.push(1i32)?;
    ///     obj.property(2, 20)?.push(2i32)?;
    ///     obj.property(3, 30)?.push(3i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let mut obj = pod.as_ref().next_object()?;
    /// assert!(!obj.is_empty());
    ///
    /// let p = obj.property()?;
    /// assert_eq!(p.key(), 1);
    /// assert_eq!(p.flags(), 10);
    /// assert_eq!(p.value().next::<i32>()?, 1);
    ///
    /// let p = obj.property()?;
    /// assert_eq!(p.key(), 2);
    /// assert_eq!(p.flags(), 20);
    /// assert_eq!(p.value().next::<i32>()?, 2);
    ///
    /// let p = obj.property()?;
    /// assert_eq!(p.key(), 3);
    /// assert_eq!(p.flags(), 30);
    /// assert_eq!(p.value().next::<i32>()?, 3);
    ///
    /// assert!(obj.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn property(&mut self) -> Result<Property<B::Split>, Error> {
        if self.size == 0 {
            return Err(Error::new(ErrorKind::ObjectUnderflow));
        }

        let [key, flags] = self.buf.read::<[u32; 2]>()?;
        let (size, ty) = self.buf.header()?;

        let Some(head) = self.buf.split(size) else {
            return Err(Error::new(ErrorKind::BufferUnderflow));
        };

        let pod = TypedPod::new(size, ty, head);

        let Some(size_with_header) = pod
            .size_with_header()
            .and_then(|v| v.checked_add(mem::size_of::<[u32; 2]>()))
        else {
            return Err(Error::new(ErrorKind::SizeOverflow));
        };

        let Some(size) = self.size.checked_sub(size_with_header) else {
            return Err(Error::new(ErrorKind::SizeUnderflow {
                size: self.size,
                sub: size_with_header,
            }));
        };

        self.size = size;
        Ok(Property::new(key, flags, pod))
    }

    /// Coerce into an owned [`Object`].
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = Pod::array();
    /// pod.as_mut().push_object(10, 20, |obj| {
    ///     obj.property(1, 10)?.push(1i32)?;
    ///     obj.property(2, 20)?.push(2i32)?;
    ///     obj.property(3, 30)?.push(3i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let obj = pod.as_ref().next_object()?.to_owned();
    ///
    /// let mut obj = obj.as_ref();
    /// assert!(!obj.is_empty());
    ///
    /// let p = obj.property()?;
    /// assert_eq!(p.key(), 1);
    /// assert_eq!(p.flags(), 10);
    /// assert_eq!(p.value().next::<i32>()?, 1);
    ///
    /// let p = obj.property()?;
    /// assert_eq!(p.key(), 2);
    /// assert_eq!(p.flags(), 20);
    /// assert_eq!(p.value().next::<i32>()?, 2);
    ///
    /// let p = obj.property()?;
    /// assert_eq!(p.key(), 3);
    /// assert_eq!(p.flags(), 30);
    /// assert_eq!(p.value().next::<i32>()?, 3);
    ///
    /// assert!(obj.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[cfg(feature = "alloc")]
    #[inline]
    pub fn to_owned(&self) -> Object<Box<[u64]>> {
        Object {
            buf: Box::from(self.buf.as_slice()),
            size: self.size,
            object_type: self.object_type,
            object_id: self.object_id,
        }
    }
}

impl<B> Object<B>
where
    B: AsReader<u64>,
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
    /// let mut pod = Pod::array();
    /// pod.as_mut().push_object(10, 20, |obj| {
    ///     obj.property(1, 10)?.push(1i32)?;
    ///     obj.property(2, 20)?.push(2i32)?;
    ///     obj.property(3, 30)?.push(3i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let obj = pod.as_ref().next_object()?.to_owned();
    ///
    /// let mut obj = obj.as_ref();
    /// assert!(!obj.is_empty());
    ///
    /// let p = obj.property()?;
    /// assert_eq!(p.key(), 1);
    /// assert_eq!(p.flags(), 10);
    /// assert_eq!(p.value().next::<i32>()?, 1);
    ///
    /// let p = obj.property()?;
    /// assert_eq!(p.key(), 2);
    /// assert_eq!(p.flags(), 20);
    /// assert_eq!(p.value().next::<i32>()?, 2);
    ///
    /// let p = obj.property()?;
    /// assert_eq!(p.key(), 3);
    /// assert_eq!(p.flags(), 30);
    /// assert_eq!(p.value().next::<i32>()?, 3);
    ///
    /// assert!(obj.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn as_ref(&self) -> Object<B::AsReader<'_>> {
        Object::new(
            self.buf.as_reader(),
            self.size,
            self.object_type,
            self.object_id,
        )
    }
}

/// [`Encode`] implementation for [`Object`].
///
/// # Examples
///
/// ```
/// use pod::{Pod, Type};
///
/// let mut pod = Pod::array();
/// pod.as_mut().push_object(10, 20, |obj| {
///     obj.property(1, 10)?.push(1i32)?;
///     obj.property(2, 20)?.push(2i32)?;
///     obj.property(3, 30)?.push(3i32)?;
///     Ok(())
/// })?;
///
/// let obj = pod.as_ref().next_object()?.to_owned();
///
/// let mut pod2 = Pod::array();
/// pod2.as_mut().push(obj)?;
///
/// let obj = pod2.as_ref().next_object()?;
///
/// let mut obj = obj.as_ref();
/// assert!(!obj.is_empty());
///
/// let p = obj.property()?;
/// assert_eq!(p.key(), 1);
/// assert_eq!(p.flags(), 10);
/// assert_eq!(p.value().next::<i32>()?, 1);
///
/// let p = obj.property()?;
/// assert_eq!(p.key(), 2);
/// assert_eq!(p.flags(), 20);
/// assert_eq!(p.value().next::<i32>()?, 2);
///
/// let p = obj.property()?;
/// assert_eq!(p.key(), 3);
/// assert_eq!(p.flags(), 30);
/// assert_eq!(p.value().next::<i32>()?, 3);
///
/// assert!(obj.is_empty());
/// # Ok::<_, pod::Error>(())
/// ```
impl<B> Encode for Object<B>
where
    B: AsReader<u64>,
{
    const TYPE: Type = Type::OBJECT;

    #[inline]
    fn size(&self) -> usize {
        let len = self.buf.as_reader().bytes_len();
        len.wrapping_add(mem::size_of::<[u32; 2]>())
    }

    #[inline]
    fn write_content(&self, mut writer: impl Writer<u64>) -> Result<(), Error> {
        writer.write([self.object_type, self.object_id])?;
        writer.write_words(self.buf.as_reader().as_slice())
    }
}

impl<'de, B> fmt::Debug for Object<B>
where
    B: AsReader<u64>,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct Properties<'a, B>(&'a Object<B>);

        impl<B> fmt::Debug for Properties<'_, B>
        where
            B: AsReader<u64>,
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
