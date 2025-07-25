use crate::TypedPod;

/// A control item inside of a sequence.
///
/// # Examples
///
/// ```
/// use pod::{Pod, Type};
///
/// let mut pod = Pod::array();
/// let mut seq = pod.as_mut().encode_sequence()?;
///
/// seq.control(1, 10)?.encode(1i32)?;
///
/// seq.close()?;
///
/// let mut pod = pod.typed()?;
/// let mut seq = pod.decode_sequence()?;
///
/// let c = seq.control()?;
/// assert_eq!(c.offset(), 1);
/// assert_eq!(c.ty(), 10);
/// assert_eq!(c.value().decode::<i32>()?, 1);
/// # Ok::<_, pod::Error>(())
/// ```
pub struct Control<B> {
    offset: u32,
    ty: u32,
    value: TypedPod<B>,
}

impl<B> Control<B> {
    #[inline]
    pub(crate) fn new(offset: u32, ty: u32, value: TypedPod<B>) -> Self {
        Self { offset, ty, value }
    }

    /// Get the offset of the control.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = Pod::array();
    /// let mut seq = pod.as_mut().encode_sequence()?;
    /// seq.control(1, 10)?.encode(1i32)?;
    /// seq.close()?;
    ///
    /// let mut pod = pod.typed()?;
    /// let mut seq = pod.decode_sequence()?;
    ///
    /// let c = seq.control()?;
    /// assert_eq!(c.offset(), 1);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn offset(&self) -> u32 {
        self.offset
    }

    /// Get the type of the control.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = Pod::array();
    /// let mut seq = pod.as_mut().encode_sequence()?;
    /// seq.control(1, 10)?.encode(1i32)?;
    /// seq.close()?;
    ///
    /// let mut pod = pod.typed()?;
    /// let mut seq = pod.decode_sequence()?;
    ///
    /// let c = seq.control()?;
    /// assert_eq!(c.ty(), 10);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn ty(&self) -> u32 {
        self.ty
    }

    /// Access the value of the control.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = Pod::array();
    /// let mut seq = pod.as_mut().encode_sequence()?;
    /// seq.control(1, 10)?.encode(1i32)?;
    /// seq.close()?;
    ///
    /// let mut pod = pod.typed()?;
    /// let mut seq = pod.decode_sequence()?;
    ///
    /// let c = seq.control()?;
    /// assert_eq!(c.value().decode::<i32>()?, 1);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn value(self) -> TypedPod<B> {
        self.value
    }
}
