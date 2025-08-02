//! Helper to allocate identifiers for protocol objects.

use core::fmt;

use bittle::{Bits, BitsMut};

/// Id allocator for the protocol.
pub struct Ids {
    /// 64 bits indicating which buckets in layer1 are used.
    layer: u128,
}

impl Ids {
    /// Create a new identifier allocator.
    pub const fn new() -> Self {
        Self { layer: 0 }
    }

    /// Explicitly set an identifier.
    ///
    /// # Examples
    ///
    /// ```
    /// use protocol::ids::Ids;
    ///
    /// let mut ids = Ids::new();
    ///
    /// assert_eq!(ids.alloc(), Some(0));
    /// assert_eq!(ids.alloc(), Some(1));
    /// ids.set(2);
    /// assert_eq!(ids.alloc(), Some(3));
    ///
    /// assert!(ids.test(0));
    /// assert!(ids.test(2));
    /// assert!(!ids.test(4));
    /// ```
    pub fn set(&mut self, index: u32) {
        assert!(index < 128, "Index out of bounds: {index}");
        self.layer.set_bit(index);
    }

    /// Unset an identifier.
    ///
    /// # Examples
    ///
    /// ```
    /// use protocol::ids::Ids;
    ///
    /// let mut ids = Ids::new();
    ///
    /// assert_eq!(ids.alloc(), Some(0));
    /// assert_eq!(ids.alloc(), Some(1));
    /// ids.set(2);
    /// assert_eq!(ids.alloc(), Some(3));
    ///
    /// assert!(ids.test(0));
    /// assert!(ids.test(2));
    /// assert!(!ids.test(4));
    /// ids.unset(2);
    /// assert!(!ids.test(2));
    /// ```
    pub fn unset(&mut self, index: u32) {
        assert!(index < 128, "Index out of bounds: {index}");
        self.layer.clear_bit(index);
    }

    /// Test if the given index is set.
    ///
    /// # Examples
    ///
    /// ```
    /// use protocol::ids::Ids;
    ///
    /// let mut ids = Ids::new();
    ///
    /// assert_eq!(ids.alloc(), Some(0));
    /// assert_eq!(ids.alloc(), Some(1));
    /// ids.set(2);
    /// assert_eq!(ids.alloc(), Some(3));
    ///
    /// assert!(ids.test(0));
    /// assert!(ids.test(2));
    /// assert!(!ids.test(4));
    /// ```
    pub fn test(&self, index: u32) -> bool {
        self.layer.test_bit(index)
    }

    /// Allocate a new identifier.
    ///
    /// # Examples
    ///
    /// ```
    /// use protocol::ids::Ids;
    ///
    /// let mut ids = Ids::new();
    ///
    /// assert_eq!(ids.alloc(), Some(0));
    /// assert_eq!(ids.alloc(), Some(1));
    /// ids.set(2);
    /// assert_eq!(ids.alloc(), Some(3));
    ///
    /// assert!(ids.test(0));
    /// assert!(ids.test(2));
    /// assert!(!ids.test(4));
    /// ```
    pub fn alloc(&mut self) -> Option<u32> {
        let id = self.layer.iter_zeros().next()?;
        self.set(id);
        Some(id)
    }

    /// Iterate over all bits that are set.
    pub fn take_next(&mut self) -> Option<u32> {
        let id = self.layer.iter_ones().next()?;
        self.unset(id);
        Some(id)
    }
}

impl Default for Ids {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for Ids {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Ids").finish_non_exhaustive()
    }
}
