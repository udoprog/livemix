use core::fmt;

use crate::{AsSlice, RawId, Value};

/// A property inside of an object.
pub struct Property<B> {
    key: u32,
    flags: u32,
    value: Value<B>,
}

impl<B> Property<B> {
    #[inline]
    pub(crate) fn new(key: u32, flags: u32, value: Value<B>) -> Self {
        Self { key, flags, value }
    }

    /// Get the key of the property.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = pod::array();
    /// pod.as_mut().write_object(10, 20, |obj| {
    ///     obj.property(1).write(1i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let mut obj = pod.as_ref().read_object()?;
    /// let p = obj.property()?;
    /// assert_eq!(p.key::<u32>(), 1);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn key<T>(&self) -> T
    where
        T: RawId,
    {
        T::from_id(self.key)
    }

    /// Get the flags of the property.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = pod::array();
    /// pod.as_mut().write_object(10, 20, |obj| {
    ///     obj.property(1).flags(0b001).write(1i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let mut obj = pod.as_ref().read_object()?;
    /// let p = obj.property()?;
    /// assert_eq!(p.flags(), 0b001);
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
    /// let mut pod = pod::array();
    /// pod.as_mut().write_object(10, 20, |obj| {
    ///     obj.property(1).write(1i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let mut obj = pod.as_ref().read_object()?;
    /// let p = obj.property()?;
    /// assert_eq!(p.value().read_sized::<i32>()?, 1);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn value(self) -> Value<B> {
        self.value
    }
}

impl<B> fmt::Debug for Property<B>
where
    B: AsSlice,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Property")
            .field("key", &self.key)
            .field("flags", &self.flags)
            .field("value", &self.value)
            .finish()
    }
}
