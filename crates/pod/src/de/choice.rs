use core::fmt;
use core::mem;

#[cfg(feature = "alloc")]
use crate::DynamicBuf;
#[cfg(feature = "alloc")]
use crate::buf::AllocError;
use crate::error::ErrorKind;
use crate::utils::array_remaining;
use crate::{
    AsReader, ChoiceType, EncodeUnsized, Error, PackedPod, Reader, Type, TypedPod, Writer,
};

/// A decoder for a choice.
///
/// # Examples
///
/// ```
/// use pod::{ChoiceType, Pod, Type};
///
/// let mut pod = pod::array();
/// pod.as_mut().push_choice(ChoiceType::RANGE, Type::INT, |choice| {
///     choice.child().push(10i32)?;
///     choice.child().push(0i32)?;
///     choice.child().push(30i32)?;
///     Ok(())
/// })?;
///
/// let mut choice = pod.as_ref().next_choice()?;
/// assert_eq!(choice.choice_type(), ChoiceType::RANGE);
///
/// let mut count = 0;
///
/// while let Some(pod) = choice.next() {
///     assert_eq!(pod.ty(), Type::INT);
///     assert_eq!(pod.size(), 4);
///     count += 1;
/// }
///
/// assert_eq!(count, 3);
/// # Ok::<_, pod::Error>(())
/// ```
pub struct Choice<B> {
    buf: B,
    choice_type: ChoiceType,
    flags: u32,
    child_size: usize,
    child_type: Type,
    remaining: usize,
}

impl<B> Choice<B> {
    /// Return the type of the choice.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ChoiceType, Pod, Type};
    ///
    /// let mut pod = pod::array();
    /// pod.as_mut().push_choice(ChoiceType::RANGE, Type::INT, |choice| {
    ///     choice.child().push(10i32)?;
    ///     choice.child().push(0i32)?;
    ///     choice.child().push(30i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let mut choice = pod.as_ref().next_choice()?;
    /// assert_eq!(choice.choice_type(), ChoiceType::RANGE);
    ///
    /// let mut count = 0;
    ///
    /// while let Some(pod) = choice.next() {
    ///     assert_eq!(pod.ty(), Type::INT);
    ///     assert_eq!(pod.size(), 4);
    ///     count += 1;
    /// }
    ///
    /// assert_eq!(count, 3);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub const fn choice_type(&self) -> ChoiceType {
        self.choice_type
    }

    /// Return the type of the child element.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ChoiceType, Pod, Type};
    ///
    /// let mut pod = pod::array();
    /// pod.as_mut().push_choice(ChoiceType::RANGE, Type::INT, |choice| {
    ///     choice.child().push(10i32)?;
    ///     choice.child().push(0i32)?;
    ///     choice.child().push(30i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let choice = pod.as_ref().next_choice()?;
    /// assert_eq!(choice.child_type(), Type::INT);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub const fn child_type(&self) -> Type {
        self.child_type
    }

    /// Return the size of the child element.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ChoiceType, Pod, Type};
    ///
    /// let mut pod = pod::array();
    /// pod.as_mut().push_choice(ChoiceType::RANGE, Type::INT, |choice| {
    ///     choice.child().push(10i32)?;
    ///     choice.child().push(0i32)?;
    ///     choice.child().push(30i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let choice = pod.as_ref().next_choice()?;
    /// assert_eq!(choice.child_size(), 4);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub const fn child_size(&self) -> usize {
        self.child_size
    }

    /// Get a reference to the underlying buffer.
    #[inline]
    pub fn as_buf(&self) -> &B {
        &self.buf
    }
}

impl<'de, B> Choice<B>
where
    B: Reader<'de>,
{
    #[inline]
    pub fn new(
        buf: B,
        choice_type: ChoiceType,
        flags: u32,
        child_size: usize,
        child_type: Type,
        remaining: usize,
    ) -> Self {
        Self {
            buf,
            choice_type,
            flags,
            child_size,
            child_type,
            remaining,
        }
    }

    #[inline]
    pub(crate) fn from_reader(mut reader: B, size: usize) -> Result<Self, Error> {
        let [choice_type, flags, child_size, child_type] = reader.read::<[u32; 4]>()?;

        let Ok(child_size) = usize::try_from(child_size) else {
            return Err(Error::new(ErrorKind::SizeOverflow));
        };

        let choice_type = ChoiceType::from_u32(choice_type);
        let child_type = Type::new(child_type);
        let remaining = array_remaining(size, child_size, mem::size_of::<[u32; 4]>())?;

        Ok(Self {
            buf: reader,
            choice_type,
            flags,
            child_size,
            child_type,
            remaining,
        })
    }

    /// Get the number of elements left to decode from the array.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = pod::array();
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
    pub const fn len(&self) -> usize {
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
    pub const fn is_empty(&self) -> bool {
        self.remaining == 0
    }

    /// Get the next element in the array.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ChoiceType, Pod, Type};
    ///
    /// let mut pod = pod::array();
    /// pod.as_mut().push_choice(ChoiceType::RANGE, Type::INT, |choice| {
    ///     choice.child().push(10i32)?;
    ///     choice.child().push(0i32)?;
    ///     choice.child().push(30i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let mut choice = pod.as_ref().next_choice()?;
    /// assert_eq!(choice.choice_type(), ChoiceType::RANGE);
    ///
    /// let mut count = 0;
    ///
    /// while let Some(pod) = choice.next() {
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

    /// Coerce into an owned [`Choice`].
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ChoiceType, Pod, Type};
    ///
    /// let mut pod = pod::array();
    /// pod.as_mut().push_choice(ChoiceType::RANGE, Type::INT, |choice| {
    ///     choice.child().push(10i32)?;
    ///     choice.child().push(0i32)?;
    ///     choice.child().push(30i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let choice = pod.as_ref().next_choice()?.to_owned()?;
    /// assert_eq!(choice.choice_type(), ChoiceType::RANGE);
    ///
    /// let mut choice = choice.as_ref();
    ///
    /// let mut count = 0;
    ///
    /// while let Some(pod) = choice.next() {
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
    pub fn to_owned(&self) -> Result<Choice<DynamicBuf>, AllocError> {
        Ok(Choice {
            buf: DynamicBuf::from_slice(self.buf.as_bytes())?,
            choice_type: self.choice_type,
            flags: self.flags,
            child_size: self.child_size,
            child_type: self.child_type,
            remaining: self.remaining,
        })
    }
}

impl<B> Choice<B>
where
    B: AsReader,
{
    /// Coerce into a borrowed [`Choice`].
    ///
    /// Decoding this object does not affect the original object.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ChoiceType, Pod, Type};
    ///
    /// let mut pod = pod::array();
    /// pod.as_mut().push_choice(ChoiceType::RANGE, Type::INT, |choice| {
    ///     choice.child().push(10i32)?;
    ///     choice.child().push(0i32)?;
    ///     choice.child().push(30i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let choice = pod.as_ref().next_choice()?.to_owned()?;
    /// assert_eq!(choice.choice_type(), ChoiceType::RANGE);
    ///
    /// let mut choice = choice.as_ref();
    ///
    /// let mut count = 0;
    ///
    /// while let Some(pod) = choice.next() {
    ///     assert_eq!(pod.ty(), Type::INT);
    ///     assert_eq!(pod.size(), 4);
    ///     count += 1;
    /// }
    ///
    /// assert_eq!(count, 3);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn as_ref(&self) -> Choice<B::AsReader<'_>> {
        Choice::new(
            self.buf.as_reader(),
            self.choice_type,
            self.flags,
            self.child_size,
            self.child_type,
            self.remaining,
        )
    }
}

/// [`Encode`] implementation for [`Choice`].
///
/// # Examples
///
/// ```
/// use pod::{ChoiceType, Pod, Type};
///
/// let mut pod = pod::array();
/// pod.as_mut().push_choice(ChoiceType::RANGE, Type::INT, |choice| {
///     choice.child().push(10i32)?;
///     choice.child().push(0i32)?;
///     choice.child().push(30i32)?;
///     Ok(())
/// })?;
///
/// let mut choice = pod.as_ref().next_choice()?;
///
/// let mut pod2 = pod::array();
/// pod2.as_mut().encode(choice)?;
///
/// let mut choice = pod2.as_ref().next_choice()?;
///
/// assert_eq!(choice.choice_type(), ChoiceType::RANGE);
/// assert_eq!(choice.len(), 3);
///
/// let c = choice.next().unwrap();
/// assert_eq!(choice.len(), 2);
/// assert_eq!(c.ty(), Type::INT);
/// assert_eq!(c.size(), 4);
/// assert_eq!(c.next::<i32>()?, 10);
///
/// let c = choice.next().unwrap();
/// assert_eq!(choice.len(), 1);
/// assert_eq!(c.ty(), Type::INT);
/// assert_eq!(c.size(), 4);
/// assert_eq!(c.next::<i32>()?, 0);
///
/// let c = choice.next().unwrap();
/// assert_eq!(choice.len(), 0);
/// assert_eq!(c.ty(), Type::INT);
/// assert_eq!(c.size(), 4);
/// assert_eq!(c.next::<i32>()?, 30);
/// # Ok::<_, pod::Error>(())
/// ```
impl<B> EncodeUnsized for Choice<B>
where
    B: AsReader,
{
    const TYPE: Type = Type::CHOICE;

    #[inline]
    fn size(&self) -> usize {
        self.remaining
            .wrapping_mul(self.child_size)
            .wrapping_add(mem::size_of::<[u32; 4]>())
    }

    #[inline]
    fn write_content(&self, mut writer: impl Writer) -> Result<(), Error> {
        let Ok(child_size) = u32::try_from(self.child_size) else {
            return Err(Error::new(ErrorKind::SizeOverflow));
        };

        writer.write(&[
            self.choice_type.into_u32(),
            self.flags,
            child_size,
            self.child_type.into_u32(),
        ])?;

        writer.write(self.buf.as_reader().as_bytes())
    }
}

crate::macros::encode_into_unsized!(impl [B] Choice<B> where B: AsReader);

impl<B> fmt::Debug for Choice<B>
where
    B: AsReader,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct Entries<'a, B>(&'a Choice<B>);

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

        let mut f = f.debug_struct("Choice");
        f.field("type", &self.choice_type());
        f.field("child_type", &self.child_type());
        f.field("entries", &Entries(self));
        f.finish()
    }
}
