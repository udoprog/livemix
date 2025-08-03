use core::fmt;

#[cfg(feature = "alloc")]
use crate::DynamicBuf;
#[cfg(feature = "alloc")]
use crate::buf::AllocError;
use crate::error::ErrorKind;
use crate::{
    AsSlice, Error, PADDING, PackedPod, Pod, PodItem, PodStream, Readable, Reader, Slice, Type,
    TypedPod, UnsizedWritable, Writer,
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

impl<B> Struct<B> {
    #[inline]
    pub(crate) fn new(buf: B) -> Self {
        Self { buf }
    }
}

impl<'de, B> Struct<B>
where
    B: Reader<'de>,
{
    /// Test if the decoder is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut pod = pod::array();
    /// pod.as_mut().write_struct(|st| {
    ///     st.field().write(1i32)?;
    ///     st.field().write(2i32)?;
    ///     st.field().write(3i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let mut st = pod.as_ref().read_struct()?;
    ///
    /// assert!(!st.is_empty());
    /// assert_eq!(st.field()?.read_sized::<i32>()?, 1i32);
    /// assert_eq!(st.field()?.read_sized::<i32>()?, 2i32);
    /// assert_eq!(st.field()?.read_sized::<i32>()?, 3i32);
    /// assert!(st.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    /// Read from the [`Struct`] using the [`Readable`] trait.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut pod = pod::array();
    /// pod.as_mut().write_struct(|st| {
    ///     st.field().write(1i32)?;
    ///     st.field().write(2i32)?;
    ///     st.field().write(3i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let mut st = pod.as_ref().read_struct()?;
    /// let (a, b, c) = st.read::<(i32, i32, i32)>()?;
    /// assert_eq!(a, 1);
    /// assert_eq!(b, 2);
    /// assert_eq!(c, 3);
    /// assert!(st.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn read<T>(&mut self) -> Result<T, Error>
    where
        T: Readable<'de>,
    {
        T::read_from(&mut Pod::new(self.buf.borrow_mut()))
    }

    /// Read the next field in the struct.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut pod = pod::array();
    /// pod.as_mut().write_struct(|st| {
    ///     st.field().write(1i32)?;
    ///     st.field().write(2i32)?;
    ///     st.field().write(3i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let mut st = pod.as_ref().read_struct()?;
    ///
    /// assert!(!st.is_empty());
    /// assert_eq!(st.field()?.read_sized::<i32>()?, 1i32);
    /// assert_eq!(st.field()?.read_sized::<i32>()?, 2i32);
    /// assert_eq!(st.field()?.read_sized::<i32>()?, 3i32);
    /// assert!(st.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn field(&mut self) -> Result<TypedPod<Slice<'de>, PackedPod>, Error> {
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
    /// pod.as_mut().write_struct(|st| {
    ///     st.field().write(1i32)?;
    ///     st.field().write(2i32)?;
    ///     st.field().write(3i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let st = pod.as_ref().read_struct()?.to_owned()?;
    ///
    /// let mut st = st.as_ref();
    ///
    /// assert!(!st.is_empty());
    /// assert_eq!(st.field()?.read_sized::<i32>()?, 1i32);
    /// assert_eq!(st.field()?.read_sized::<i32>()?, 2i32);
    /// assert_eq!(st.field()?.read_sized::<i32>()?, 3i32);
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
    fn into_slice(self) -> Struct<Slice<'de>> {
        Struct {
            buf: Slice::new(self.buf.as_bytes()),
        }
    }
}

impl<B> Struct<B>
where
    B: AsSlice,
{
    /// Coerce into an owned [`Struct`].
    ///
    /// Decoding this object does not affect the original object.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut pod = pod::array();
    /// pod.as_mut().write_struct(|st| {
    ///     st.field().write(1i32)?;
    ///     st.field().write(2i32)?;
    ///     st.field().write(3i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let st = pod.as_ref().read_struct()?.to_owned()?;
    /// let mut st = st.as_ref();
    ///
    /// assert!(!st.is_empty());
    /// assert_eq!(st.field()?.read_sized::<i32>()?, 1i32);
    /// assert_eq!(st.field()?.read_sized::<i32>()?, 2i32);
    /// assert_eq!(st.field()?.read_sized::<i32>()?, 3i32);
    /// assert!(st.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn as_ref(&self) -> Struct<Slice<'_>> {
        Struct::new(self.buf.as_slice())
    }
}

/// [`UnsizedWritable`] implementation for [`Struct`].
///
/// # Examples
///
/// ```
/// let mut pod = pod::array();
/// pod.as_mut().write_struct(|st| {
///     st.field().write(1i32)?;
///     st.field().write(2i32)?;
///     st.field().write(3i32)?;
///     Ok(())
/// })?;
///
/// let st = pod.as_ref().read_struct()?;
///
/// let mut pod2 = pod::array();
/// pod2.as_mut().write(st)?;
///
/// let mut st = pod2.as_ref().read_struct()?;
///
/// assert!(!st.is_empty());
/// assert_eq!(st.field()?.read_sized::<i32>()?, 1i32);
/// assert_eq!(st.field()?.read_sized::<i32>()?, 2i32);
/// assert_eq!(st.field()?.read_sized::<i32>()?, 3i32);
/// assert!(st.is_empty());
/// # Ok::<_, pod::Error>(())
/// ```
impl<B> UnsizedWritable for Struct<B>
where
    B: AsSlice,
{
    const TYPE: Type = Type::STRUCT;

    #[inline]
    fn size(&self) -> Option<usize> {
        Some(self.buf.as_slice().len())
    }

    #[inline]
    fn write_unsized(&self, mut writer: impl Writer) -> Result<(), Error> {
        writer.write(self.buf.as_slice().as_bytes())
    }
}

crate::macros::encode_into_unsized!(impl [B] Struct<B> where B: AsSlice);

impl<'de> Readable<'de> for Struct<Slice<'de>> {
    #[inline]
    fn read_from(pod: &mut impl PodStream<'de>) -> Result<Self, Error> {
        Ok(pod.next()?.read_struct()?.into_slice())
    }
}

impl<B> fmt::Debug for Struct<B>
where
    B: AsSlice,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct Fields<'a, B>(&'a Struct<B>);

        impl<B> fmt::Debug for Fields<'_, B>
        where
            B: AsSlice,
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
