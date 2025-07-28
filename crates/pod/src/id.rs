/// Helper trait for dealing with raw identifiers.
///
/// This allows identifiers types to be used with the [`Id`] helper type.
pub trait RawId
where
    Self: Copy,
{
    /// Convert into a numerical identifier.
    #[doc(hidden)]
    fn into_id(self) -> u32;

    /// Convert an `Id` into the underlying type.
    #[doc(hidden)]
    fn from_id(id: u32) -> Self
    where
        Self: Sized;
}

impl RawId for u32 {
    #[inline]
    fn into_id(self) -> u32 {
        self
    }

    #[inline]
    fn from_id(id: u32) -> Self {
        id
    }
}

/// Helper type that can be used to encode and decode identifiers, including raw
/// ones based on `u32`.
///
/// # Examples
///
/// ```
/// use pod::{Pod, Id};
///
/// let mut pod = Pod::array();
/// pod.as_mut().push(Id(142u32))?;
/// assert_eq!(pod.next::<Id<u32>>()?, Id(142u32));
/// # Ok::<_, pod::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Id<T>(pub T);
