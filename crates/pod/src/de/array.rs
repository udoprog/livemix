use core::fmt;
use core::mem;

#[cfg(feature = "alloc")]
use crate::DynamicBuf;
#[cfg(feature = "alloc")]
use crate::buf::AllocError;
use crate::error::ErrorKind;
use crate::utils::array_remaining;
use crate::{AsReader, EncodeUnsized, Error, PackedPod, Reader, Type, TypedPod, Writer};

/// A decoder for an array.
///
/// # Examples
///
/// ```
/// use pod::{Pod, Type};
///
/// let mut pod = pod::array();
///
/// pod.as_mut().push_array(Type::INT, |array| {
///     array.child().push(1i32)?;
///     array.child().push(2i32)?;
///     array.child().push(3i32)?;
///     Ok(())
/// })?;
///
/// let mut array = pod.as_ref().next_array()?;
///
/// assert!(!array.is_empty());
/// assert_eq!(array.len(), 3);
///
/// assert_eq!(array.next().unwrap().next::<i32>()?, 1i32);
/// assert_eq!(array.next().unwrap().next::<i32>()?, 2i32);
/// assert_eq!(array.next().unwrap().next::<i32>()?, 3i32);
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
/// let mut pod = pod::array();
/// pod.as_mut().push_unsized_array(Type::STRING, 4, |array| {
///     array.child().push_unsized("foo")?;
///     array.child().push_unsized("bar")?;
///     array.child().push_unsized("baz")?;
///     Ok(())
/// })?;
///
/// let mut array = pod.as_ref().next_array()?;
///
/// assert!(!array.is_empty());
/// assert_eq!(array.len(), 3);
/// assert_eq!(array.next().unwrap().next_unsized::<str>()?, "foo");
/// assert_eq!(array.next().unwrap().next_unsized::<str>()?, "bar");
/// assert_eq!(array.next().unwrap().next_unsized::<str>()?, "baz");
/// assert!(array.is_empty());
/// assert_eq!(array.len(), 0);
/// # Ok::<_, pod::Error>(())
/// ```
pub struct Array<B> {
    buf: B,
    child_size: usize,
    child_type: Type,
    remaining: usize,
}

impl<B> Array<B> {
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
    /// let mut pod = pod::array();
    ///
    /// pod.as_mut().push_array(Type::INT, |array| {
    ///     array.child().push(1i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let mut array = pod.as_ref().next_array()?;
    /// assert_eq!(array.len(), 1);
    /// assert!(!array.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn len(&self) -> usize {
        self.remaining
    }

    /// Check if the array is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = pod::array();
    /// pod.as_mut().push_array(Type::INT, |_| Ok(()))?;
    ///
    /// let mut array = pod.as_ref().next_array()?;
    /// assert!(array.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.remaining == 0
    }

    /// Get a reference to the underlying buffer.
    #[inline]
    pub fn as_buf(&self) -> &B {
        &self.buf
    }
}

impl<'de, B> Array<B>
where
    B: Reader<'de>,
{
    #[inline]
    fn new(buf: B, child_size: usize, child_type: Type, remaining: usize) -> Self {
        Self {
            buf,
            child_size,
            child_type,
            remaining,
        }
    }

    #[inline]
    pub(crate) fn from_reader(mut buf: B) -> Result<Self, Error> {
        let (child_size, child_type) = buf.header()?;
        let remaining = array_remaining(buf.len(), child_size)?;

        Ok(Self {
            buf,
            child_size,
            child_type,
            remaining,
        })
    }

    /// Get the next element in the array.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = pod::array();
    /// pod.as_mut().push_array(Type::INT, |array| {
    ///     array.child().push(1i32)?;
    ///     array.child().push(2i32)?;
    ///     array.child().push(3i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let mut array = pod.as_ref().next_array()?;
    ///
    /// let mut count = 0;
    ///
    /// while !array.is_empty() {
    ///     let pod = array.next().unwrap();
    ///     assert_eq!(pod.ty(), Type::INT);
    ///     assert_eq!(pod.size(), 4);
    ///     count += 1;
    /// }
    ///
    /// assert_eq!(count, 3);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn next(&mut self) -> Option<TypedPod<B::Split, PackedPod>> {
        if self.remaining == 0 {
            return None;
        }

        let tail = self.buf.split(self.child_size)?;
        let pod = TypedPod::packed(tail, self.child_size, self.child_type);
        self.remaining -= 1;
        Some(pod)
    }

    /// Coerce into an owned [`Array`].
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = pod::array();
    /// pod.as_mut().push_array(Type::INT, |array| {
    ///     array.child().push(1i32)?;
    ///     array.child().push(2i32)?;
    ///     array.child().push(3i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let array = pod.as_ref().next_array()?.to_owned()?;
    /// let mut array = array.as_ref();
    ///
    /// let mut count = 0;
    ///
    /// while !array.is_empty() {
    ///     let pod = array.next().unwrap();
    ///     assert_eq!(pod.ty(), Type::INT);
    ///     assert_eq!(pod.size(), 4);
    ///     count += 1;
    /// }
    ///
    /// assert_eq!(count, 3);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[cfg(feature = "alloc")]
    #[inline]
    pub fn to_owned(&self) -> Result<Array<DynamicBuf>, AllocError> {
        Ok(Array {
            buf: DynamicBuf::from_slice(self.buf.as_bytes())?,
            child_size: self.child_size,
            child_type: self.child_type,
            remaining: self.remaining,
        })
    }
}

impl<B> Array<B>
where
    B: AsReader,
{
    /// Coerce into a borrowed [`Array`].
    ///
    /// Decoding this object does not affect the original object.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = pod::array();
    /// pod.as_mut().push_array(Type::INT, |array| {
    ///     array.child().push(1i32)?;
    ///     array.child().push(2i32)?;
    ///     array.child().push(3i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let array = pod.as_ref().next_array()?.to_owned()?;
    /// let mut array = array.as_ref();
    ///
    /// let mut count = 0;
    ///
    /// while let Some(pod) = array.next() {
    ///     assert_eq!(pod.ty(), Type::INT);
    ///     assert_eq!(pod.size(), 4);
    ///     count += 1;
    /// }
    ///
    /// assert_eq!(count, 3);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn as_ref(&self) -> Array<B::AsReader<'_>> {
        Array::new(
            self.buf.as_reader(),
            self.child_size,
            self.child_type,
            self.remaining,
        )
    }
}

/// [`Encode`] implementation for [`Array`].
///
/// # Examples
///
/// ```
/// use pod::{Pod, Type};
///
/// let mut pod = pod::array();
///
/// pod.as_mut().push_array(Type::INT, |array| {
///     array.child().push(1i32)?;
///     array.child().push(2i32)?;
///     array.child().push(3i32)?;
///     Ok(())
/// })?;
///
/// let array = pod.as_ref().next_array()?;
/// let mut pod2 = pod::array();
/// pod2.as_mut().encode(array)?;
///
/// let mut array = pod2.as_ref().next_array()?;
///
/// assert!(!array.is_empty());
/// assert_eq!(array.len(), 3);
///
/// assert_eq!(array.next().unwrap().next::<i32>()?, 1i32);
/// assert_eq!(array.next().unwrap().next::<i32>()?, 2i32);
/// assert_eq!(array.next().unwrap().next::<i32>()?, 3i32);
///
/// assert!(array.is_empty());
/// assert_eq!(array.len(), 0);
/// # Ok::<_, pod::Error>(())
/// ```
impl<B> EncodeUnsized for Array<B>
where
    B: AsReader,
{
    const TYPE: Type = Type::ARRAY;

    #[inline]
    fn size(&self) -> usize {
        self.child_size
            .wrapping_mul(self.remaining)
            .wrapping_add(mem::size_of::<[u32; 2]>())
    }

    #[inline]
    fn write_content(&self, mut writer: impl Writer) -> Result<(), Error> {
        let Ok(child_size) = u32::try_from(self.child_size) else {
            return Err(Error::new(ErrorKind::SizeOverflow));
        };

        writer.write(&[child_size, self.child_type.into_u32()])?;
        writer.write(self.buf.as_reader().as_bytes())
    }
}

crate::macros::encode_into_unsized!(impl [B] Array<B> where B: AsReader);

impl<B> fmt::Debug for Array<B>
where
    B: AsReader,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct Entries<'a, B>(&'a Array<B>);

        impl<B> fmt::Debug for Entries<'_, B>
        where
            B: AsReader,
        {
            #[inline]
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let mut this = self.0.as_ref();

                let mut f = f.debug_list();

                while let Some(child) = this.next() {
                    f.entry(&child);
                }

                f.finish()
            }
        }

        let mut f = f.debug_struct("Array");
        f.field("child_type", &self.child_type());
        f.field("entries", &Entries(self));
        f.finish()
    }
}
