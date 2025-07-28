use crate::{Error, Pod, PodKind, Writer};

/// Helper trait to more easily encode values into a [`Pod`].
///
/// This is used through the [`Pod::encode`] and similar methods.
pub trait EncodeInto {
    #[doc(hidden)]
    fn encode_into(&self, pod: Pod<impl Writer<u64>, impl PodKind>) -> Result<(), Error>;
}

impl<T> EncodeInto for &T
where
    T: ?Sized + EncodeInto,
{
    #[inline]
    fn encode_into(&self, pod: Pod<impl Writer<u64>, impl PodKind>) -> Result<(), Error> {
        (*self).encode_into(pod)
    }
}

/// Implementation of [`EncodeInto`] for an array.
///
/// # Examples
///
/// ```
/// use pod::Pod;
/// ```
impl<T, const N: usize> EncodeInto for [T; N]
where
    T: EncodeInto,
{
    #[inline]
    fn encode_into(&self, mut pod: Pod<impl Writer<u64>, impl PodKind>) -> Result<(), Error> {
        for item in self {
            item.encode_into(pod.as_mut())?;
        }

        Ok(())
    }
}

/// Implementation of [`EncodeInto`] for the empty tuple, which will be encoded
/// as an array.
///
/// # Examples
impl<T> EncodeInto for &[T]
where
    T: EncodeInto,
{
    #[inline]
    fn encode_into(&self, mut pod: Pod<impl Writer<u64>, impl PodKind>) -> Result<(), Error> {
        for item in self.iter() {
            item.encode_into(pod.as_mut())?;
        }

        Ok(())
    }
}

/// Implementation of [`EncodeInto`] for the empty tuple, which will be encoded
/// as an empty struct.
///
/// # Examples
///
/// ```
/// use pod::Pod;
///
/// let mut pod = Pod::array();
/// pod.as_mut().push_struct(|st| st.encode(()))?;
///
/// let mut pod = pod.as_ref();
/// let mut st = pod.next_struct()?;
/// assert!(st.is_empty());
/// # Ok::<_, pod::Error>(())
/// ```
impl EncodeInto for () {
    #[inline]
    fn encode_into(&self, _: Pod<impl Writer<u64>, impl PodKind>) -> Result<(), Error> {
        Ok(())
    }
}

macro_rules! encode_into_tuple {
    ($count:expr $(, $ident:ident, $var:ident)*) => {
        /// Implementation of [`EncodeInto`] for tuples, which will be encoded as a struct.
        ///
        /// # Examples
        ///
        /// ```
        /// use pod::Pod;
        ///
        /// let mut pod = Pod::array();
        /// pod.as_mut().push_struct(|st| st.encode((10i32, "hello world", [1u32, 2u32])))?;
        ///
        /// let mut pod = pod.as_ref();
        /// let mut st = pod.next_struct()?;
        ///
        /// assert_eq!(st.field()?.next::<i32>()?, 10i32);
        /// assert_eq!(st.field()?.next_borrowed::<str>()?, "hello world");
        /// assert_eq!(st.field()?.next::<u32>()?, 1);
        /// assert_eq!(st.field()?.next::<u32>()?, 2);
        /// assert!(st.is_empty());
        /// # Ok::<_, pod::Error>(())
        /// ```
        impl<$($ident,)*> EncodeInto for ($($ident,)*)
        where
            $($ident: EncodeInto,)*
        {
            #[inline]
            fn encode_into(&self, mut pod: Pod<impl Writer<u64>, impl PodKind>) -> Result<(), Error> {
                let ($(ref $var,)*) = *self;
                $($var.encode_into(pod.as_mut())?;)*
                Ok(())
            }
        }
    };
}

crate::macros::repeat_tuple!(encode_into_tuple);
