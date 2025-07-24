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
    R: Reader<'de>,
{
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
    pub fn object_type(&self) -> u32 {
        self.object_type
    }

    /// Get the id of the object.
    #[inline]
    pub fn object_id(&self) -> u32 {
        self.object_id
    }

    /// Test if the decoder is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Pod, TypedPod};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let pod = Pod::new(&mut buf);
    /// let mut st = pod.encode_struct()?;
    ///
    /// st.field()?.encode(1i32)?;
    /// st.field()?.encode(2i32)?;
    /// st.field()?.encode(3i32)?;
    ///
    /// st.close()?;
    ///
    /// let pod = Pod::new(buf.as_slice());
    /// let mut st = pod.decode_struct()?;
    ///
    /// assert!(!st.is_empty());
    /// assert_eq!(st.field()?.decode::<i32>()?, 1i32);
    /// assert_eq!(st.field()?.decode::<i32>()?, 2i32);
    /// assert_eq!(st.field()?.decode::<i32>()?, 3i32);
    /// assert!(st.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    /// Decode the next field in the struct.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Pod, TypedPod};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let pod = Pod::new(&mut buf);
    /// let mut st = pod.encode_struct()?;
    ///
    /// st.field()?.encode(1i32)?;
    /// st.field()?.encode(2i32)?;
    /// st.field()?.encode(3i32)?;
    ///
    /// st.close()?;
    ///
    /// let pod = Pod::new(buf.as_slice());
    /// let mut st = pod.decode_struct()?;
    ///
    /// assert!(!st.is_empty());
    /// assert_eq!(st.field()?.decode::<i32>()?, 1i32);
    /// assert_eq!(st.field()?.decode::<i32>()?, 2i32);
    /// assert_eq!(st.field()?.decode::<i32>()?, 3i32);
    /// assert!(st.is_empty());
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
}
