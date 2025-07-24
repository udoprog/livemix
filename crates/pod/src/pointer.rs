/// A pointer stored in a pod.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C, align(8))]
pub struct Pointer {
    ty: u32,
    pointer: usize,
}

impl Pointer {
    /// Construct a new pointer with the given address.
    #[inline]
    pub const fn new(pointer: usize) -> Self {
        Self { ty: 0, pointer }
    }

    /// Construct a new pointer with the given address and type.
    #[inline]
    pub(crate) const fn new_with_type(pointer: usize, ty: u32) -> Self {
        Self { ty, pointer }
    }

    /// Modify the pointer to include the specified type.
    #[inline]
    pub const fn with_type(self, ty: u32) -> Self {
        Self { ty, ..self }
    }

    /// Returns the type of the pointer.
    #[inline]
    pub const fn ty(&self) -> u32 {
        self.ty
    }

    /// Returns the address of the pointer.
    #[inline]
    pub const fn pointer(&self) -> usize {
        self.pointer
    }
}
