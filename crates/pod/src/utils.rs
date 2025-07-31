//! Utilities for working with aligned types and buffers.

use core::mem;
use core::mem::MaybeUninit;
use core::slice;

use crate::Error;
use crate::error::ErrorKind;

/// Trait implemented for types which are word-sized and can inhabit all bit
/// patterns.
pub unsafe trait AlignableWith
where
    Self: Sized,
{
    /// The size of the word in the alignment.
    #[doc(hidden)]
    const WORD_SIZE: usize = {
        assert!(mem::size_of::<Self>() >= mem::size_of::<u64>());
        assert!(mem::size_of::<Self>() % mem::size_of::<u64>() == 0);
        // Helper types to deal with alignment assumes that alignment is less than or equal to 8.
        assert!(mem::align_of::<Self>() <= 8);
        mem::size_of::<Self>() / mem::size_of::<u64>()
    };
}

unsafe impl AlignableWith for u64 {}
unsafe impl<const N: usize> AlignableWith for [u64; N] {}
unsafe impl AlignableWith for [u32; 2] {}
unsafe impl AlignableWith for [u32; 4] {}
unsafe impl AlignableWith for [u32; 6] {}

/// Indicates a type which has all bit patterns inhabited.
pub unsafe trait BytesInhabited
where
    Self: 'static + Copy,
{
}

unsafe impl BytesInhabited for u8 {}
unsafe impl BytesInhabited for i8 {}
unsafe impl BytesInhabited for u16 {}
unsafe impl BytesInhabited for i16 {}
unsafe impl BytesInhabited for i32 {}
unsafe impl BytesInhabited for u32 {}
unsafe impl BytesInhabited for i64 {}
unsafe impl BytesInhabited for u64 {}
unsafe impl<T, const N: usize> BytesInhabited for [T; N] where T: BytesInhabited {}

/// Helper type which alllows for building buffers of type `U` which are aligned
/// to type `T` of size `N`.
#[repr(C, align(8))]
pub struct UninitAlign<T>(MaybeUninit<T>);

impl<T> UninitAlign<T>
where
    T: Copy,
{
    /// Get a mutable slice of the aligned value.
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [MaybeUninit<u8>] {
        unsafe { slice::from_raw_parts_mut(self.as_mut_ptr(), self.size()) }
    }

    /// Get a pointer to the word representation of the value.
    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut MaybeUninit<u8> {
        (&mut self.0 as *mut MaybeUninit<T>).cast()
    }

    /// Get the size of the region in word.
    #[inline]
    pub fn size(&self) -> usize {
        mem::size_of::<T>()
    }
}

impl<T> UninitAlign<T> {
    /// Construct a new uninitialized value.
    #[inline]
    pub const fn uninit() -> Self {
        // SAFETY: This just constructs an array of uninitialized values.
        Self(MaybeUninit::uninit())
    }

    /// Assume that the value is initialized and return it.
    ///
    /// # Safety
    ///
    /// The value must have been initialized before calling this method.
    #[inline]
    pub unsafe fn assume_init(self) -> T {
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
}

pub(crate) fn array_remaining(
    size: usize,
    child_size: usize,
    header_size: usize,
) -> Result<usize, Error> {
    let Some(size) = size.checked_sub(header_size) else {
        return Err(Error::new(ErrorKind::ArraySizeUnderflow));
    };

    if size % child_size != 0 || child_size == 0 {
        return Err(Error::new(ErrorKind::ArraySizeMismatch {
            size,
            child_size,
        }));
    }

    Ok(size / child_size)
}
