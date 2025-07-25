use crate::error::ErrorKind;
use crate::{Control, Error, Reader, TypedPod, WORD_SIZE};

/// A decoder for a sequence.
pub struct SequenceDecoder<R> {
    reader: R,
    size: u32,
    unit: u32,
    pad: u32,
}

impl<'de, R> SequenceDecoder<R>
where
    R: Reader<'de>,
{
    #[inline]
    pub(crate) fn from_reader(mut reader: R, size: u32) -> Result<Self, Error> {
        let [unit, pad] = reader.read::<[u32; 2]>()?;

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
            unit,
            pad,
        })
    }

    /// Get the unit of the sequence.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::Pod;
    ///
    /// let mut pod = Pod::array();
    /// let mut st = pod.as_mut().encode_sequence()?;
    /// st.control(1, 2)?.encode(1i32)?;
    /// st.control(1, 2)?.encode(2i32)?;
    /// st.control(1, 2)?.encode(3i32)?;
    /// st.close()?;
    ///
    /// let st = pod.decode_sequence()?;
    /// assert_eq!(st.unit(), 0);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub const fn unit(&self) -> u32 {
        self.unit
    }

    /// Get the pad of the sequence.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::Pod;
    ///
    /// let mut pod = Pod::array();
    /// let mut st = pod.as_mut().encode_sequence()?;
    /// st.control(1, 2)?.encode(1i32)?;
    /// st.control(1, 2)?.encode(2i32)?;
    /// st.control(1, 2)?.encode(3i32)?;
    /// st.close()?;
    ///
    /// let st = pod.decode_sequence()?;
    /// assert_eq!(st.pad(), 0);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub const fn pad(&self) -> u32 {
        self.pad
    }

    /// Test if the decoder is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::Pod;
    ///
    /// let mut pod = Pod::array();
    /// let mut st = pod.as_mut().encode_sequence()?;
    /// st.control(1, 2)?.encode(1i32)?;
    /// st.control(1, 2)?.encode(2i32)?;
    /// st.control(1, 2)?.encode(3i32)?;
    /// st.close()?;
    ///
    /// let mut st = pod.decode_sequence()?;
    ///
    /// assert!(!st.is_empty());
    /// assert_eq!(st.control()?.value().decode::<i32>()?, 1i32);
    /// assert_eq!(st.control()?.value().decode::<i32>()?, 2i32);
    /// assert_eq!(st.control()?.value().decode::<i32>()?, 3i32);
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
    /// use pod::{Pod, TypedPod};
    ///
    /// let mut pod = Pod::array();
    /// let mut st = pod.as_mut().encode_sequence()?;
    /// st.control(1, 2)?.encode(1i32)?;
    /// st.control(1, 2)?.encode(2i32)?;
    /// st.control(1, 2)?.encode(3i32)?;
    /// st.close()?;
    ///
    /// let mut st = pod.decode_sequence()?;
    ///
    /// assert!(!st.is_empty());
    /// assert_eq!(st.control()?.value().decode::<i32>()?, 1i32);
    /// assert_eq!(st.control()?.value().decode::<i32>()?, 2i32);
    /// assert_eq!(st.control()?.value().decode::<i32>()?, 3i32);
    /// assert!(st.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn control(&mut self) -> Result<Control<R::Clone<'_>>, Error> {
        if self.size == 0 {
            return Err(Error::new(ErrorKind::ObjectUnderflow));
        }

        let [control_offset, control_ty] = self.reader.read::<[u32; 2]>()?;
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
        Ok(Control::new(control_offset, control_ty, pod))
    }
}
