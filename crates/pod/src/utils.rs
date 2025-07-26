//! Utilities for working with aligned types and buffers.

use core::mem;
use core::mem::MaybeUninit;
use core::slice;

use crate::error::ErrorKind;
use crate::{Error, WORD_SIZE};

/// Trait implemented for types which are word-sized and can inhabit all bit
/// patterns.
///
/// # Safety
///
/// Must only be implemented for types which are word-aligned and packed. That
/// is, some multiple of `WORD_SIZE` and can inhabit all bit-patterns.
pub unsafe trait AlignableWith<T>
where
    Self: Sized,
    T: Sized,
{
    /// The size of the word in the alignment.
    #[doc(hidden)]
    const WORD_SIZE: usize = mem::size_of::<Self>() / mem::size_of::<T>();
}

unsafe impl<T> AlignableWith<T> for T where T: BytesInhabited {}
unsafe impl<T, const N: usize> AlignableWith<T> for [T; N] where T: AlignableWith<T> {}
unsafe impl AlignableWith<u64> for [u32; 2] {}
unsafe impl AlignableWith<u64> for [u32; 4] {}
unsafe impl AlignableWith<u64> for [u32; 6] {}
unsafe impl AlignableWith<u64> for u128 {}

/// Indicates a type which has all bit patterns inhabited.
pub unsafe trait BytesInhabited
where
    Self: Copy,
{
}
unsafe impl BytesInhabited for u32 {}
unsafe impl BytesInhabited for u64 {}
unsafe impl BytesInhabited for u128 {}
unsafe impl<T, const N: usize> BytesInhabited for [T; N] where T: BytesInhabited {}

/// Helper to align a value to a word, making necessary write conversions safe.
#[repr(align(8))]
pub struct Align<T>(pub T);

impl<T> Align<T> {
    /// Coerce a value into a slice of words.
    #[inline]
    pub fn as_words<U>(&self) -> &[U]
    where
        T: AlignableWith<U>,
    {
        // SAFETY: The value must be word-aligned and packed.
        unsafe { slice::from_raw_parts(self.as_ptr(), T::WORD_SIZE) }
    }

    /// Get a pointer to the word representation of the value.
    #[inline]
    pub fn as_ptr<U>(&self) -> *const U
    where
        T: AlignableWith<U>,
    {
        &self.0 as *const T as *const U
    }

    /// Get the size of the region in word.
    #[inline]
    pub fn size(&self) -> usize
    where
        T: AlignableWith<u64>,
    {
        T::WORD_SIZE
    }
}

impl<T> Clone for Align<T>
where
    T: Copy,
{
    #[inline]
    fn clone(&self) -> Self {
        Self(self.0)
    }
}

impl<T> Copy for Align<T> where T: Copy {}

impl<T> Align<T> where T: AlignableWith<u64> {}

/// Helper type which alllows for building buffers of type `U` which are aligned
/// to type `T` of size `N`.
#[repr(C, align(8))]
pub(crate) struct UninitAlign<T>(MaybeUninit<T>);

impl<T> UninitAlign<T> {
    /// Get a mutable slice of the aligned value.
    #[inline]
    pub(crate) fn as_mut_slice<U>(&mut self) -> &mut [MaybeUninit<U>]
    where
        T: AlignableWith<U>,
    {
        unsafe {
            slice::from_raw_parts_mut(
                (&mut self.0 as *mut MaybeUninit<T>).cast::<MaybeUninit<U>>(),
                T::WORD_SIZE,
            )
        }
    }
}

impl<T> UninitAlign<T> {
    /// Construct a new uninitialized value.
    #[inline]
    pub(crate) const fn uninit() -> Self {
        // SAFETY: This just constructs an array of uninitialized values.
        Self(MaybeUninit::uninit())
    }

    /// Assume that the value is initialized and return it.
    ///
    /// # Safety
    ///
    /// The value must have been initialized before calling this method.
    #[inline]
    pub(crate) unsafe fn assume_init(self) -> T {
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

mod sealed {
    pub trait Sealed {}
    impl Sealed for u32 {}
    impl Sealed for u64 {}
    impl Sealed for u128 {}
}
