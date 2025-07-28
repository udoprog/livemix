use crate::{Encode, Error, Pod, PodKind, Writer};

/// Helper trait to more easily encode values into a [`Pod`].
///
/// This is used through the [`Pod::encode`] method.
///
/// The *special* behaviors implemented by this trait are:
/// - For tuples, it encodes them as a struct as long as each item implements
///   [`EncodeInto`].
/// - For fixed sized arrays, it encodes them as an array as long as `T`
///   implements [`Encode`].
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

/// Implementation of [`EncodeInto`] for the empty tuple, which will be encoded
/// as an array.
///
/// # Examples
impl<T, const N: usize> EncodeInto for [T; N]
where
    T: Encode + EncodeInto,
{
    #[inline]
    fn encode_into(&self, pod: Pod<impl Writer<u64>, impl PodKind>) -> Result<(), Error> {
        pod.push_array(T::TYPE, |array| {
            for item in self {
                item.encode_into(array.child())?;
            }

            Ok(())
        })
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
/// pod.as_mut().encode(())?;
///
/// let mut pod = pod.as_ref();
/// let st = pod.next_struct()?;
/// assert!(st.is_empty());
/// # Ok::<_, pod::Error>(())
/// ```
impl EncodeInto for () {
    #[inline]
    fn encode_into(&self, pod: Pod<impl Writer<u64>, impl PodKind>) -> Result<(), Error> {
        pod.push_struct(|_| Ok(()))
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
        /// pod.as_mut().encode((10i32, "hello world", [1u32, 2u32]))?;
        ///
        /// let mut pod = pod.as_ref();
        /// let mut st = pod.next_struct()?;
        ///
        /// assert_eq!(st.field()?.next::<i32>()?, 10i32);
        /// assert_eq!(st.field()?.next_borrowed::<str>()?, "hello world");
        ///
        /// let mut array = st.field()?.next_array()?;
        /// assert_eq!(array.next().unwrap().next::<u32>()?, 1);
        /// assert_eq!(array.next().unwrap().next::<u32>()?, 2);
        /// assert!(st.is_empty());
        /// # Ok::<_, pod::Error>(())
        /// ```
        impl<$($ident,)*> EncodeInto for ($($ident,)*)
        where
            $($ident: EncodeInto,)*
        {
            #[inline]
            fn encode_into(&self, pod: Pod<impl Writer<u64>, impl PodKind>) -> Result<(), Error> {
                pod.push_struct(|st| {
                    let ($(ref $var,)*) = *self;
                    $($var.encode_into(st.field())?;)*
                    Ok(())
                })
            }
        }
    };
}

crate::macros::repeat_tuple!(encode_into_tuple);
