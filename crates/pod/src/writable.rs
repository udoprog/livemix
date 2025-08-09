use crate::macros::{tuple_types, tuple_values};
use crate::{Error, PodSink};

/// Helper trait to more easily write value to a [`Builder`].
///
/// This is used through the [`Builder::write`] and similar methods.
///
/// This is implemented for many types, including tuples and arrays. When tuples
/// and arrays are used, they write each "contained" value in sequence. For
/// structs this means each field, for choices each choice, and so forth.
///
/// [`Builder`]: crate::Builder
/// [`Builder::write`]: crate::Builder::write
///
/// # Examples
///
/// ```
/// let mut pod = pod::array();
/// pod.as_mut().write_struct(|st| st.write((1, 2, 3)))?;
///
/// let pod = pod.as_ref();
/// assert_eq!(pod.read_struct()?.read::<(i32, i32, i32)>()?, (1, 2, 3));
/// # Ok::<_, pod::Error>(())
/// ```
pub trait Writable {
    #[doc(hidden)]
    fn write_into(&self, pod: &mut impl PodSink) -> Result<(), Error>;
}

impl<T> Writable for &T
where
    T: ?Sized + Writable,
{
    #[inline]
    fn write_into(&self, pod: &mut impl PodSink) -> Result<(), Error> {
        (*self).write_into(pod)
    }
}

/// Implementation of [`Writable`] for an array.
///
/// # Examples
///
/// ```
/// let mut pod = pod::array();
/// pod.as_mut().write([1, 2, 3])?;
/// let pod = pod.as_ref();
/// assert_eq!(pod.read::<[i32; 3]>()?, [1, 2, 3]);
/// # Ok::<_, pod::Error>(())
/// ```
impl<T, const N: usize> Writable for [T; N]
where
    T: Writable,
{
    #[inline]
    fn write_into(&self, pod: &mut impl PodSink) -> Result<(), Error> {
        for item in self {
            item.write_into(pod)?;
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
    fn write_into(&self, pod: &mut impl PodSink) -> Result<(), Error> {
        for item in self.iter() {
            item.write_into(pod)?;
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
/// let mut pod = pod::array();
/// pod.as_mut().write_struct(|st| st.write(()))?;
///
/// let mut pod = pod.as_ref();
/// let mut st = pod.read_struct()?;
/// assert!(st.is_empty());
/// # Ok::<_, pod::Error>(())
/// ```
impl Writable for () {
    #[inline]
    fn write_into(&self, _: &mut impl PodSink) -> Result<(), Error> {
        Ok(())
    }
}

macro_rules! impl_writable {
    ($count:literal $(, $ident:ident, $var:ident)*) => {
        /// Implementation of [`Writable`] for tuples, which will be encoded as a struct.
        ///
        /// # Examples
        ///
        /// ```
        /// let mut pod = pod::array();
        #[doc = concat!(" pod.as_mut().write_struct(|st| st.write(", tuple_values!($($var),*), "))?;")]
        ///
        /// let mut pod = pod.as_ref();
        /// let mut st = pod.read_struct()?;
        #[doc = concat!(" assert_eq!(st.read::<", tuple_types!($($var),*), ">()?, ", tuple_values!($($var),*), ");")]
        /// assert!(st.is_empty());
        /// # Ok::<_, pod::Error>(())
        /// ```
        impl<$($ident,)*> Writable for ($($ident,)*)
        where
            $($ident: Writable,)*
        {
            #[inline]
            fn write_into(&self, pod: &mut impl PodSink) -> Result<(), Error> {
                let ($(ref $var,)*) = *self;
                $($var.write_into(pod)?;)*
                Ok(())
            }
        }
    };
}

crate::macros::repeat_tuple!(impl_writable);
