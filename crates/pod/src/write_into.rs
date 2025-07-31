use crate::{BuildPod, Builder, Error, Writer};

/// Helper trait to more easily encode values into a [`Builder`].
///
/// This is used through the [`Builder::encode`] and similar methods.
pub trait Writable {
    #[doc(hidden)]
    fn write_into(&self, pod: Builder<impl Writer, impl BuildPod>) -> Result<(), Error>;
}

impl<T> Writable for &T
where
    T: ?Sized + Writable,
{
    #[inline]
    fn write_into(&self, pod: Builder<impl Writer, impl BuildPod>) -> Result<(), Error> {
        (*self).write_into(pod)
    }
}

/// Implementation of [`Writable`] for an array.
///
/// # Examples
///
/// ```
/// use pod::Builder;
/// ```
impl<T, const N: usize> Writable for [T; N]
where
    T: Writable,
{
    #[inline]
    fn write_into(&self, pod: Builder<impl Writer, impl BuildPod>) -> Result<(), Error> {
        let mut pod = pod.into_envelope()?;

        for item in self {
            item.write_into(pod.as_mut())?;
        }

        Ok(())
    }
}

/// Implementation of [`Writable`] for the empty tuple, which will be encoded
/// as an array.
///
/// # Examples
impl<T> Writable for &[T]
where
    T: Writable,
{
    #[inline]
    fn write_into(&self, pod: Builder<impl Writer, impl BuildPod>) -> Result<(), Error> {
        let mut pod = pod.into_envelope()?;

        for item in self.iter() {
            item.write_into(pod.as_mut())?;
        }

        Ok(())
    }
}

/// Implementation of [`Writable`] for the empty tuple, which will be encoded
/// as an empty struct.
///
/// # Examples
///
/// ```
/// use pod::Builder;
///
/// let mut pod = Builder::array();
/// pod.as_mut().push_struct(|st| st.write(()))?;
///
/// let mut pod = pod.as_ref();
/// let mut st = pod.next_struct()?;
/// assert!(st.is_empty());
/// # Ok::<_, pod::Error>(())
/// ```
impl Writable for () {
    #[inline]
    fn write_into(&self, _: Builder<impl Writer, impl BuildPod>) -> Result<(), Error> {
        Ok(())
    }
}

macro_rules! encode_into_tuple {
    ($count:expr $(, $ident:ident, $var:ident)*) => {
        /// Implementation of [`Writable`] for tuples, which will be encoded as a struct.
        ///
        /// # Examples
        ///
        /// ```
        /// use pod::Builder;
        ///
        /// let mut pod = Builder::array();
        /// pod.as_mut().push_struct(|st| st.write((10i32, "hello world", [1u32, 2u32])))?;
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
        impl<$($ident,)*> Writable for ($($ident,)*)
        where
            $($ident: Writable,)*
        {
            #[inline]
            fn write_into(&self, pod: Builder<impl Writer, impl BuildPod>) -> Result<(), Error> {
                let ($(ref $var,)*) = *self;
                let mut pod = pod.into_envelope()?;
                $($var.write_into(pod.as_mut())?;)*
                Ok(())
            }
        }
    };
}

crate::macros::repeat_tuple!(encode_into_tuple);
