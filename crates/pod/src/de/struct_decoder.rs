use crate::error::ErrorKind;
use crate::{Error, Reader, TypedPod};

/// A decoder for a struct.
pub struct StructDecoder<R> {
    reader: R,
    size: u32,
}

impl<'de, R> StructDecoder<R>
where
    R: Reader<'de, u64>,
{
    #[inline]
    pub(crate) fn new(reader: R, size: u32) -> Self {
        Self { reader, size }
    }

    /// Test if the decoder is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::Pod;
    ///
    /// let mut pod = Pod::array();
    /// let mut st = pod.as_mut().encode_struct()?;
    /// st.field()?.encode(1i32)?;
    /// st.field()?.encode(2i32)?;
    /// st.field()?.encode(3i32)?;
    /// st.close()?;
    ///
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
    pub const fn is_empty(&self) -> bool {
        self.size == 0
    }

    /// Decode the next field in the struct.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::Pod;
    ///
    /// let mut pod = Pod::array();
    /// let mut st = pod.as_mut().encode_struct()?;
    /// st.field()?.encode(1i32)?;
    /// st.field()?.encode(2i32)?;
    /// st.field()?.encode(3i32)?;
    /// st.close()?;
    ///
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
    pub fn field(&mut self) -> Result<TypedPod<R::Clone<'_>>, Error> {
        if self.size == 0 {
            return Err(Error::new(ErrorKind::StructUnderflow));
        }

        let (size, ty) = self.reader.header()?;

        let head = self.reader.split(size)?;
        let pod = TypedPod::new(size, ty, head);

        let Some(size_with_header) = pod.size_with_header() else {
            return Err(Error::new(ErrorKind::SizeOverflow));
        };

        let Some(size) = self.size.checked_sub(size_with_header) else {
            return Err(Error::new(ErrorKind::SizeUnderflow {
                size: self.size,
                sub: size_with_header,
            }));
        };

        self.size = size;
        Ok(pod)
    }
}
