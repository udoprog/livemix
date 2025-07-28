use core::fmt;

use crate::{Reader, TypedPod};

/// A property inside of an object.
pub struct Property<B> {
    key: u32,
    flags: u32,
    value: TypedPod<B>,
}

impl<B> Property<B> {
    #[inline]
    pub(crate) fn new(key: u32, flags: u32, value: TypedPod<B>) -> Self {
        Self { key, flags, value }
    }

    /// Get the key of the property.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = Pod::array();
    /// pod.as_mut().push_object(10, 20, |obj| {
    ///     obj.property(1, 10)?.push(1i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let mut obj = pod.next_object()?;
    /// let p = obj.property()?;
    /// assert_eq!(p.key(), 1);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn key(&self) -> u32 {
        self.key
    }

    /// Get the flags of the property.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = Pod::array();
    /// pod.as_mut().push_object(10, 20, |obj| {
    ///     obj.property(1, 10)?.push(1i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let mut obj = pod.next_object()?;
    /// let p = obj.property()?;
    /// assert_eq!(p.flags(), 10);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn flags(&self) -> u32 {
        self.flags
    }

    /// Access the value of the property.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = Pod::array();
    /// pod.as_mut().push_object(10, 20, |obj| {
    ///     obj.property(1, 10)?.push(1i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let mut obj = pod.next_object()?;
    /// let p = obj.property()?;
    /// assert_eq!(p.value().next::<i32>()?, 1);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn value(self) -> TypedPod<B> {
        self.value
    }
}

impl<'de, B> fmt::Debug for Property<B>
where
    B: Reader<'de, u64>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Property")
            .field("key", &self.key)
            .field("flags", &self.flags)
            .field("value", &self.value)
            .finish()
    }
}
