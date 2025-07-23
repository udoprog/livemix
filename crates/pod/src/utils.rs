use core::mem::MaybeUninit;

/// Helper type which alllows for building buffers of type `U` which are aligned
/// to type `T` of size `N`.
#[repr(C)]
pub(crate) struct Align<T, A>([T; 0], A);

pub(crate) unsafe trait WordAligned: Copy {
    /// The size of the word in the alignment.
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

impl<T, U, const N: usize> Align<T, [U; N]> {
    /// Get a mutable slice of the aligned value.
    #[inline]
    pub(crate) fn as_mut_slice(&mut self) -> &mut [U] {
        &mut self.1
    }
}

impl<T, const N: usize> Align<T, [MaybeUninit<u32>; N]>
where
    T: WordAligned,
{
    #[inline]
    pub(crate) const fn uninit() -> Self {
        const {
            assert!(
                T::WORD_SIZE == N,
                "The size of T must match the underlying array size N"
            );
        }

        // SAFETY: This just constructs an array of uninitialized values.
        Self([], unsafe { MaybeUninit::uninit().assume_init() })
    }

    /// Read the aligned value.
    #[inline]
    pub(crate) unsafe fn assume_init(&self) -> Align<T, [u32; N]> {
        // Assume that the value is initialized.
        Align([], unsafe {
            (&self.1 as *const [MaybeUninit<u32>; N])
                .cast::<[u32; N]>()
                .read()
        })
    }
}

impl<T, const N: usize> Align<T, [u32; N]>
where
    T: WordAligned,
{
    #[cfg(all(test, feature = "alloc"))]
    #[inline]
    pub(crate) fn new(value: [u32; N]) -> Self {
        const {
            assert!(
                T::WORD_SIZE == N,
                "The size of T must match the underlying array size N"
            );
        }

        Align([], value)
    }

    /// Read the aligned value.
    #[inline]
    pub(crate) fn read(&self) -> T {
        // SAFETY: The slice is guaranteed to be N elements long.
        unsafe { (&self.1 as *const [u32; N]).cast::<T>().read() }
    }
}
