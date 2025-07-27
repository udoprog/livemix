use core::{fmt, mem};

#[cfg(feature = "alloc")]
use alloc::boxed::Box;

use crate::error::ErrorKind;
use crate::utils::array_remaining;
use crate::{AsReader, Encode, Error, Reader, Type, TypedPod, WORD_SIZE, Writer};

/// A decoder for an array.
///
/// # Examples
///
/// ```
/// use pod::{Pod, Type};
///
/// let mut pod = Pod::array();
///
/// pod.as_mut().push_array(Type::INT, |array| {
///     array.child()?.push(1i32)?;
///     array.child()?.push(2i32)?;
///     array.child()?.push(3i32)?;
///     Ok(())
/// })?;
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
/// pod.as_mut().push_unsized_array(Type::STRING, 4, |array| {
///     array.child()?.push_unsized("foo")?;
///     array.child()?.push_unsized("bar")?;
///     array.child()?.push_unsized("baz")?;
///     Ok(())
/// })?;
///
/// let mut array = pod.as_ref().decode_array()?;
/// assert!(!array.is_empty());
/// assert_eq!(array.len(), 3);
/// assert_eq!(array.item()?.decode_borrowed::<str>()?, "foo");
/// assert_eq!(array.item()?.decode_borrowed::<str>()?, "bar");
/// assert_eq!(array.item()?.decode_borrowed::<str>()?, "baz");
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
/// pod.as_mut().push_unsized_array(Type::STRING, 4, |array| {
///     array.child()?.push_unsized("foo")?;
///     array.child()?.push_unsized("bar")?;
///     array.child()?.push_unsized("baz")?;
///     Ok(())
/// })?;
///
/// let mut array = pod.as_ref().decode_array()?;
/// assert!(!array.is_empty());
/// assert_eq!(array.len(), 3);
/// assert_eq!(array.item()?.decode_borrowed::<str>()?, "foo");
/// assert_eq!(array.item()?.decode_borrowed::<str>()?, "bar");
/// assert_eq!(array.item()?.decode_borrowed::<str>()?, "baz");
/// assert!(array.is_empty());
/// assert_eq!(array.len(), 0);
/// # Ok::<_, pod::Error>(())
/// ```
pub struct Array<B> {
    buf: B,
    child_size: u32,
    child_type: Type,
    remaining: u32,
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
    /// let mut pod = Pod::array();
    ///
    /// pod.as_mut().push_array(Type::INT, |array| {
    ///     array.child()?.push(1i32)?;
    ///     Ok(())
    /// })?;
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
    /// pod.as_mut().push_array(Type::INT, |_| Ok(()))?;
    ///
    /// let mut array = pod.decode_array()?;
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
    B: Reader<'de, u64>,
{
    #[inline]
    fn new(buf: B, child_size: u32, child_type: Type, remaining: u32) -> Self {
        Self {
            buf,
            child_size,
            child_type,
            remaining,
        }
    }

    #[inline]
    pub(crate) fn from_reader(mut reader: B, size: u32) -> Result<Self, Error> {
        let (child_size, child_type) = reader.header()?;
        let remaining = array_remaining(size, child_size, WORD_SIZE)?;

        Ok(Self {
            buf: reader,
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
    /// let mut pod = Pod::array();
    /// pod.as_mut().push_array(Type::INT, |array| {
    ///     array.child()?.push(1i32)?;
    ///     array.child()?.push(2i32)?;
    ///     array.child()?.push(3i32)?;
    ///     Ok(())
    /// })?;
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
    pub fn item(&mut self) -> Result<TypedPod<B::Reader<'_>>, Error> {
        if self.remaining == 0 {
            return Err(Error::new(ErrorKind::ArrayUnderflow));
        }

        let tail = self.buf.split(self.child_size)?;

        let pod = TypedPod::new(self.child_size, self.child_type, tail);
        self.remaining -= 1;
        Ok(pod)
    }

    /// Coerce into an owned [`Array`].
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = Pod::array();
    /// pod.as_mut().push_array(Type::INT, |array| {
    ///     array.child()?.push(1i32)?;
    ///     array.child()?.push(2i32)?;
    ///     array.child()?.push(3i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let array = pod.decode_array()?.to_owned();
    /// let mut array = array.as_ref();
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
    #[cfg(feature = "alloc")]
    #[inline]
    pub fn to_owned(&self) -> Array<Box<[u64]>> {
        Array {
            buf: Box::from(self.buf.as_slice()),
            child_size: self.child_size,
            child_type: self.child_type,
            remaining: self.remaining,
        }
    }
}

impl<B> Array<B>
where
    B: AsReader<u64>,
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
    /// let mut pod = Pod::array();
    /// pod.as_mut().push_array(Type::INT, |array| {
    ///     array.child()?.push(1i32)?;
    ///     array.child()?.push(2i32)?;
    ///     array.child()?.push(3i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let array = pod.decode_array()?.to_owned();
    /// let mut array = array.as_ref();
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
    pub fn as_ref(&self) -> Array<B::Reader<'_>> {
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
/// let mut pod = Pod::array();
///
/// pod.as_mut().push_array(Type::INT, |array| {
///     array.child()?.push(1i32)?;
///     array.child()?.push(2i32)?;
///     array.child()?.push(3i32)?;
///     Ok(())
/// })?;
///
/// let array = pod.decode_array()?;
/// let mut pod2 = Pod::array();
/// pod2.as_mut().push(array)?;
///
/// let mut array = pod2.decode_array()?;
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
impl<B> Encode for Array<B>
where
    B: AsReader<u64>,
{
    const TYPE: Type = Type::ARRAY;

    #[inline]
    fn size(&self) -> u32 {
        (self
            .buf
            .as_reader()
            .as_slice()
            .len()
            .wrapping_mul(mem::size_of::<u64>()) as u32)
            .wrapping_add(WORD_SIZE)
    }

    #[inline]
    fn write(&self, mut writer: impl Writer<u64>) -> Result<(), Error> {
        let data = self.buf.as_reader();
        let data = data.as_slice();

        let size = data
            .len()
            .wrapping_mul(mem::size_of::<u64>())
            .wrapping_add(WORD_SIZE as usize);

        let Ok(size) = u32::try_from(size) else {
            return Err(Error::new(ErrorKind::SizeOverflow));
        };

        writer.write([
            size,
            Type::ARRAY.into_u32(),
            self.child_size,
            self.child_type.into_u32(),
        ])?;

        writer.write_words(data.as_slice())?;
        Ok(())
    }

    #[inline]
    fn write_content(&self, mut writer: impl Writer<u64>) -> Result<(), Error> {
        writer.write([self.child_size, self.child_type.into_u32()])?;
        writer.write_words(self.buf.as_reader().as_slice())
    }
}

impl<B> fmt::Debug for Array<B>
where
    B: AsReader<u64>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct Entries<'a, B>(&'a Array<B>);

        impl<B> fmt::Debug for Entries<'_, B>
        where
            B: AsReader<u64>,
        {
            #[inline]
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let mut this = self.0.as_ref();

                let mut f = f.debug_list();

                while !this.is_empty() {
                    match this.item() {
                        Ok(e) => {
                            f.entry(&e);
                        }
                        Err(e) => {
                            f.entry(&e);
                        }
                    }
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
