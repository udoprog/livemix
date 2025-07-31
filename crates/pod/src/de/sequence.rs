use core::fmt;
use core::mem;

#[cfg(feature = "alloc")]
use crate::DynamicBuf;
#[cfg(feature = "alloc")]
use crate::buf::AllocError;
use crate::error::ErrorKind;
use crate::{
    AsSlice, Control, EncodeUnsized, Error, PADDING, Reader, Slice, Type, TypedPod, Writer,
};

/// A decoder for a sequence.
pub struct Sequence<B> {
    buf: B,
    unit: u32,
    pad: u32,
}

impl<B> Sequence<B> {
    /// Get the unit of the sequence.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut pod = pod::array();
    /// pod.as_mut().push_sequence(|seq| {
    ///     seq.control().push(1i32)?;
    ///     seq.control().push(2i32)?;
    ///     seq.control().push(3i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let seq = pod.as_ref().next_sequence()?;
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
    /// let mut pod = pod::array();
    /// pod.as_mut().push_sequence(|seq| {
    ///     seq.control().push(1i32)?;
    ///     seq.control().push(2i32)?;
    ///     seq.control().push(3i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let seq = pod.as_ref().next_sequence()?;
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
    B: Reader<'de>,
{
    #[inline]
    pub fn new(buf: B, unit: u32, pad: u32) -> Self {
        Self { buf, unit, pad }
    }

    #[inline]
    pub(crate) fn from_reader(mut reader: B) -> Result<Self, Error> {
        let [unit, pad] = reader.read::<[u32; 2]>()?;

        Ok(Self {
            buf: reader,
            unit,
            pad,
        })
    }

    /// Test if the decoder is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut pod = pod::array();
    /// pod.as_mut().push_sequence(|seq| {
    ///     seq.control().push(1i32)?;
    ///     seq.control().push(2i32)?;
    ///     seq.control().push(3i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let mut seq = pod.as_ref().next_sequence()?;
    /// assert!(!seq.is_empty());
    /// assert_eq!(seq.control()?.value().next::<i32>()?, 1i32);
    /// assert_eq!(seq.control()?.value().next::<i32>()?, 2i32);
    /// assert_eq!(seq.control()?.value().next::<i32>()?, 3i32);
    /// assert!(seq.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    /// Decode the next field in the struct.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, TypedPod};
    ///
    /// let mut pod = pod::array();
    /// pod.as_mut().push_sequence(|seq| {
    ///     seq.control().push(1i32)?;
    ///     seq.control().push(2i32)?;
    ///     seq.control().push(3i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let mut seq = pod.as_ref().next_sequence()?;
    /// assert!(!seq.is_empty());
    /// assert_eq!(seq.control()?.value().next::<i32>()?, 1i32);
    /// assert_eq!(seq.control()?.value().next::<i32>()?, 2i32);
    /// assert_eq!(seq.control()?.value().next::<i32>()?, 3i32);
    /// assert!(seq.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn control(&mut self) -> Result<Control<Slice<'de>>, Error> {
        if self.buf.is_empty() {
            return Err(Error::new(ErrorKind::ObjectUnderflow));
        }

        let [control_offset, control_type] = self.buf.read::<[u32; 2]>()?;
        let (size, ty) = self.buf.header()?;

        let Some(head) = self.buf.split(size) else {
            return Err(Error::new(ErrorKind::BufferUnderflow));
        };

        let pod = TypedPod::packed(head, size, ty);
        self.buf.unpad(PADDING)?;
        Ok(Control::new(control_offset, control_type, pod))
    }

    /// Coerce into an owned [`Sequence`].
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, TypedPod};
    ///
    /// let mut pod = pod::array();
    /// pod.as_mut().push_sequence(|seq| {
    ///     seq.control().push(1i32)?;
    ///     seq.control().push(2i32)?;
    ///     seq.control().push(3i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let seq = pod.as_ref().next_sequence()?.to_owned()?;
    ///
    /// let mut seq = seq.as_ref();
    /// assert!(!seq.is_empty());
    /// assert_eq!(seq.control()?.value().next::<i32>()?, 1i32);
    /// assert_eq!(seq.control()?.value().next::<i32>()?, 2i32);
    /// assert_eq!(seq.control()?.value().next::<i32>()?, 3i32);
    /// assert!(seq.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[cfg(feature = "alloc")]
    #[inline]
    pub fn to_owned(&self) -> Result<Sequence<DynamicBuf>, AllocError> {
        Ok(Sequence {
            buf: DynamicBuf::from_slice(self.buf.as_bytes())?,
            unit: self.unit,
            pad: self.pad,
        })
    }
}

impl<B> Sequence<B>
where
    B: AsSlice,
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
    /// let mut pod = pod::array();
    /// pod.as_mut().push_sequence(|seq| {
    ///     seq.control().push(1i32)?;
    ///     seq.control().push(2i32)?;
    ///     seq.control().push(3i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let seq = pod.as_ref().next_sequence()?.to_owned()?;
    ///
    /// let mut seq = seq.as_ref();
    /// assert!(!seq.is_empty());
    /// assert_eq!(seq.control()?.value().next::<i32>()?, 1i32);
    /// assert_eq!(seq.control()?.value().next::<i32>()?, 2i32);
    /// assert_eq!(seq.control()?.value().next::<i32>()?, 3i32);
    /// assert!(seq.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn as_ref(&self) -> Sequence<Slice<'_>> {
        Sequence::new(self.buf.as_slice(), self.unit, self.pad)
    }
}

/// [`Encode`] implementation for [`Struct`].
///
/// # Examples
///
/// ```
/// use pod::{Pod, TypedPod};
///
/// let mut pod = pod::array();
/// pod.as_mut().push_sequence(|seq| {
///     seq.control().push(1i32)?;
///     seq.control().push(2i32)?;
///     seq.control().push(3i32)?;
///     Ok(())
/// })?;
///
/// let seq = pod.as_ref().next_sequence()?;
///
/// let mut pod2 = pod::array();
/// pod2.as_mut().write(seq)?;
///
/// let seq = pod2.as_ref().next_sequence()?;
///
/// let mut seq = seq.as_ref();
/// assert!(!seq.is_empty());
/// assert_eq!(seq.control()?.value().next::<i32>()?, 1i32);
/// assert_eq!(seq.control()?.value().next::<i32>()?, 2i32);
/// assert_eq!(seq.control()?.value().next::<i32>()?, 3i32);
/// assert!(seq.is_empty());
/// # Ok::<_, pod::Error>(())
/// ```
impl<B> EncodeUnsized for Sequence<B>
where
    B: AsSlice,
{
    const TYPE: Type = Type::SEQUENCE;

    #[inline]
    fn size(&self) -> Option<usize> {
        let len = self.buf.as_slice().len();
        len.checked_add(mem::size_of::<[u32; 2]>())
    }

    #[inline]
    fn write_content(&self, mut writer: impl Writer) -> Result<(), Error> {
        writer.write(&[self.unit, self.pad])?;
        writer.write(self.buf.as_slice().as_bytes())
    }
}

crate::macros::encode_into_unsized!(impl [B] Sequence<B> where B: AsSlice);

impl<B> fmt::Debug for Sequence<B>
where
    B: AsSlice,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct Controls<'a, B>(&'a Sequence<B>);

        impl<B> fmt::Debug for Controls<'_, B>
        where
            B: AsSlice,
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
