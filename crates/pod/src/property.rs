use crate::TypedPod;

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
    #[inline]
    pub fn key(&self) -> u32 {
        self.key
    }

    /// Get the flags of the property.
    #[inline]
    pub fn flags(&self) -> u32 {
        self.flags
    }

    /// Access the value of the property.
    #[inline]
    pub fn value(self) -> TypedPod<B> {
        self.value
    }
}
