use core::fmt;
use core::mem;

#[cfg(feature = "alloc")]
use alloc::boxed::Box;

use crate::error::ErrorKind;
use crate::{AsReader, Control, Encode, Error, Reader, Type, TypedPod, WORD_SIZE, Writer};

/// A decoder for a sequence.
pub struct Sequence<B> {
    buf: B,
    size: u32,
    unit: u32,
    pad: u32,
}

impl<B> Sequence<B> {
    /// Get the unit of the sequence.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::Pod;
    ///
    /// let mut pod = Pod::array();
    /// pod.as_mut().encode_sequence(|seq| {
    ///     seq.control(1, 2)?.push(1i32)?;
    ///     seq.control(1, 2)?.push(2i32)?;
    ///     seq.control(1, 2)?.push(3i32)?;
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
    ///     seq.control(1, 2)?.push(1i32)?;
    ///     seq.control(1, 2)?.push(2i32)?;
    ///     seq.control(1, 2)?.push(3i32)?;
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

    /// Get a reference to the underlying buffer.
    #[inline]
    pub fn as_buf(&self) -> &B {
        &self.buf
    }
}

impl<'de, B> Sequence<B>
where
    B: Reader<'de, u64>,
{
    #[inline]
    pub fn new(buf: B, size: u32, unit: u32, pad: u32) -> Self {
        Self {
            buf,
            size,
            unit,
            pad,
        }
    }

    #[inline]
    pub(crate) fn from_reader(mut reader: B, size: u32) -> Result<Self, Error> {
        let [unit, pad] = reader.read::<[u32; 2]>()?;

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
            unit,
            pad,
        })
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
    ///     seq.control(1, 2)?.push(1i32)?;
    ///     seq.control(1, 2)?.push(2i32)?;
    ///     seq.control(1, 2)?.push(3i32)?;
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
    ///     seq.control(1, 2)?.push(1i32)?;
    ///     seq.control(1, 2)?.push(2i32)?;
    ///     seq.control(1, 2)?.push(3i32)?;
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
    pub fn control(&mut self) -> Result<Control<B::Reader<'_>>, Error> {
        if self.size == 0 {
            return Err(Error::new(ErrorKind::ObjectUnderflow));
        }

        let [control_offset, control_ty] = self.buf.read::<[u32; 2]>()?;
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
        Ok(Control::new(control_offset, control_ty, pod))
    }

    /// Coerce into an owned [`Sequence`].
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, TypedPod};
    ///
    /// let mut pod = Pod::array();
    /// pod.as_mut().encode_sequence(|seq| {
    ///     seq.control(1, 2)?.push(1i32)?;
    ///     seq.control(1, 2)?.push(2i32)?;
    ///     seq.control(1, 2)?.push(3i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let seq = pod.decode_sequence()?.to_owned();
    ///
    /// let mut seq = seq.as_ref();
    /// assert!(!seq.is_empty());
    /// assert_eq!(seq.control()?.value().decode::<i32>()?, 1i32);
    /// assert_eq!(seq.control()?.value().decode::<i32>()?, 2i32);
    /// assert_eq!(seq.control()?.value().decode::<i32>()?, 3i32);
    /// assert!(seq.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[cfg(feature = "alloc")]
    #[inline]
    pub fn to_owned(&self) -> Sequence<Box<[u64]>> {
        Sequence {
            buf: Box::from(self.buf.as_slice()),
            size: self.size,
            unit: self.unit,
            pad: self.pad,
        }
    }
}

impl<B> Sequence<B>
where
    B: AsReader<u64>,
{
    /// Coerce into a borrowed [`Sequence`].
    ///
    /// Decoding this object does not affect the original object.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, TypedPod};
    ///
    /// let mut pod = Pod::array();
    /// pod.as_mut().encode_sequence(|seq| {
    ///     seq.control(1, 2)?.push(1i32)?;
    ///     seq.control(1, 2)?.push(2i32)?;
    ///     seq.control(1, 2)?.push(3i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let seq = pod.decode_sequence()?.to_owned();
    ///
    /// let mut seq = seq.as_ref();
    /// assert!(!seq.is_empty());
    /// assert_eq!(seq.control()?.value().decode::<i32>()?, 1i32);
    /// assert_eq!(seq.control()?.value().decode::<i32>()?, 2i32);
    /// assert_eq!(seq.control()?.value().decode::<i32>()?, 3i32);
    /// assert!(seq.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn as_ref(&self) -> Sequence<B::Reader<'_>> {
        Sequence::new(self.buf.as_reader(), self.size, self.unit, self.pad)
    }
}

/// [`Encode`] implementation for [`Struct`].
///
/// # Examples
///
/// ```
/// use pod::{Pod, TypedPod};
///
/// let mut pod = Pod::array();
/// pod.as_mut().encode_sequence(|seq| {
///     seq.control(1, 2)?.push(1i32)?;
///     seq.control(1, 2)?.push(2i32)?;
///     seq.control(1, 2)?.push(3i32)?;
///     Ok(())
/// })?;
///
/// let seq = pod.decode_sequence()?;
///
/// let mut pod2 = Pod::array();
/// pod2.as_mut().push(seq)?;
///
/// let seq = pod2.decode_sequence()?;
///
/// let mut seq = seq.as_ref();
/// assert!(!seq.is_empty());
/// assert_eq!(seq.control()?.value().decode::<i32>()?, 1i32);
/// assert_eq!(seq.control()?.value().decode::<i32>()?, 2i32);
/// assert_eq!(seq.control()?.value().decode::<i32>()?, 3i32);
/// assert!(seq.is_empty());
/// # Ok::<_, pod::Error>(())
/// ```
impl<B> Encode for Sequence<B>
where
    B: AsReader<u64>,
{
    const TYPE: Type = Type::SEQUENCE;

    #[inline]
    fn size(&self) -> u32 {
        (self
            .buf
            .as_reader()
            .as_slice()
            .len()
            .wrapping_mul(mem::size_of::<u64>()) as u32)
            .wrapping_add(WORD_SIZE)
    }

    #[inline]
    fn write_content(&self, mut writer: impl Writer<u64>) -> Result<(), Error> {
        writer.write([self.unit, self.pad])?;
        writer.write_words(self.buf.as_reader().as_slice())
    }
}

impl<B> fmt::Debug for Sequence<B>
where
    B: AsReader<u64>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct Controls<'a, B>(&'a Sequence<B>);

        impl<B> fmt::Debug for Controls<'_, B>
        where
            B: AsReader<u64>,
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
