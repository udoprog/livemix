use core::fmt;

#[repr(transparent)]
pub(crate) struct BStr([u8]);

impl BStr {
    pub(crate) fn new(bytes: &[u8]) -> &Self {
        // SAFETY: BStr is layout compatible with [u8].
        unsafe { &*(bytes as *const [u8] as *const Self) }
    }
}

impl fmt::Debug for BStr {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "b\"")?;

        for chunk in self.0.utf8_chunks() {
            write!(f, "{}", chunk.valid())?;

            for byte in chunk.invalid() {
                write!(f, "\\x{:02x}", byte)?;
            }
        }

        write!(f, "\"")?;
        Ok(())
    }
}
