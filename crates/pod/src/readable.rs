use crate::buf::ArrayVec;
use crate::error::ErrorKind;
use crate::macros::{tuple_types, tuple_values};
use crate::{Error, PodItem, PodStream};

/// Helper trait to more easily read values from a [`Pod`].
///
/// This is used through the [`Pod::read`] and similar methods.
///
/// This is implemented for many types, including tuples and arrays. When tuples
/// and arrays are used, they read each "contained" value in sequence. For
/// structs this means each field, for choices each choice, and so forth.
///
/// [`Pod`]: crate::Pod
/// [`Pod::read`]: crate::Pod::read
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
pub trait Readable<'de>
where
    Self: Sized,
{
    #[doc(hidden)]
    fn read_from(pod: &mut impl PodStream<'de>) -> Result<Self, Error>;
}

/// Implementation of [`Readable`] for an optional type.
///
/// # Examples
/// ```
/// use pod::Builder;
///
/// let mut pod = Builder::array();
/// pod.as_mut().write(42u32)?;
/// assert_eq!(pod.as_ref().read::<Option<u32>>()?, Some(42));
/// # Ok::<_, pod::Error>(())
/// ```
impl<'de, T> Readable<'de> for Option<T>
where
    T: Readable<'de>,
{
    #[inline]
    fn read_from(pod: &mut impl PodStream<'de>) -> Result<Self, Error> {
        match pod.next()?.read_option()? {
            Some(mut pod) => Ok(Some(T::read_from(&mut pod)?)),
            None => Ok(None),
        }
    }
}

/// Implementation of [`Readable`] for an array.
///
/// # Examples
///
/// ```1```
impl<'de, T, const N: usize> Readable<'de> for [T; N]
where
    T: Readable<'de>,
{
    #[inline]
    fn read_from(pod: &mut impl PodStream<'de>) -> Result<Self, Error> {
        let mut values = ArrayVec::<T, N>::new();

        for _ in 0..N {
            values.push(T::read_from(pod)?)?;
        }

        let Some(values) = values.into_inner() else {
            return Err(Error::new(ErrorKind::InvalidArrayLength));
        };

        Ok(values)
    }
}

/// Implementation of [`Readable`] for the empty tuple, which will be encoded
/// as an empty struct.
///
/// # Examples
///
/// ```
/// let mut pod = pod::array();
/// pod.as_mut().write_struct(|st| st.write(()))?;
///
/// let mut pod = pod.as_ref();
/// let () = pod.read_struct()?.read::<()>()?;
/// # Ok::<_, pod::Error>(())
/// ```
impl<'de> Readable<'de> for () {
    #[inline]
    fn read_from(_: &mut impl PodStream<'de>) -> Result<(), Error> {
        Ok(())
    }
}

macro_rules! encode_into_tuple {
    ($count:expr $(, $ident:ident, $var:ident)*) => {
        /// Implementation of [`Readable`] for tuples, which will be encoded as a struct.
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
        impl<'de, $($ident,)*> Readable<'de> for ($($ident,)*)
        where
            $($ident: Readable<'de>,)*
        {
            #[inline]
            fn read_from(pod: &mut impl PodStream<'de>) -> Result<Self, Error> {
                $(let $var = $ident::read_from(pod)?;)*
                Ok(($($var,)*))
            }
        }
    };
}

crate::macros::repeat_tuple!(encode_into_tuple);
