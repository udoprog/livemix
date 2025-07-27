use core::fmt;

use crate::error::ErrorKind;
use crate::{Error, Property, Reader, TypedPod, WORD_SIZE};

/// A decoder for a struct.
pub struct ObjectDecoder<R> {
    reader: R,
    size: u32,
    object_type: u32,
    object_id: u32,
}

impl<'de, R> ObjectDecoder<R>
where
    R: Reader<'de, u64>,
{
    #[inline]
    fn new(reader: R, size: u32, object_type: u32, object_id: u32) -> Self {
        Self {
            reader,
            size,
            object_type,
            object_id,
        }
    }

    #[inline]
    pub(crate) fn from_reader(mut reader: R, size: u32) -> Result<Self, Error> {
        let [object_type, object_id] = reader.read::<[u32; 2]>()?;

        // Remove the size of the object header.
        let Some(size) = size.checked_sub(WORD_SIZE) else {
            return Err(Error::new(ErrorKind::SizeUnderflow {
                size,
                sub: WORD_SIZE,
            }));
        };

        Ok(Self {
            reader,
            size,
            object_type,
            object_id,
        })
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
    pub fn property(&mut self) -> Result<Property<R::Clone<'_>>, Error> {
        if self.size == 0 {
            return Err(Error::new(ErrorKind::ObjectUnderflow));
        }

        let [key, flags] = self.reader.read::<[u32; 2]>()?;
        let (size, ty) = self.reader.header()?;
        let pod = TypedPod::new(size, ty, self.reader.split(size)?);

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

    /// Convert the [`ObjectDecoder`] into a one borrowing from but without
    /// modifying the current buffer.
    #[inline]
    pub fn as_ref(&self) -> ObjectDecoder<R::Clone<'_>> {
        ObjectDecoder::new(
            self.reader.clone_reader(),
            self.size,
            self.object_type,
            self.object_id,
        )
    }
}

impl<'de, R> fmt::Debug for ObjectDecoder<R>
where
    R: Reader<'de, u64>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct Properties<'a, R>(&'a ObjectDecoder<R>);

        impl<'de, R> fmt::Debug for Properties<'_, R>
        where
            R: Reader<'de, u64>,
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
        f.field("properties", &Properties(self));
        f.finish()
    }
}
