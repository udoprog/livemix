use crate::error::ErrorKind;
use crate::{ArrayBuf, Error, Pod, Reader};

/// Helper trait to more easily encode values into a [`Pod`].
///
/// This is used through the [`Pod::decode`] and similar methods.
pub trait DecodeFrom<'de>
where
    Self: Sized,
{
    #[doc(hidden)]
    fn decode_from(pod: Pod<impl Reader<'de, u64>>) -> Result<Self, Error>;
}

/// Implementation of [`DecodeFrom`] for an optional type.
///
/// # Examples
/// ```
/// use pod::Builder;
///
/// let mut pod = Builder::array();
/// pod.as_mut().push(42u32)?;
/// assert_eq!(pod.as_ref().decode::<Option<u32>>()?, Some(42));
/// # Ok::<_, pod::Error>(())
/// ```
impl<'de, T> DecodeFrom<'de> for Option<T>
where
    T: DecodeFrom<'de>,
{
    #[inline]
    fn decode_from(pod: Pod<impl Reader<'de, u64>>) -> Result<Self, Error> {
        match pod.next_option()? {
            Some(pod) => Ok(Some(T::decode_from(pod)?)),
            None => Ok(None),
        }
    }
}

/// Implementation of [`DecodeFrom`] for an array.
///
/// # Examples
///
/// ```1```
impl<'de, T, const N: usize> DecodeFrom<'de> for [T; N]
where
    T: DecodeFrom<'de>,
{
    #[inline]
    fn decode_from(mut pod: Pod<impl Reader<'de, u64>>) -> Result<Self, Error> {
        let mut values = ArrayBuf::<T, N>::new();

        for _ in 0..N {
            values.push(T::decode_from(pod.as_read_mut())?)?;
        }

        let Some(values) = values.into_inner() else {
            return Err(Error::new(ErrorKind::InvalidArrayLength));
        };

        Ok(values)
    }
}

/// Implementation of [`DecodeFrom`] for the empty tuple, which will be encoded
/// as an empty struct.
///
/// # Examples
///
/// ```
/// let mut pod = pod::array();
/// pod.as_mut().push_struct(|st| st.encode(()))?;
///
/// let mut pod = pod.as_ref();
/// let () = pod.next_struct()?.decode::<()>()?;
/// # Ok::<_, pod::Error>(())
/// ```
impl<'de> DecodeFrom<'de> for () {
    #[inline]
    fn decode_from(_: Pod<impl Reader<'de, u64>>) -> Result<(), Error> {
        Ok(())
    }
}

macro_rules! encode_into_tuple {
    ($count:expr $(, $ident:ident, $var:ident)*) => {
        /// Implementation of [`DecodeFrom`] for tuples, which will be encoded as a struct.
        ///
        /// # Examples
        ///
        /// ```
        /// let mut pod = pod::array();
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
        impl<'de, $($ident,)*> DecodeFrom<'de> for ($($ident,)*)
        where
            $($ident: DecodeFrom<'de>,)*
        {
            #[inline]
            fn decode_from(mut pod: Pod<impl Reader<'de, u64>>) -> Result<Self, Error> {
                $(let $var = $ident::decode_from(pod.as_read_mut())?;)*
                Ok(($($var,)*))
            }
        }
    };
}

crate::macros::repeat_tuple!(encode_into_tuple);
