use core::fmt;

#[cfg(feature = "alloc")]
use alloc::boxed::Box;

use crate::error::ErrorKind;
use crate::{AsReader, Encode, Error, Reader, Type, TypedPod, Writer};

/// A decoder for a struct.
pub struct Struct<B> {
    buf: B,
    size: usize,
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
    B: Reader<'de, u64>,
{
    #[inline]
    pub(crate) fn new(buf: B, size: usize) -> Self {
        Self { buf, size }
    }

    /// Test if the decoder is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::Pod;
    ///
    /// let mut pod = Pod::array();
    /// pod.as_mut().push_struct(|st| {
    ///     st.field()?.push(1i32)?;
    ///     st.field()?.push(2i32)?;
    ///     st.field()?.push(3i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let mut st = pod.decode_struct()?;
    /// assert!(!st.is_empty());
    /// assert_eq!(st.field()?.decode::<i32>()?, 1i32);
    /// assert_eq!(st.field()?.decode::<i32>()?, 2i32);
    /// assert_eq!(st.field()?.decode::<i32>()?, 3i32);
    /// assert!(st.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.size == 0
    }

    /// Decode the next field in the struct.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::Pod;
    ///
    /// let mut pod = Pod::array();
    /// pod.as_mut().push_struct(|st| {
    ///     st.field()?.push(1i32)?;
    ///     st.field()?.push(2i32)?;
    ///     st.field()?.push(3i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let mut st = pod.decode_struct()?;
    ///
    /// assert!(!st.is_empty());
    /// assert_eq!(st.field()?.decode::<i32>()?, 1i32);
    /// assert_eq!(st.field()?.decode::<i32>()?, 2i32);
    /// assert_eq!(st.field()?.decode::<i32>()?, 3i32);
    /// assert!(st.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn field(&mut self) -> Result<TypedPod<B::Reader<'_>>, Error> {
        if self.size == 0 {
            return Err(Error::new(ErrorKind::StructUnderflow));
        }

        let (size, ty) = self.buf.header()?;

        let head = self.buf.split(size)?;
        let pod = TypedPod::new(size, ty, head);

        let Some(size_with_header) = pod.size_with_header() else {
            return Err(Error::new(ErrorKind::SizeOverflow));
        };

        let Some(size) = self.size.checked_sub(size_with_header) else {
            return Err(Error::new(ErrorKind::SizeUnderflow {
                size: self.size,
                sub: size_with_header,
            }));
        };

        self.size = size;
        Ok(pod)
    }

    /// Coerce into an owned [`Struct`].
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::Pod;
    ///
    /// let mut pod = Pod::array();
    /// pod.as_mut().push_struct(|st| {
    ///     st.field()?.push(1i32)?;
    ///     st.field()?.push(2i32)?;
    ///     st.field()?.push(3i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let st = pod.decode_struct()?.to_owned();
    ///
    /// let mut st = st.as_ref();
    ///
    /// assert!(!st.is_empty());
    /// assert_eq!(st.field()?.decode::<i32>()?, 1i32);
    /// assert_eq!(st.field()?.decode::<i32>()?, 2i32);
    /// assert_eq!(st.field()?.decode::<i32>()?, 3i32);
    /// assert!(st.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[cfg(feature = "alloc")]
    #[inline]
    pub fn to_owned(&self) -> Struct<Box<[u64]>> {
        Struct {
            buf: Box::from(self.buf.as_slice()),
            size: self.size,
        }
    }
}

impl<B> Struct<B>
where
    B: AsReader<u64>,
{
    /// Coerce into an owned [`Struct`].
    ///
    /// Decoding this object does not affect the original object.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::Pod;
    ///
    /// let mut pod = Pod::array();
    /// pod.as_mut().push_struct(|st| {
    ///     st.field()?.push(1i32)?;
    ///     st.field()?.push(2i32)?;
    ///     st.field()?.push(3i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let st = pod.decode_struct()?.to_owned();
    ///
    /// let mut st = st.as_ref();
    ///
    /// assert!(!st.is_empty());
    /// assert_eq!(st.field()?.decode::<i32>()?, 1i32);
    /// assert_eq!(st.field()?.decode::<i32>()?, 2i32);
    /// assert_eq!(st.field()?.decode::<i32>()?, 3i32);
    /// assert!(st.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn as_ref(&self) -> Struct<B::Reader<'_>> {
        Struct::new(self.buf.as_reader(), self.size)
    }
}

/// [`Encode`] implementation for [`Struct`].
///
/// # Examples
///
/// ```
/// use pod::Pod;
///
/// let mut pod = Pod::array();
/// pod.as_mut().push_struct(|st| {
///     st.field()?.push(1i32)?;
///     st.field()?.push(2i32)?;
///     st.field()?.push(3i32)?;
///     Ok(())
/// })?;
///
/// let st = pod.decode_struct()?;
///
/// let mut pod2 = Pod::array();
/// pod2.as_mut().push(st)?;
///
/// let mut st = pod2.decode_struct()?;
///
/// assert!(!st.is_empty());
/// assert_eq!(st.field()?.decode::<i32>()?, 1i32);
/// assert_eq!(st.field()?.decode::<i32>()?, 2i32);
/// assert_eq!(st.field()?.decode::<i32>()?, 3i32);
/// assert!(st.is_empty());
/// # Ok::<_, pod::Error>(())
/// ```
impl<B> Encode for Struct<B>
where
    B: AsReader<u64>,
{
    const TYPE: Type = Type::STRUCT;

    #[inline]
    fn size(&self) -> usize {
        self.buf.as_reader().bytes_len()
    }

    #[inline]
    fn write_content(&self, mut writer: impl Writer<u64>) -> Result<(), Error> {
        writer.write_words(self.buf.as_reader().as_slice())
    }
}

impl<B> fmt::Debug for Struct<B>
where
    B: AsReader<u64>,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct Fields<'a, B>(&'a Struct<B>);

        impl<B> fmt::Debug for Fields<'_, B>
        where
            B: AsReader<u64>,
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
