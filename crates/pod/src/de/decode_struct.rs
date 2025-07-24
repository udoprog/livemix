use crate::error::ErrorKind;
use crate::{Error, Reader, TypedPod};

/// A decoder for a struct.
pub struct DecodeStruct<R> {
    reader: R,
    size: u32,
}

impl<'de, R> DecodeStruct<R>
where
    R: Reader<'de>,
{
    pub(crate) fn new(reader: R, size: u32) -> Self {
        Self { reader, size }
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
    /// st.add()?.encode(1i32)?;
    /// st.add()?.encode(2i32)?;
    /// st.add()?.encode(3i32)?;
    ///
    /// st.close()?;
    ///
    /// let pod = Pod::new(buf.as_slice());
    /// let mut st = pod.decode_struct()?;
    ///
    /// assert!(!st.is_empty());
    /// assert_eq!(st.next()?.decode::<i32>()?, 1i32);
    /// assert_eq!(st.next()?.decode::<i32>()?, 2i32);
    /// assert_eq!(st.next()?.decode::<i32>()?, 3i32);
    /// assert!(st.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    /// Decode the next field in the struct.
    #[inline]
    pub fn next(&mut self) -> Result<TypedPod<R::Clone<'_>>, Error> {
        if self.size == 0 {
            return Err(Error::new(ErrorKind::StructUnderflow));
        }

        let (size, ty) = self.reader.header()?;

        let Ok(split_size) = usize::try_from(size) else {
            return Err(Error::new(ErrorKind::SizeOverflow));
        };

        let head = self.reader.split(split_size)?;

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
