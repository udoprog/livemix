use crate::buf::ArrayVec;
use crate::error::ErrorKind;
use crate::{Error, Pod, ReadPod, Reader};

/// Helper trait to more easily encode values into a [`Pod`].
///
/// This is used through the [`Pod::decode`] and similar methods.
pub trait Readable<'de>
where
    Self: Sized,
{
    #[doc(hidden)]
    fn read_from(pod: Pod<impl Reader<'de>, impl ReadPod>) -> Result<Self, Error>;
}

/// Implementation of [`Readable`] for an optional type.
///
/// # Examples
/// ```
/// use pod::Builder;
///
/// let mut pod = Builder::array();
/// pod.as_mut().push(42u32)?;
/// assert_eq!(pod.as_ref().read::<Option<u32>>()?, Some(42));
/// # Ok::<_, pod::Error>(())
/// ```
impl<'de, T> Readable<'de> for Option<T>
where
    T: Readable<'de>,
{
    #[inline]
    fn read_from(pod: Pod<impl Reader<'de>, impl ReadPod>) -> Result<Self, Error> {
        match pod.next_option()? {
            Some(pod) => Ok(Some(T::read_from(pod)?)),
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
    fn read_from(mut pod: Pod<impl Reader<'de>, impl ReadPod>) -> Result<Self, Error> {
        let mut values = ArrayVec::<T, N>::new();

        for _ in 0..N {
            values.push(T::read_from(pod.as_mut())?)?;
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
/// pod.as_mut().push_struct(|st| st.write(()))?;
///
/// let mut pod = pod.as_ref();
/// let () = pod.next_struct()?.read::<()>()?;
/// # Ok::<_, pod::Error>(())
/// ```
impl<'de> Readable<'de> for () {
    #[inline]
    fn read_from(_: Pod<impl Reader<'de>, impl ReadPod>) -> Result<(), Error> {
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
        impl<'de, $($ident,)*> Readable<'de> for ($($ident,)*)
        where
            $($ident: Readable<'de>,)*
        {
            #[inline]
            fn read_from(mut pod: Pod<impl Reader<'de>, impl ReadPod>) -> Result<Self, Error> {
                $(let $var = $ident::read_from(pod.as_mut())?;)*
                Ok(($($var,)*))
            }
        }
    };
}

crate::macros::repeat_tuple!(encode_into_tuple);
