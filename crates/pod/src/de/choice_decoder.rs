use crate::error::ErrorKind;
use crate::utils::array_remaining;
use crate::{Choice, Error, Reader, Type, TypedPod, WORD_SIZE};

/// A decoder for a choice.
pub struct ChoiceDecoder<R> {
    reader: R,
    ty: Choice,
    #[allow(unused)]
    flags: u32,
    child_size: u32,
    child_type: Type,
    remaining: u32,
}

impl<'de, R> ChoiceDecoder<R>
where
    R: Reader<'de, u64>,
{
    #[inline]
    pub(crate) fn from_reader(mut reader: R, size: u32) -> Result<Self, Error> {
        let [ty, flags, child_size, child_type] = reader.read::<[u32; 4]>()?;
        let ty = Choice::from_u32(ty);
        let child_type = Type::new(child_type);
        let remaining = array_remaining(size, child_size, WORD_SIZE * 2)?;

        Ok(Self {
            reader,
            ty,
            flags,
            child_size,
            child_type,
            remaining,
        })
    }

    /// Return the type of the choice.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Choice, Pod, Type};
    ///
    /// let mut pod = Pod::array();
    /// let mut choice = pod.as_mut().encode_choice(Choice::RANGE, Type::INT)?;
    ///
    /// choice.entry()?.encode(10i32)?;
    /// choice.entry()?.encode(0i32)?;
    /// choice.entry()?.encode(30i32)?;
    ///
    /// choice.close()?;
    ///
    /// let mut choice = pod.decode_choice()?;
    ///
    /// assert_eq!(choice.ty(), Choice::RANGE);
    ///
    /// let mut count = 0;
    ///
    /// while !choice.is_empty() {
    ///     let pod = choice.entry()?;
    ///     assert_eq!(pod.ty(), Type::INT);
    ///     assert_eq!(pod.size(), 4);
    ///     count += 1;
    /// }
    ///
    /// assert_eq!(count, 3);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub const fn ty(&self) -> Choice {
        self.ty
    }

    /// Return the type of the child element.
    #[inline]
    pub const fn child_type(&self) -> Type {
        self.child_type
    }

    /// Return the size of the child element.
    #[inline]
    pub const fn child_size(&self) -> u32 {
        self.child_size
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
    pub const fn len(&self) -> u32 {
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
    ///
    /// assert!(array.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.remaining == 0
    }

    /// Get the next element in the array.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Choice, Pod, Type};
    ///
    /// let mut pod = Pod::array();
    /// let mut choice = pod.as_mut().encode_choice(Choice::RANGE, Type::INT)?;
    ///
    /// choice.entry()?.encode(10i32)?;
    /// choice.entry()?.encode(0i32)?;
    /// choice.entry()?.encode(30i32)?;
    ///
    /// choice.close()?;
    ///
    /// let mut choice = pod.decode_choice()?;
    ///
    /// assert_eq!(choice.ty(), Choice::RANGE);
    ///
    /// let mut count = 0;
    ///
    /// while !choice.is_empty() {
    ///     let pod = choice.entry()?;
    ///     assert_eq!(pod.ty(), Type::INT);
    ///     assert_eq!(pod.size(), 4);
    ///     count += 1;
    /// }
    ///
    /// assert_eq!(count, 3);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn entry(&mut self) -> Result<TypedPod<R::Clone<'_>>, Error> {
        if self.remaining == 0 {
            return Err(Error::new(ErrorKind::ArrayUnderflow));
        }

        let tail = self.reader.split(self.child_size)?;

        let pod = TypedPod::new(self.child_size, self.child_type, tail);
        self.remaining -= 1;
        Ok(pod)
    }
}
