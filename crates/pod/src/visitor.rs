use super::Error;
use super::error::ErrorKind;

/// An unsized visitor.
///
/// This can be used to visit unsized data in a pod, allowing it to be accessed
/// without having to borrow the pod before reading it.
///
/// # Examples
///
/// This is implemented for functions or closures like `|value: &T| -> U`:
///
/// ```
/// let mut pod = pod::array();
/// pod.as_mut().push_unsized(&b"hello world"[..])?;
///
/// let pod = pod.as_ref();
/// assert_eq!(pod.visit_unsized::<[u8], _>(<[u8]>::to_owned)?, b"hello world");
/// # Ok::<_, pod::Error>(())
/// ```
pub trait Visitor<'de, T>
where
    T: ?Sized,
{
    /// The ok outcome of a visit.
    type Ok;

    /// Visit a borrowed slice of bytes with the specified lifetime.
    #[inline]
    fn visit_borrowed(self, bytes: &'de T) -> Result<Self::Ok, Error>
    where
        Self: Sized,
    {
        self.visit_ref(bytes)
    }

    /// Visit a borrowed slice of bytes.
    #[inline]
    fn visit_ref(self, _: &T) -> Result<Self::Ok, Error>
    where
        Self: Sized,
    {
        Err(Error::new(ErrorKind::NotSupportedRef))
    }
}

impl<F, T, U> Visitor<'_, T> for F
where
    F: FnOnce(&T) -> U,
    T: ?Sized,
{
    type Ok = U;

    #[inline]
    fn visit_ref(self, bytes: &T) -> Result<Self::Ok, Error> {
        Ok(self(bytes))
    }
}
