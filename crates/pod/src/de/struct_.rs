use core::fmt;

use crate::error::ErrorKind;
use crate::{Error, Reader, TypedPod};

/// A decoder for a struct.
pub struct Struct<R> {
    reader: R,
    size: u32,
}

impl<'de, R> Struct<R>
where
    R: Reader<'de, u64>,
{
    #[inline]
    pub(crate) fn new(reader: R, size: u32) -> Self {
        Self { reader, size }
    }

    /// Test if the decoder is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::Pod;
    ///
    /// let mut pod = Pod::array();
    /// pod.as_mut().encode_struct(|st| {
    ///     st.field()?.encode(1i32)?;
    ///     st.field()?.encode(2i32)?;
    ///     st.field()?.encode(3i32)?;
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
    /// pod.as_mut().encode_struct(|st| {
    ///     st.field()?.encode(1i32)?;
    ///     st.field()?.encode(2i32)?;
    ///     st.field()?.encode(3i32)?;
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
    pub fn field(&mut self) -> Result<TypedPod<R::Clone<'_>>, Error> {
        if self.size == 0 {
            return Err(Error::new(ErrorKind::StructUnderflow));
        }

        let (size, ty) = self.reader.header()?;

        let head = self.reader.split(size)?;
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

    /// Convert the [`StructDecoder`] into a one borrowing from but without
    /// modifying the current buffer.
    #[inline]
    pub fn as_ref(&self) -> Struct<R::Clone<'_>> {
        Struct::new(self.reader.clone_reader(), self.size)
    }
}

impl<'de, R> fmt::Debug for Struct<R>
where
    R: Reader<'de, u64>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct Fields<'a, R>(&'a Struct<R>);

        impl<'de, R> fmt::Debug for Fields<'_, R>
        where
            R: Reader<'de, u64>,
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
