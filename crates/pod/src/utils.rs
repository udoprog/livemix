use core::mem::MaybeUninit;
use core::slice;

use crate::error::ErrorKind;
use crate::{Error, WORD_SIZE};

/// Helper to align a value to a word, making necessary write conversions safe.
#[repr(align(8))]
pub(crate) struct Align<T>(pub T)
where
    T: WordSized;

impl<T> Align<T>
where
    T: WordSized,
{
    /// Coerce a value into a slice of words.
    #[inline]
    pub(crate) fn as_words(&self) -> &[u64] {
        // SAFETY: The value must be word-aligned and packed.
        unsafe { slice::from_raw_parts(self.as_ptr(), Self::WORD_SIZE) }
    }

    /// Get a pointer to the word representation of the value.
    #[inline]
    pub(crate) fn as_ptr(&self) -> *const u64 {
        &self.0 as *const T as *const u64
    }
}

impl<T> Clone for Align<T>
where
    T: WordSized,
{
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Align<T> where T: WordSized {}

impl<T> Align<T> where T: WordSized {}

/// Helper type which alllows for building buffers of type `U` which are aligned
/// to type `T` of size `N`.
#[repr(C, align(8))]
pub(crate) struct UninitAlign<T>(MaybeUninit<T>);

unsafe impl<T> WordSized for Align<T>
where
    T: WordSized,
{
    const WORD_SIZE: usize = T::WORD_SIZE;
}

/// Trait implemented for types which are word-sized and can inhabit all bit
/// patterns.
///
/// # Safety
///
/// Must only be implemented for types which are word-aligned and packed. That
/// is, some multiple of `WORD_SIZE` and can inhabit all bit-patterns.
pub unsafe trait WordSized: Copy {
    /// The size of the word in the alignment.
    #[doc(hidden)]
    const WORD_SIZE: usize;
}

unsafe impl WordSized for u64 {
    const WORD_SIZE: usize = 1;
}
unsafe impl WordSized for [u32; 2] {
    const WORD_SIZE: usize = 1;
}
unsafe impl WordSized for [u32; 4] {
    const WORD_SIZE: usize = 2;
}
unsafe impl WordSized for [u32; 6] {
    const WORD_SIZE: usize = 3;
}
unsafe impl<const N: usize> WordSized for [u64; N] {
    const WORD_SIZE: usize = N;
}
unsafe impl WordSized for u128 {
    const WORD_SIZE: usize = 2;
}

impl<T> UninitAlign<T>
where
    T: WordSized,
{
    /// Get a mutable slice of the aligned value.
    #[inline]
    pub(crate) fn as_mut_slice(&mut self) -> &mut [MaybeUninit<u64>] {
        unsafe {
            slice::from_raw_parts_mut(
                (&mut self.0 as *mut MaybeUninit<T>).cast::<MaybeUninit<u64>>(),
                T::WORD_SIZE,
            )
        }
    }
}

impl<T> UninitAlign<T>
where
    T: WordSized,
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

#[repr(align(8))]
pub(crate) struct WordBytes([u8; 8]);

impl WordBytes {
    pub(crate) fn new() -> Self {
        // SAFETY: This just constructs an array of uninitialized values.
        Self([0; 8])
    }

    /// Write a `usize` value to the lower end of the region.
    #[inline]
    pub(crate) fn write_usize(&mut self, value: usize) {
        // SAFETY: 8-byte alignment ensures that the pointer is valid for
        // writing.
        unsafe {
            self.0.as_mut_ptr().cast::<usize>().write(value);
        }
    }

    /// Write a `u64` value to the region.
    #[inline]
    pub(crate) fn write_u64(&mut self, value: u64) {
        // SAFETY: 8-byte alignment ensures that the pointer is valid for
        // writing.
        unsafe {
            self.0.as_mut_ptr().cast::<u64>().write(value);
        }
    }

    /// Reading a `usize` value from the lower end of the region..
    #[inline]
    pub(crate) fn read_usize(&self) -> usize {
        // SAFETY: 8-byte alignment ensures that the pointer is valid for
        // reading.
        unsafe { self.0.as_ptr().cast::<usize>().read() }
    }

    /// Write literal half-words to the entirety for the region.
    #[inline]
    pub(crate) fn write_half_words(&mut self, value: [u32; 2]) {
        // SAFETY: The region is valid for writing.
        unsafe {
            self.0.as_mut_ptr().cast::<[u32; 2]>().write(value);
        }
    }

    #[inline]
    pub(crate) fn as_array(&self) -> &[u64; 1] {
        // SAFETY: Type is always initialized to something valid.
        unsafe { &*self.0.as_ptr().cast::<[u64; 1]>() }
    }

    #[inline]
    pub(crate) fn as_array_u32(&self) -> &[u32; 2] {
        // SAFETY: Type is always initialized to something valid.
        unsafe { &*self.0.as_ptr().cast::<[u32; 2]>() }
    }
}

pub(crate) fn array_remaining(size: u32, child_size: u32, header_size: u32) -> Result<u32, Error> {
    let Some(size) = size.checked_sub(header_size) else {
        return Err(Error::new(ErrorKind::SizeOverflow));
    };

    let remaining = 'out: {
        if size == 0 {
            break 'out 0;
        }

        let Some(padded_child_size) = child_size.checked_next_multiple_of(WORD_SIZE) else {
            return Err(Error::new(ErrorKind::SizeOverflow));
        };

        let Some(size) = size.checked_div(padded_child_size) else {
            break 'out 0;
        };

        size
    };

    Ok(remaining)
}
