use core::fmt;
use core::mem;

#[cfg(feature = "alloc")]
use alloc::boxed::Box;

use crate::error::ErrorKind;
use crate::utils::array_remaining;
use crate::{AsReader, ChoiceType, Encode, Error, Reader, Type, TypedPod, Writer};

/// A decoder for a choice.
///
/// # Examples
///
/// ```
/// use pod::{ChoiceType, Pod, Type};
///
/// let mut pod = Pod::array();
/// pod.as_mut().push_choice(ChoiceType::RANGE, Type::INT, |choice| {
///     choice.entry()?.push(10i32)?;
///     choice.entry()?.push(0i32)?;
///     choice.entry()?.push(30i32)?;
///     Ok(())
/// })?;
///
/// let mut choice = pod.decode_choice()?;
/// assert_eq!(choice.choice_type(), ChoiceType::RANGE);
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
    /// let mut pod = Pod::array();
    /// pod.as_mut().push_choice(ChoiceType::RANGE, Type::INT, |choice| {
    ///     choice.entry()?.push(10i32)?;
    ///     choice.entry()?.push(0i32)?;
    ///     choice.entry()?.push(30i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let mut choice = pod.decode_choice()?;
    /// assert_eq!(choice.choice_type(), ChoiceType::RANGE);
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
    /// let mut pod = Pod::array();
    /// pod.as_mut().push_choice(ChoiceType::RANGE, Type::INT, |choice| {
    ///     choice.entry()?.push(10i32)?;
    ///     choice.entry()?.push(0i32)?;
    ///     choice.entry()?.push(30i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let choice = pod.decode_choice()?;
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
    /// let mut pod = Pod::array();
    /// pod.as_mut().push_choice(ChoiceType::RANGE, Type::INT, |choice| {
    ///     choice.entry()?.push(10i32)?;
    ///     choice.entry()?.push(0i32)?;
    ///     choice.entry()?.push(30i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let choice = pod.decode_choice()?;
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
    B: Reader<'de, u64>,
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
    /// let mut pod = Pod::array();
    /// pod.as_mut().push_array(Type::INT, |array| {
    ///     array.child()?.push(1i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let mut array = pod.decode_array()?;
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
    /// let mut pod = Pod::array();
    /// pod.as_mut().push_array(Type::INT, |_| Ok(()))?;
    ///
    /// let mut array = pod.decode_array()?;
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
    /// let mut pod = Pod::array();
    /// pod.as_mut().push_choice(ChoiceType::RANGE, Type::INT, |choice| {
    ///     choice.entry()?.push(10i32)?;
    ///     choice.entry()?.push(0i32)?;
    ///     choice.entry()?.push(30i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let mut choice = pod.decode_choice()?;
    /// assert_eq!(choice.choice_type(), ChoiceType::RANGE);
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
    pub fn entry(&mut self) -> Result<TypedPod<B::Reader<'_>>, Error> {
        if self.remaining == 0 {
            return Err(Error::new(ErrorKind::ArrayUnderflow));
        }

        let tail = self.buf.split(self.child_size)?;

        let pod = TypedPod::new(self.child_size, self.child_type, tail);
        self.remaining -= 1;
        Ok(pod)
    }

    /// Coerce into an owned [`Choice`].
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ChoiceType, Pod, Type};
    ///
    /// let mut pod = Pod::array();
    /// pod.as_mut().push_choice(ChoiceType::RANGE, Type::INT, |choice| {
    ///     choice.entry()?.push(10i32)?;
    ///     choice.entry()?.push(0i32)?;
    ///     choice.entry()?.push(30i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let choice = pod.decode_choice()?.to_owned();
    /// assert_eq!(choice.choice_type(), ChoiceType::RANGE);
    ///
    /// let mut choice = choice.as_ref();
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
    #[cfg(feature = "alloc")]
    #[inline]
    pub fn to_owned(&self) -> Choice<Box<[u64]>> {
        Choice {
            buf: Box::from(self.buf.as_slice()),
            choice_type: self.choice_type,
            flags: self.flags,
            child_size: self.child_size,
            child_type: self.child_type,
            remaining: self.remaining,
        }
    }
}

impl<B> Choice<B>
where
    B: AsReader<u64>,
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
    /// let mut pod = Pod::array();
    /// pod.as_mut().push_choice(ChoiceType::RANGE, Type::INT, |choice| {
    ///     choice.entry()?.push(10i32)?;
    ///     choice.entry()?.push(0i32)?;
    ///     choice.entry()?.push(30i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let choice = pod.decode_choice()?.to_owned();
    /// assert_eq!(choice.choice_type(), ChoiceType::RANGE);
    ///
    /// let mut choice = choice.as_ref();
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
    pub fn as_ref(&self) -> Choice<B::Reader<'_>> {
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
/// let mut pod = Pod::array();
/// pod.as_mut().push_choice(ChoiceType::RANGE, Type::INT, |choice| {
///     choice.entry()?.push(10i32)?;
///     choice.entry()?.push(0i32)?;
///     choice.entry()?.push(30i32)?;
///     Ok(())
/// })?;
///
/// let mut choice = pod.decode_choice()?;
///
/// let mut pod2 = Pod::array();
/// pod2.as_mut().push(choice)?;
///
/// let mut choice = pod2.decode_choice()?;
/// assert_eq!(choice.choice_type(), ChoiceType::RANGE);
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
impl<B> Encode for Choice<B>
where
    B: AsReader<u64>,
{
    const TYPE: Type = Type::CHOICE;

    #[inline]
    fn size(&self) -> usize {
        let len = self.buf.as_reader().bytes_len();
        len.wrapping_add(mem::size_of::<[u32; 4]>())
    }

    #[inline]
    fn write_content(&self, mut writer: impl Writer<u64>) -> Result<(), Error> {
        let Ok(child_size) = u32::try_from(self.child_size) else {
            return Err(Error::new(ErrorKind::SizeOverflow));
        };

        writer.write([
            self.choice_type.into_u32(),
            self.flags,
            child_size,
            self.child_type.into_u32(),
        ])?;

        writer.write_words(self.buf.as_reader().as_slice())
    }
}

impl<B> fmt::Debug for Choice<B>
where
    B: AsReader<u64>,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct Entries<'a, B>(&'a Choice<B>);

        impl<B> fmt::Debug for Entries<'_, B>
        where
            B: AsReader<u64>,
        {
            #[inline]
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let mut this = self.0.as_ref();

                let mut f = f.debug_list();

                while !this.is_empty() {
                    match this.entry() {
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

        let mut f = f.debug_struct("Choice");
        f.field("type", &self.choice_type());
        f.field("child_type", &self.child_type());
        f.field("entries", &Entries(self));
        f.finish()
    }
}
