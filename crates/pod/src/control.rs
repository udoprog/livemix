use core::fmt;

use crate::{Reader, TypedPod};

/// A control item inside of a sequence.
///
/// # Examples
///
/// ```
/// use pod::{Pod, Type};
///
/// let mut pod = Pod::array();
/// pod.as_mut().encode_sequence(|seq| {
///     seq.control(1, 10)?.push(1i32)?;
///     Ok(())
/// })?;
///
/// let mut seq = pod.as_ref().next_sequence()?;
/// assert!(!seq.is_empty());
/// let c = seq.control()?;
/// assert_eq!(c.offset(), 1);
/// assert_eq!(c.ty(), 10);
/// assert_eq!(c.value().next::<i32>()?, 1);
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
    /// pod.as_mut().encode_sequence(|seq| {
    ///     seq.control(1, 10)?.push(1i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let mut seq = pod.as_ref().next_sequence()?;
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
    /// pod.as_mut().encode_sequence(|seq| {
    ///     seq.control(1, 10)?.push(1i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let mut seq = pod.as_ref().next_sequence()?;
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
    /// pod.as_mut().encode_sequence(|seq| {
    ///     seq.control(1, 10)?.push(1i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let mut seq = pod.as_ref().next_sequence()?;
    /// let c = seq.control()?;
    /// assert_eq!(c.value().next::<i32>()?, 1);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn value(self) -> TypedPod<B> {
        self.value
    }
}

impl<'de, B> fmt::Debug for Control<B>
where
    B: Reader<'de, u64>,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Control")
            .field("offset", &self.offset)
            .field("type", &self.ty)
            .field("value", &self.value)
            .finish()
    }
}
