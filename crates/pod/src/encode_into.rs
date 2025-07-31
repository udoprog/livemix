use crate::{BuildPod, Builder, Error, Writer};

/// Helper trait to more easily encode values into a [`Builder`].
///
/// This is used through the [`Builder::encode`] and similar methods.
pub trait EncodeInto {
    #[doc(hidden)]
    fn encode_into(&self, pod: Builder<impl Writer, impl BuildPod>) -> Result<(), Error>;
}

impl<T> EncodeInto for &T
where
    T: ?Sized + EncodeInto,
{
    #[inline]
    fn encode_into(&self, pod: Builder<impl Writer, impl BuildPod>) -> Result<(), Error> {
        (*self).encode_into(pod)
    }
}

/// Implementation of [`EncodeInto`] for an array.
///
/// # Examples
///
/// ```
/// use pod::Builder;
/// ```
impl<T, const N: usize> EncodeInto for [T; N]
where
    T: EncodeInto,
{
    #[inline]
    fn encode_into(&self, pod: Builder<impl Writer, impl BuildPod>) -> Result<(), Error> {
        let mut pod = pod.into_envelope()?;

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
    fn encode_into(&self, pod: Builder<impl Writer, impl BuildPod>) -> Result<(), Error> {
        let mut pod = pod.into_envelope()?;

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
/// use pod::Builder;
///
/// let mut pod = Builder::array();
/// pod.as_mut().push_struct(|st| st.encode(()))?;
///
/// let mut pod = pod.as_ref();
/// let mut st = pod.next_struct()?;
/// assert!(st.is_empty());
/// # Ok::<_, pod::Error>(())
/// ```
impl EncodeInto for () {
    #[inline]
    fn encode_into(&self, _: Builder<impl Writer, impl BuildPod>) -> Result<(), Error> {
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
        /// use pod::Builder;
        ///
        /// let mut pod = Builder::array();
        /// pod.as_mut().push_struct(|st| st.encode((10i32, "hello world", [1u32, 2u32])))?;
        ///
        /// let mut pod = pod.as_ref();
        /// let mut st = pod.next_struct()?;
        ///
        /// assert_eq!(st.field()?.next::<i32>()?, 10i32);
        /// assert_eq!(st.field()?.next_unsized::<str>()?, "hello world");
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
            fn encode_into(&self, pod: Builder<impl Writer, impl BuildPod>) -> Result<(), Error> {
                let ($(ref $var,)*) = *self;
                let mut pod = pod.into_envelope()?;
                $($var.encode_into(pod.as_mut())?;)*
                Ok(())
            }
        }
    };
}

crate::macros::repeat_tuple!(encode_into_tuple);
