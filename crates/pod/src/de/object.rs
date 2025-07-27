use core::fmt;

#[cfg(feature = "alloc")]
use alloc::boxed::Box;

use crate::error::ErrorKind;
use crate::{AsReader, Error, Property, Reader, TypedPod, WORD_SIZE};

/// A decoder for a struct.
pub struct Object<B> {
    buf: B,
    size: u32,
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
}

impl<'de, B> Object<B>
where
    B: Reader<'de, u64>,
{
    #[inline]
    fn new(buf: B, size: u32, object_type: u32, object_id: u32) -> Self {
        Self {
            buf,
            size,
            object_type,
            object_id,
        }
    }

    #[inline]
    pub(crate) fn from_reader(mut reader: B, size: u32) -> Result<Self, Error> {
        let [object_type, object_id] = reader.read::<[u32; 2]>()?;

        // Remove the size of the object header.
        let Some(size) = size.checked_sub(WORD_SIZE) else {
            return Err(Error::new(ErrorKind::SizeUnderflow {
                size,
                sub: WORD_SIZE,
            }));
        };

        Ok(Self {
            buf: reader,
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
    /// pod.as_mut().encode_object(10, 20, |obj| {
    ///     obj.property(1, 10)?.encode(1i32)?;
    ///     obj.property(2, 20)?.encode(2i32)?;
    ///     obj.property(3, 30)?.encode(3i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let mut obj = pod.decode_object()?;
    /// assert!(!obj.is_empty());
    ///
    /// let p = obj.property()?;
    /// assert_eq!(p.key(), 1);
    /// assert_eq!(p.flags(), 10);
    /// assert_eq!(p.value().decode::<i32>()?, 1);
    ///
    /// let p = obj.property()?;
    /// assert_eq!(p.key(), 2);
    /// assert_eq!(p.flags(), 20);
    /// assert_eq!(p.value().decode::<i32>()?, 2);
    ///
    /// let p = obj.property()?;
    /// assert_eq!(p.key(), 3);
    /// assert_eq!(p.flags(), 30);
    /// assert_eq!(p.value().decode::<i32>()?, 3);
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
    /// pod.as_mut().encode_object(10, 20, |obj| {
    ///     obj.property(1, 10)?.encode(1i32)?;
    ///     obj.property(2, 20)?.encode(2i32)?;
    ///     obj.property(3, 30)?.encode(3i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let mut obj = pod.decode_object()?;
    /// assert!(!obj.is_empty());
    ///
    /// let p = obj.property()?;
    /// assert_eq!(p.key(), 1);
    /// assert_eq!(p.flags(), 10);
    /// assert_eq!(p.value().decode::<i32>()?, 1);
    ///
    /// let p = obj.property()?;
    /// assert_eq!(p.key(), 2);
    /// assert_eq!(p.flags(), 20);
    /// assert_eq!(p.value().decode::<i32>()?, 2);
    ///
    /// let p = obj.property()?;
    /// assert_eq!(p.key(), 3);
    /// assert_eq!(p.flags(), 30);
    /// assert_eq!(p.value().decode::<i32>()?, 3);
    ///
    /// assert!(obj.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn property(&mut self) -> Result<Property<B::Reader<'_>>, Error> {
        if self.size == 0 {
            return Err(Error::new(ErrorKind::ObjectUnderflow));
        }

        let [key, flags] = self.buf.read::<[u32; 2]>()?;
        let (size, ty) = self.buf.header()?;
        let pod = TypedPod::new(size, ty, self.buf.split(size)?);

        let Some(size_with_header) = pod
            .size_with_header()
            .and_then(|v| v.checked_add(WORD_SIZE))
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
    /// pod.as_mut().encode_object(10, 20, |obj| {
    ///     obj.property(1, 10)?.encode(1i32)?;
    ///     obj.property(2, 20)?.encode(2i32)?;
    ///     obj.property(3, 30)?.encode(3i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let obj = pod.decode_object()?.to_owned();
    ///
    /// let mut obj = obj.as_ref();
    /// assert!(!obj.is_empty());
    ///
    /// let p = obj.property()?;
    /// assert_eq!(p.key(), 1);
    /// assert_eq!(p.flags(), 10);
    /// assert_eq!(p.value().decode::<i32>()?, 1);
    ///
    /// let p = obj.property()?;
    /// assert_eq!(p.key(), 2);
    /// assert_eq!(p.flags(), 20);
    /// assert_eq!(p.value().decode::<i32>()?, 2);
    ///
    /// let p = obj.property()?;
    /// assert_eq!(p.key(), 3);
    /// assert_eq!(p.flags(), 30);
    /// assert_eq!(p.value().decode::<i32>()?, 3);
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
    /// pod.as_mut().encode_object(10, 20, |obj| {
    ///     obj.property(1, 10)?.encode(1i32)?;
    ///     obj.property(2, 20)?.encode(2i32)?;
    ///     obj.property(3, 30)?.encode(3i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let obj = pod.decode_object()?.to_owned();
    ///
    /// let mut obj = obj.as_ref();
    /// assert!(!obj.is_empty());
    ///
    /// let p = obj.property()?;
    /// assert_eq!(p.key(), 1);
    /// assert_eq!(p.flags(), 10);
    /// assert_eq!(p.value().decode::<i32>()?, 1);
    ///
    /// let p = obj.property()?;
    /// assert_eq!(p.key(), 2);
    /// assert_eq!(p.flags(), 20);
    /// assert_eq!(p.value().decode::<i32>()?, 2);
    ///
    /// let p = obj.property()?;
    /// assert_eq!(p.key(), 3);
    /// assert_eq!(p.flags(), 30);
    /// assert_eq!(p.value().decode::<i32>()?, 3);
    ///
    /// assert!(obj.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn as_ref(&self) -> Object<B::Reader<'_>> {
        Object::new(
            self.buf.as_reader(),
            self.size,
            self.object_type,
            self.object_id,
        )
    }
}

impl<'de, B> fmt::Debug for Object<B>
where
    B: AsReader<u64>,
{
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
