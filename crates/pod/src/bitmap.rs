#[cfg(feature = "alloc")]
use core::borrow::Borrow;
use core::fmt;
#[cfg(feature = "alloc")]
use core::ops::Deref;

#[cfg(feature = "alloc")]
use alloc::borrow::ToOwned;

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

/// An owned bitmap type.
#[cfg(feature = "alloc")]
#[derive(Clone, PartialEq, Eq)]
pub struct OwnedBitmap {
    data: Vec<u8>,
}

#[cfg(feature = "alloc")]
impl OwnedBitmap {
    /// construct a new `OwnedBitmap` from a vector of bytes.
    #[inline]
    pub(crate) fn new(data: Vec<u8>) -> Self {
        Self { data }
    }
}

#[cfg(feature = "alloc")]
impl ToOwned for Bitmap {
    type Owned = OwnedBitmap;

    #[inline]
    fn to_owned(&self) -> Self::Owned {
        OwnedBitmap::new(self.data.to_vec())
    }
}

#[cfg(feature = "alloc")]
impl Borrow<Bitmap> for OwnedBitmap {
    #[inline]
    fn borrow(&self) -> &Bitmap {
        Bitmap::new(&self.data)
    }
}

#[cfg(feature = "alloc")]
impl fmt::Debug for OwnedBitmap {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BitOwnedBitmapmap({} bytes)", self.data.len())
    }
}

#[cfg(feature = "alloc")]
impl Deref for OwnedBitmap {
    type Target = Bitmap;

    #[inline]
    fn deref(&self) -> &Self::Target {
        Bitmap::new(&self.data)
    }
}

/// A borrowed bitmap type.
#[derive(PartialEq, Eq)]
#[repr(transparent)]
pub struct Bitmap {
    data: [u8],
}

impl Bitmap {
    /// Construct a new `Bitmap` from a slice of bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::Bitmap;
    /// let bitmap = Bitmap::new(b"hello world");
    /// assert_eq!(bitmap.as_bytes(), b"hello world");
    /// ```
    #[inline]
    pub fn new(data: &[u8]) -> &Self {
        unsafe { &*(data as *const [u8] as *const Self) }
    }

    /// Get the underlying bytes of the bitmap.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::Bitmap;
    /// let bitmap = Bitmap::new(b"hello world");
    /// assert_eq!(bitmap.as_bytes(), b"hello world");
    /// ```
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }
}

impl fmt::Debug for Bitmap {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Bitmap({} bytes)", self.data.len())
    }
}

/// Perform a partial comparison between a bitmap and bytes.
///
/// # Examples
///
/// ```
/// use pod::Bitmap;
///
/// let bitmap = Bitmap::new(b"hello world");
/// assert_eq!(bitmap, &b"hello world"[..]);
/// # Ok::<_, pod::Error>(())
/// ```
impl PartialEq<[u8]> for Bitmap {
    #[inline]
    fn eq(&self, other: &[u8]) -> bool {
        &self.data == other
    }
}

/// Perform a partial comparison between a bitmap and a byte array.
///
/// # Examples
///
/// ```
/// use pod::Bitmap;
///
/// let bitmap = Bitmap::new(b"hello world");
/// assert_eq!(bitmap, b"hello world");
/// # Ok::<_, pod::Error>(())
/// ```
impl<const N: usize> PartialEq<[u8; N]> for Bitmap {
    #[inline]
    fn eq(&self, other: &[u8; N]) -> bool {
        &self.data == other
    }
}
