use core::fmt;

use crate::error::ErrorKind;
use crate::{Control, Error, Reader, TypedPod, WORD_SIZE};

/// A decoder for a sequence.
pub struct Sequence<R> {
    reader: R,
    size: u32,
    unit: u32,
    pad: u32,
}

impl<'de, R> Sequence<R>
where
    R: Reader<'de, u64>,
{
    #[inline]
    pub fn new(reader: R, size: u32, unit: u32, pad: u32) -> Self {
        Self {
            reader,
            size,
            unit,
            pad,
        }
    }

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
    /// pod.as_mut().encode_sequence(|seq| {
    ///     seq.control(1, 2)?.encode(1i32)?;
    ///     seq.control(1, 2)?.encode(2i32)?;
    ///     seq.control(1, 2)?.encode(3i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let seq = pod.decode_sequence()?;
    /// assert_eq!(seq.unit(), 0);
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
    /// pod.as_mut().encode_sequence(|seq| {
    ///     seq.control(1, 2)?.encode(1i32)?;
    ///     seq.control(1, 2)?.encode(2i32)?;
    ///     seq.control(1, 2)?.encode(3i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let seq = pod.decode_sequence()?;
    /// assert_eq!(seq.pad(), 0);
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
    /// pod.as_mut().encode_sequence(|seq| {
    ///     seq.control(1, 2)?.encode(1i32)?;
    ///     seq.control(1, 2)?.encode(2i32)?;
    ///     seq.control(1, 2)?.encode(3i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let mut seq = pod.decode_sequence()?;
    /// assert!(!seq.is_empty());
    /// assert_eq!(seq.control()?.value().decode::<i32>()?, 1i32);
    /// assert_eq!(seq.control()?.value().decode::<i32>()?, 2i32);
    /// assert_eq!(seq.control()?.value().decode::<i32>()?, 3i32);
    /// assert!(seq.is_empty());
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
    /// pod.as_mut().encode_sequence(|seq| {
    ///     seq.control(1, 2)?.encode(1i32)?;
    ///     seq.control(1, 2)?.encode(2i32)?;
    ///     seq.control(1, 2)?.encode(3i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let mut seq = pod.decode_sequence()?;
    /// assert!(!seq.is_empty());
    /// assert_eq!(seq.control()?.value().decode::<i32>()?, 1i32);
    /// assert_eq!(seq.control()?.value().decode::<i32>()?, 2i32);
    /// assert_eq!(seq.control()?.value().decode::<i32>()?, 3i32);
    /// assert!(seq.is_empty());
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

    /// Convert the [`SequenceDecoder`] into a one borrowing from but without
    /// modifying the current buffer.
    #[inline]
    pub fn as_ref(&self) -> Sequence<R::Clone<'_>> {
        Sequence::new(self.reader.clone_reader(), self.size, self.unit, self.pad)
    }
}

impl<'de, R> fmt::Debug for Sequence<R>
where
    R: Reader<'de, u64>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct Controls<'a, R>(&'a Sequence<R>);

        impl<'de, R> fmt::Debug for Controls<'_, R>
        where
            R: Reader<'de, u64>,
        {
            #[inline]
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let mut this = self.0.as_ref();

                let mut f = f.debug_list();

                while !this.is_empty() {
                    match this.control() {
                        Ok(control) => {
                            f.entry(&control);
                        }
                        Err(e) => {
                            f.entry(&e);
                        }
                    }
                }

                f.finish()
            }
        }

        let mut f = f.debug_struct("Sequence");
        f.field("unit", &self.unit());
        f.field("pad", &self.pad());
        f.field("controls", &Controls(self));
        f.finish()
    }
}
