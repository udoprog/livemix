use core::fmt;

#[cfg(feature = "alloc")]
use crate::DynamicBuf;
#[cfg(feature = "alloc")]
use crate::buf::AllocError;
use crate::error::ErrorKind;
use crate::{
    AsReader, DecodeFrom, EncodeUnsized, Error, PADDING, PackedPod, Pod, ReadPodKind, Reader,
    SliceBuf, Type, TypedPod, Writer,
};

/// A decoder for a struct.
pub struct Struct<B> {
    buf: B,
}

impl<B> Struct<B> {
    /// Get a reference to the underlying buffer.
    #[inline]
    pub fn as_buf(&self) -> &B {
        &self.buf
    }
}

impl<'de, B> Struct<B>
where
    B: Reader<'de>,
{
    #[inline]
    pub(crate) fn new(buf: B) -> Self {
        Self { buf }
    }

    /// Test if the decoder is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut pod = pod::array();
    /// pod.as_mut().push_struct(|st| {
    ///     st.field().push(1i32)?;
    ///     st.field().push(2i32)?;
    ///     st.field().push(3i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let mut st = pod.as_ref().next_struct()?;
    ///
    /// assert!(!st.is_empty());
    /// assert_eq!(st.field()?.next::<i32>()?, 1i32);
    /// assert_eq!(st.field()?.next::<i32>()?, 2i32);
    /// assert_eq!(st.field()?.next::<i32>()?, 3i32);
    /// assert!(st.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    /// Decode from the struct using the [`DecodeFrom`] trait.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut pod = pod::array();
    /// pod.as_mut().push_struct(|st| {
    ///     st.field().push(1i32)?;
    ///     st.field().push(2i32)?;
    ///     st.field().push(3i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let mut st = pod.as_ref().next_struct()?;
    ///
    /// assert!(!st.is_empty());
    /// assert_eq!(st.field()?.next::<i32>()?, 1i32);
    /// assert_eq!(st.field()?.next::<i32>()?, 2i32);
    /// assert_eq!(st.field()?.next::<i32>()?, 3i32);
    /// assert!(st.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn decode<T>(&mut self) -> Result<T, Error>
    where
        T: DecodeFrom<'de>,
    {
        T::decode_from(Pod::new(self.buf.borrow_mut()))
    }

    /// Decode the next field in the struct.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut pod = pod::array();
    /// pod.as_mut().push_struct(|st| {
    ///     st.field().push(1i32)?;
    ///     st.field().push(2i32)?;
    ///     st.field().push(3i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let mut st = pod.as_ref().next_struct()?;
    ///
    /// assert!(!st.is_empty());
    /// assert_eq!(st.field()?.next::<i32>()?, 1i32);
    /// assert_eq!(st.field()?.next::<i32>()?, 2i32);
    /// assert_eq!(st.field()?.next::<i32>()?, 3i32);
    /// assert!(st.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn field(&mut self) -> Result<TypedPod<B::Split, PackedPod>, Error> {
        if self.buf.is_empty() {
            return Err(Error::new(ErrorKind::StructUnderflow));
        }

        let (size, ty) = self.buf.header()?;

        let Some(head) = self.buf.split(size) else {
            return Err(Error::new(ErrorKind::BufferUnderflow));
        };

        let pod = TypedPod::packed(head, size, ty);
        self.buf.unpad(PADDING)?;
        Ok(pod)
    }

    /// Coerce into an owned [`Struct`].
    ///
    /// # Examples
    ///
    /// ```
    /// let mut pod = pod::array();
    /// pod.as_mut().push_struct(|st| {
    ///     st.field().push(1i32)?;
    ///     st.field().push(2i32)?;
    ///     st.field().push(3i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let st = pod.as_ref().next_struct()?.to_owned()?;
    ///
    /// let mut st = st.as_ref();
    ///
    /// assert!(!st.is_empty());
    /// assert_eq!(st.field()?.next::<i32>()?, 1i32);
    /// assert_eq!(st.field()?.next::<i32>()?, 2i32);
    /// assert_eq!(st.field()?.next::<i32>()?, 3i32);
    /// assert!(st.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[cfg(feature = "alloc")]
    #[inline]
    pub fn to_owned(&self) -> Result<Struct<DynamicBuf>, AllocError> {
        Ok(Struct {
            buf: DynamicBuf::from_slice(self.buf.as_bytes())?,
        })
    }

    #[inline]
    fn into_slice(self) -> Struct<SliceBuf<'de>> {
        Struct {
            buf: self.buf.as_slice(),
        }
    }
}

impl<B> Struct<B>
where
    B: AsReader,
{
    /// Coerce into an owned [`Struct`].
    ///
    /// Decoding this object does not affect the original object.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut pod = pod::array();
    /// pod.as_mut().push_struct(|st| {
    ///     st.field().push(1i32)?;
    ///     st.field().push(2i32)?;
    ///     st.field().push(3i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let st = pod.as_ref().next_struct()?.to_owned()?;
    /// let mut st = st.as_ref();
    ///
    /// assert!(!st.is_empty());
    /// assert_eq!(st.field()?.next::<i32>()?, 1i32);
    /// assert_eq!(st.field()?.next::<i32>()?, 2i32);
    /// assert_eq!(st.field()?.next::<i32>()?, 3i32);
    /// assert!(st.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn as_ref(&self) -> Struct<B::AsReader<'_>> {
        Struct::new(self.buf.as_reader())
    }
}

/// [`Encode`] implementation for [`Struct`].
///
/// # Examples
///
/// ```
/// let mut pod = pod::array();
/// pod.as_mut().push_struct(|st| {
///     st.field().push(1i32)?;
///     st.field().push(2i32)?;
///     st.field().push(3i32)?;
///     Ok(())
/// })?;
///
/// let st = pod.as_ref().next_struct()?;
///
/// let mut pod2 = pod::array();
/// pod2.as_mut().encode(st)?;
///
/// let mut st = pod2.as_ref().next_struct()?;
///
/// assert!(!st.is_empty());
/// assert_eq!(st.field()?.next::<i32>()?, 1i32);
/// assert_eq!(st.field()?.next::<i32>()?, 2i32);
/// assert_eq!(st.field()?.next::<i32>()?, 3i32);
/// assert!(st.is_empty());
/// # Ok::<_, pod::Error>(())
/// ```
impl<B> EncodeUnsized for Struct<B>
where
    B: AsReader,
{
    const TYPE: Type = Type::STRUCT;

    #[inline]
    fn size(&self) -> usize {
        self.buf.as_reader().len()
    }

    #[inline]
    fn write_content(&self, mut writer: impl Writer) -> Result<(), Error> {
        writer.write(self.buf.as_reader().as_bytes())
    }
}

crate::macros::encode_into_unsized!(impl [B] Struct<B> where B: AsReader);

impl<'de> DecodeFrom<'de> for Struct<SliceBuf<'de>> {
    #[inline]
    fn decode_from(pod: Pod<impl Reader<'de>, impl ReadPodKind>) -> Result<Self, Error> {
        Ok(pod.next_struct()?.into_slice())
    }
}

impl<B> fmt::Debug for Struct<B>
where
    B: AsReader,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct Fields<'a, B>(&'a Struct<B>);

        impl<B> fmt::Debug for Fields<'_, B>
        where
            B: AsReader,
        {
            #[inline]
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let mut this = self.0.as_ref();

                let mut f = f.debug_list();

                while !this.is_empty() {
                    match this.field() {
                        Ok(field) => {
                            f.entry(&field);
                        }
                        Err(e) => {
                            f.entry(&e);
                        }
                    }
                }

                f.finish()
            }
        }

        let mut f = f.debug_struct("Struct");
        f.field("fields", &Fields(self));
        f.finish()
    }
}
