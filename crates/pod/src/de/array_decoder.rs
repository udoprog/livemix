use crate::error::ErrorKind;
use crate::utils::array_remaining;
use crate::{Error, Reader, Type, TypedPod, WORD_SIZE};

/// A decoder for an array.
///
/// # Examples
///
/// ```
/// use pod::{Pod, Type};
///
/// let mut pod = Pod::array();
/// let mut array = pod.as_mut().encode_array(Type::INT)?;
/// array.push()?.encode(1i32)?;
/// array.push()?.encode(2i32)?;
/// array.push()?.encode(3i32)?;
/// array.close()?;
///
/// let mut array = pod.decode_array()?;
///
/// assert!(!array.is_empty());
/// assert_eq!(array.len(), 3);
///
/// assert_eq!(array.item()?.decode::<i32>()?, 1i32);
/// assert_eq!(array.item()?.decode::<i32>()?, 2i32);
/// assert_eq!(array.item()?.decode::<i32>()?, 3i32);
///
/// assert!(array.is_empty());
/// assert_eq!(array.len(), 0);
/// # Ok::<_, pod::Error>(())
/// ```
///
/// Decoding unsized items:
///
/// ```
/// use pod::{Pod, Type};
///
/// let mut pod = Pod::array();
/// let mut array = pod.as_mut().encode_unsized_array(Type::STRING, 4)?;
/// array.push()?.encode_unsized("foo")?;
/// array.push()?.encode_unsized("bar")?;
/// array.push()?.encode_unsized("baz")?;
/// array.close()?;
///
/// let mut array = pod.as_ref().decode_array()?;
///
/// assert!(!array.is_empty());
/// assert_eq!(array.len(), 3);
///
/// assert_eq!(array.item()?.decode_borrowed::<str>()?, "foo");
/// assert_eq!(array.item()?.decode_borrowed::<str>()?, "bar");
/// assert_eq!(array.item()?.decode_borrowed::<str>()?, "baz");
///
/// assert!(array.is_empty());
/// assert_eq!(array.len(), 0);
/// # Ok::<_, pod::Error>(())
/// ```
///
/// Decoding borrowed values:
///
/// ```
/// use pod::{Pod, Type};
///
/// let mut pod = Pod::array();
/// let mut array = pod.as_mut().encode_unsized_array(Type::STRING, 4)?;
///
/// array.push()?.encode_unsized("foo")?;
/// array.push()?.encode_unsized("bar")?;
/// array.push()?.encode_unsized("baz")?;
///
/// array.close()?;
///
/// let mut array = pod.as_ref().decode_array()?;
///
/// assert!(!array.is_empty());
/// assert_eq!(array.len(), 3);
///
/// assert_eq!(array.item()?.decode_borrowed::<str>()?, "foo");
/// assert_eq!(array.item()?.decode_borrowed::<str>()?, "bar");
/// assert_eq!(array.item()?.decode_borrowed::<str>()?, "baz");
///
/// assert!(array.is_empty());
/// assert_eq!(array.len(), 0);
/// # Ok::<_, pod::Error>(())
/// ```
pub struct ArrayDecoder<R> {
    reader: R,
    child_size: u32,
    child_type: Type,
    remaining: u32,
}

impl<'de, R> ArrayDecoder<R>
where
    R: Reader<'de, u64>,
{
    #[inline]
    pub(crate) fn from_reader(mut reader: R, size: u32) -> Result<Self, Error> {
        let (child_size, child_type) = reader.header()?;
        let remaining = array_remaining(size, child_size, WORD_SIZE)?;

        Ok(Self {
            reader,
            child_size,
            child_type,
            remaining,
        })
    }

    /// Return the type of the child element.
    #[inline]
    pub fn child_type(&self) -> Type {
        self.child_type
    }

    /// Get the number of elements left to decode from the array.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = Pod::array();
    /// let mut array = pod.as_mut().encode_array(Type::INT)?;
    /// array.push()?.encode(1i32)?;
    /// array.close()?;
    ///
    /// let mut array = pod.decode_array()?;
    ///
    /// assert_eq!(array.len(), 1);
    /// assert!(!array.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn len(&self) -> u32 {
        self.remaining
    }

    /// Check if the array is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = Pod::array();
    /// let mut array = pod.as_mut().encode_array(Type::INT)?;
    /// array.close()?;
    ///
    /// let mut array = pod.decode_array()?;
    /// assert!(array.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.remaining == 0
    }

    /// Get the next element in the array.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = Pod::array();
    /// let mut array = pod.as_mut().encode_array(Type::INT)?;
    /// array.push()?.encode(1i32)?;
    /// array.push()?.encode(2i32)?;
    /// array.push()?.encode(3i32)?;
    /// array.close()?;
    ///
    /// let mut array = pod.decode_array()?;
    ///
    /// let mut count = 0;
    ///
    /// while !array.is_empty() {
    ///     let pod = array.item()?;
    ///     assert_eq!(pod.ty(), Type::INT);
    ///     assert_eq!(pod.size(), 4);
    ///     count += 1;
    /// }
    ///
    /// assert_eq!(count, 3);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn item(&mut self) -> Result<TypedPod<R::Clone<'_>>, Error> {
        if self.remaining == 0 {
            return Err(Error::new(ErrorKind::ArrayUnderflow));
        }

        let tail = self.reader.split(self.child_size)?;

        let pod = TypedPod::new(self.child_size, self.child_type, tail);
        self.remaining -= 1;
        Ok(pod)
    }
}
