use crate::TypedPod;

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
    /// use pod::{ArrayBuf, Pod, Type};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let pod = Pod::new(&mut buf);
    /// let mut obj = pod.encode_object(10, 20)?;
    ///
    /// obj.property(1, 10)?.encode(1i32)?;
    /// obj.close()?;
    ///
    /// let pod = Pod::new(buf.as_slice());
    /// let mut obj = pod.decode_object()?;
    ///
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
    /// use pod::{ArrayBuf, Pod, Type};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let pod = Pod::new(&mut buf);
    /// let mut obj = pod.encode_object(10, 20)?;
    ///
    /// obj.property(1, 10)?.encode(1i32)?;
    /// obj.close()?;
    ///
    /// let pod = Pod::new(buf.as_slice());
    /// let mut obj = pod.decode_object()?;
    ///
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
    /// use pod::{ArrayBuf, Pod, Type};
    ///
    /// let mut buf = ArrayBuf::new();
    /// let pod = Pod::new(&mut buf);
    /// let mut obj = pod.encode_object(10, 20)?;
    ///
    /// obj.property(1, 10)?.encode(1i32)?;
    /// obj.close()?;
    ///
    /// let pod = Pod::new(buf.as_slice());
    /// let mut obj = pod.decode_object()?;
    ///
    /// let p = obj.property()?;
    /// assert_eq!(p.value().decode::<i32>()?, 1);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn value(self) -> TypedPod<B> {
        self.value
    }
}
