use core::mem::MaybeUninit;
use core::slice;

/// Helper type which alllows for building buffers of type `U` which are aligned
/// to type `T` of size `N`.
#[repr(transparent)]
pub(crate) struct Align<T>(MaybeUninit<T>);

/// Trait implemented for types which are word-aligned and can inhabit all bit
/// patterns.
///
/// # Safety
///
/// Must only be implemented for types which are word-aligned and packed. That
/// is, some multiple of `WORD_SIZE` and can inhabit all bit-patterns.
pub unsafe trait WordAligned: Copy {
    /// The size of the word in the alignment.
    #[doc(hidden)]
    const WORD_SIZE: usize;
}

unsafe impl WordAligned for u64 {
    const WORD_SIZE: usize = 2;
}
unsafe impl WordAligned for u32 {
    const WORD_SIZE: usize = 1;
}
unsafe impl<const N: usize> WordAligned for [u32; N] {
    const WORD_SIZE: usize = N;
}
unsafe impl<const N: usize> WordAligned for [u64; N] {
    const WORD_SIZE: usize = N * 2;
}

impl<T> Align<T>
where
    T: WordAligned,
{
    /// Get a mutable slice of the aligned value.
    #[inline]
    pub(crate) fn as_mut_slice(&mut self) -> &mut [MaybeUninit<u32>] {
        unsafe {
            slice::from_raw_parts_mut(
                (&mut self.0 as *mut MaybeUninit<T>).cast::<MaybeUninit<u32>>(),
                T::WORD_SIZE,
            )
        }
    }
}

impl<T> Align<T>
where
    T: WordAligned,
{
    #[inline]
    pub(crate) const fn uninit() -> Self {
        // SAFETY: This just constructs an array of uninitialized values.
        Self(MaybeUninit::uninit())
    }

    /// Read the aligned value.
    #[inline]
    pub(crate) unsafe fn assume_init(&self) -> T {
        // Assume that the value is initialized.
        unsafe { self.0.assume_init() }
    }
}
