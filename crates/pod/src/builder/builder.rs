use core::fmt;
use core::mem;

#[cfg(feature = "alloc")]
use alloc::boxed::Box;

#[cfg(feature = "alloc")]
use crate::DynamicBuf;
use crate::SplitReader;
use crate::builder::{ArrayBuilder, ChoiceBuilder, ObjectBuilder, SequenceBuilder, StructBuilder};
use crate::error::ErrorKind;
use crate::{ArrayBuf, Encode, EncodeInto};
use crate::{
    AsReader, ChoiceType, EncodeUnsized, Error, Pod, RawId, Reader, Type, TypedPod, Writer,
};

/// An unlimited pod.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct EnvelopePod;

/// A pod limited for a specific child type and size.
#[derive(Clone, Copy, Debug)]
pub struct ChildPod {
    size: usize,
    ty: Type,
}

pub(crate) mod sealed {
    use super::{ChildPod, EnvelopePod};

    pub trait Sealed {}
    impl Sealed for EnvelopePod {}
    impl Sealed for ChildPod {}
}

pub trait PodKind
where
    Self: self::sealed::Sealed,
{
    const ENVELOPE: bool;

    #[inline]
    fn header(&self, _: impl Writer) -> Result<(), Error> {
        Ok(())
    }

    fn push<T>(&self, value: T, buf: impl Writer) -> Result<(), Error>
    where
        T: Encode;

    fn push_unsized<T>(&self, value: &T, buf: impl Writer) -> Result<(), Error>
    where
        T: ?Sized + EncodeUnsized;

    #[inline]
    fn check(&self, _: Type, _: usize) -> Result<(), Error> {
        Ok(())
    }

    #[inline]
    fn check_size<W>(&self, ty: Type, writer: &W, header: W::Pos) -> Result<u32, Error>
    where
        W: ?Sized + Writer,
    {
        // This should always hold, since when we reserve space, we always
        // reserve space for the header, which is 64 bits wide.
        debug_assert!(writer.distance_from(header) >= mem::size_of::<[u32; 2]>());

        // Calculate the size of the struct at the header position.
        //
        // Every header is exactly 64-bits wide and this is not included in the
        // size of the objects, so we have to subtract it here.
        let size = writer
            .distance_from(header)
            .wrapping_sub(mem::size_of::<[u32; 2]>());

        self.check(ty, size)?;

        let Ok(size) = u32::try_from(size) else {
            return Err(Error::new(ErrorKind::SizeOverflow));
        };

        Ok(size)
    }
}

impl PodKind for ChildPod {
    const ENVELOPE: bool = false;

    #[inline]
    fn push<T>(&self, value: T, buf: impl Writer) -> Result<(), Error>
    where
        T: Encode,
    {
        self.check(T::TYPE, T::SIZE)?;
        value.write_content(buf)
    }

    #[inline]
    fn push_unsized<T>(&self, value: &T, buf: impl Writer) -> Result<(), Error>
    where
        T: ?Sized + EncodeUnsized,
    {
        self.check(T::TYPE, value.size())?;
        value.write_content(buf)
    }

    #[inline]
    fn check(&self, ty: Type, size: usize) -> Result<(), Error> {
        if self.ty != ty {
            return Err(Error::new(ErrorKind::Expected {
                expected: self.ty,
                actual: ty,
            }));
        }

        if size > self.size {
            return Err(Error::new(ErrorKind::ChildSizeMismatch {
                expected: self.size,
                actual: size,
            }));
        }

        Ok(())
    }
}

impl PodKind for EnvelopePod {
    const ENVELOPE: bool = true;

    #[inline]
    fn push<T>(&self, value: T, mut buf: impl Writer) -> Result<(), Error>
    where
        T: Encode,
    {
        let Ok(size) = u32::try_from(T::SIZE) else {
            return Err(Error::new(ErrorKind::SizeOverflow));
        };

        buf.write([size, T::TYPE.into_u32()])?;
        value.write_content(buf)
    }

    #[inline]
    fn push_unsized<T>(&self, value: &T, mut buf: impl Writer) -> Result<(), Error>
    where
        T: ?Sized + EncodeUnsized,
    {
        let Ok(size) = u32::try_from(value.size()) else {
            return Err(Error::new(ErrorKind::SizeOverflow));
        };

        buf.write([size, T::TYPE.into_u32()])?;
        value.write_content(buf)
    }

    #[inline]
    fn check(&self, _: Type, _: usize) -> Result<(), Error> {
        Ok(())
    }
}

/// A POD (Plain Old Data) handler.
///
/// This is a wrapper that can be used for encoding and decoding data.
pub struct Builder<B, K = EnvelopePod> {
    buf: B,
    pub(crate) kind: K,
}

impl<B, K> Clone for Builder<B, K>
where
    B: Clone,
    K: Clone,
{
    #[inline]
    fn clone(&self) -> Self {
        Builder {
            buf: self.buf.clone(),
            kind: self.kind.clone(),
        }
    }
}

#[cfg(feature = "alloc")]
impl Builder<DynamicBuf> {
    /// Construct a new [`Builder`] with a dynamically sized buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut pod = pod::dynamic();
    /// pod.as_mut().push(10i32)?;
    /// assert_eq!(pod.as_ref().next::<i32>()?, 10i32);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub const fn dynamic() -> Self {
        Self::new(DynamicBuf::new())
    }

    /// Clear the current builder.
    ///
    /// This will clear the buffer and reset the pod to an empty state.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut pod = pod::dynamic();
    /// pod.as_mut().push(10i32)?;
    /// assert_eq!(pod.as_ref().next::<i32>()?, 10i32);
    /// pod.clear();
    /// pod.as_mut().push(20i32)?;
    /// assert_eq!(pod.as_ref().next::<i32>()?, 20i32);
    /// # Ok::<_, pod::Error>(())
    /// ```
    pub fn clear(&mut self) {
        self.buf.clear();
    }
}

impl Builder<ArrayBuf> {
    /// Construct a new [`Builder`] with a 128 word-sized array buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::Builder;
    ///
    /// let mut pod = Builder::array();
    /// pod.as_mut().push(10i32)?;
    /// assert_eq!(pod.as_ref().next::<i32>()?, 10i32);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub const fn array() -> Self {
        Self::new(ArrayBuf::new())
    }

    /// Clear the current builder.
    ///
    /// This will clear the buffer and reset the pod to an empty state.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut pod = pod::array();
    /// pod.as_mut().push(10i32)?;
    /// assert_eq!(pod.as_ref().next::<i32>()?, 10i32);
    /// pod.clear();
    /// pod.as_mut().push(20i32)?;
    /// assert_eq!(pod.as_ref().next::<i32>()?, 20i32);
    /// # Ok::<_, pod::Error>(())
    /// ```
    pub fn clear(&mut self) {
        self.buf.clear();
    }
}

impl<B> Builder<B> {
    /// Construct a new [`Builder`] arround the specified buffer `B`.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Builder};
    ///
    /// let mut buf = ArrayBuf::default();
    /// _ = Builder::new(&mut buf);
    ///
    /// _ = Builder::new(ArrayBuf::<16>::new());
    /// ```
    #[inline]
    pub const fn new(buf: B) -> Self {
        Builder {
            buf,
            kind: EnvelopePod,
        }
    }

    /// Coerce into a [`Builder`] with the current buffer.
    pub fn into_pod(self) -> Pod<B> {
        Pod::new(self.buf)
    }
}

impl<B> Builder<B>
where
    B: Writer,
{
    /// Borrow the current pod mutably, allowing multiple elements to be encoded
    /// into it or the pod immediately re-used.
    #[inline]
    pub fn as_mut(&mut self) -> Builder<B::Mut<'_>> {
        Builder::new(self.buf.borrow_mut())
    }
}

impl<B> Builder<B>
where
    B: SplitReader,
{
    /// Split a builder off.
    ///
    /// This will clear the builder which is currently associated with `self`
    /// and return the data written so far in the pod.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut pod = pod::array();
    /// pod.as_mut().push(10i32)?;
    /// assert_eq!(pod.take().decode::<i32>()?, 10);
    ///
    /// pod.as_mut().push(42i32)?;
    /// assert_eq!(pod.take().decode::<i32>()?, 42);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn take(&mut self) -> Pod<B::TakeReader<'_>> {
        Pod::new(self.buf.take_reader())
    }
}

impl<B> Builder<B>
where
    B: AsReader,
{
    /// Coerce any pod into an owned pod.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut pod = pod::array();
    /// pod.as_mut().push(10i32)?;
    ///
    /// let pod = pod.to_owned();
    ///
    /// assert_eq!(pod.as_ref().next::<i32>()?, 10i32);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[cfg(feature = "alloc")]
    pub fn to_owned(&self) -> Pod<Box<[u64]>> {
        Pod::new(Box::from(self.buf.as_reader().as_slice()))
    }

    /// Coerce an owned pod into a borrowed pod which can be used for reading.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut pod = pod::array();
    /// pod.as_mut().push(10i32)?;
    ///
    /// let pod = pod.to_owned();
    ///
    /// assert_eq!(pod.as_ref().next::<i32>()?, 10i32);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn as_ref(&self) -> Pod<B::AsReader<'_>> {
        Pod::new(self.buf.as_reader())
    }
}

impl<B> Builder<B, ChildPod> {
    /// Construct a new child pod.
    pub(crate) const fn new_child(buf: B, size: usize, ty: Type) -> Self {
        Builder {
            buf,
            kind: ChildPod { size, ty },
        }
    }
}

impl<B, K> Builder<B, K> {
    #[inline]
    pub(crate) fn new_with(buf: B, kind: K) -> Self
    where
        B: Writer,
    {
        Builder { buf, kind }
    }

    /// Access the underlying buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut pod = pod::array();
    /// pod.as_mut().push(10i32)?;
    ///
    /// let buf = pod.as_buf();
    /// assert_eq!(buf.as_slice().len(), 2);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn as_buf(&self) -> &B {
        &self.buf
    }

    /// Coerce into the underlying buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut pod = pod::array();
    /// pod.as_mut().push(10i32)?;
    ///
    /// let buf = pod.into_buf();
    /// assert_eq!(buf.as_slice().len(), 2);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn into_buf(self) -> B {
        self.buf
    }
}

impl<B, K> Builder<B, K>
where
    B: Writer,
    K: PodKind,
{
    /// Conveniently encode a value into the pod.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut pod = pod::array();
    /// pod.as_mut().push_struct(|st| st.encode((10i32, "hello world", [1u32, 2u32])))?;
    ///
    /// let mut pod = pod.as_ref();
    /// let mut st = pod.next_struct()?;
    ///
    /// assert_eq!(st.field()?.next::<i32>()?, 10i32);
    /// assert_eq!(st.field()?.next_borrowed::<str>()?, "hello world");
    /// assert_eq!(st.field()?.next::<u32>()?, 1);
    /// assert_eq!(st.field()?.next::<u32>()?, 2);
    /// assert!(st.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    pub fn encode<T>(self, value: T) -> Result<(), Error>
    where
        T: EncodeInto,
    {
        value.encode_into(self)
    }

    /// Encode a value from the pod.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut pod = pod::array();
    /// pod.push(10i32)?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    pub fn push<T>(mut self, value: T) -> Result<(), Error>
    where
        T: Encode,
    {
        self.kind.header(self.buf.borrow_mut())?;
        self.kind.push(value, self.buf)
    }

    /// Encode an unsized value from the pod.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut pod = pod::array();
    /// pod.push_unsized(&b"hello world"[..])?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    pub fn push_unsized<T>(mut self, value: &T) -> Result<(), Error>
    where
        T: ?Sized + EncodeUnsized,
    {
        self.kind.header(self.buf.borrow_mut())?;
        self.kind.push_unsized(value, self.buf)
    }

    /// Encode a `None` value.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut pod = pod::array();
    /// pod.push_none()?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn push_none(mut self) -> Result<(), Error> {
        self.kind.check(Type::NONE, 0)?;

        if K::ENVELOPE {
            self.buf.write([0, Type::NONE.into_u32()])?;
        }

        Ok(())
    }

    /// Encode an array with the given sized type.
    ///
    /// To encode an array with unsized types, use [`Pod::push_unsized_array`]
    /// where a length in bytes must be specified for every element.
    ///
    /// # Errors
    ///
    /// This will error if:
    ///
    /// * The specified type is unsized, an error will be returned.
    /// * An element is being inserted which does not match the specified
    ///   `child_type`.
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = pod::array();
    /// assert!(pod.as_mut().push_array(Type::STRING, |_| Ok(())).is_err());
    ///
    /// let mut pod = pod::array();
    /// let mut array = pod.as_mut().push_array(Type::INT, |array| {
    ///     assert!(array.child().push(42.42f32).is_err());
    ///     Ok(())
    /// })?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = pod::array();
    /// let mut array = pod.as_mut().push_array(Type::INT, |array| {
    ///     array.child().push(1i32)?;
    ///     array.child().push(2i32)?;
    ///     array.child().push(3i32)?;
    ///     Ok(())
    /// })?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn push_array(
        mut self,
        child_type: Type,
        f: impl FnOnce(&mut ArrayBuilder<B, K>) -> Result<(), Error>,
    ) -> Result<(), Error> {
        self.kind.header(self.buf.borrow_mut())?;
        let mut encoder = ArrayBuilder::to_writer(self.buf, self.kind, child_type)?;
        f(&mut encoder)?;
        encoder.close()?;
        Ok(())
    }

    /// Encode an array with items of an unsized type.
    ///
    /// The `len` specified must match every element of the array.
    ///
    /// # Errors
    ///
    ///
    /// # Errors
    ///
    /// This will error if:
    ///
    /// * The specified type is size and the specified length does not match the
    ///   size of the type.
    /// * An element is being inserted which does not match the specified
    ///   `child_type`.
    /// * An unsized element is being inserted which does not match the size in
    ///   bytes of `len`.
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    ///
    /// let mut pod = pod::array();
    /// assert!(pod.push_unsized_array(Type::INT, 5, |_| Ok(())).is_err());
    ///
    /// let mut pod = pod::array();
    ///
    /// pod.push_unsized_array(Type::STRING, 4, |array| {
    ///     // Note: strings are null-terminated, so the length is 4.
    ///     array.child().push_unsized("foo")?;
    ///
    ///     assert!(array.child().push(1i32).is_err());
    ///     assert!(array.child().push_unsized("barbaz").is_err());
    ///     Ok(())
    /// })?;
    ///
    /// # Ok::<_, pod::Error>(())
    /// ```
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = pod::array();
    ///
    /// pod.as_mut().push_unsized_array(Type::STRING, 4, |array| {
    ///     // Note: strings are null-terminated, so the length is 4.
    ///     array.child().push_unsized("foo")?;
    ///     array.child().push_unsized("bar")?;
    ///     array.child().push_unsized("baz")?;
    ///     Ok(())
    /// })?;
    ///
    /// let buf = pod.into_buf();
    /// assert_eq!(buf.as_slice().len(), 5);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn push_unsized_array(
        mut self,
        child_type: Type,
        child_size: usize,
        f: impl FnOnce(&mut ArrayBuilder<B, K>) -> Result<(), Error>,
    ) -> Result<(), Error> {
        self.kind.header(self.buf.borrow_mut())?;
        let mut array =
            ArrayBuilder::to_writer_unsized(self.buf, self.kind, child_size, child_type)?;
        f(&mut array)?;
        array.close()?;
        Ok(())
    }

    /// Encode a struct.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = pod::array();
    /// pod.as_mut().push_struct(|st| {
    ///     st.field().push(1i32)?;
    ///     st.field().push(2i32)?;
    ///     st.field().push(3i32)?;
    ///     Ok(())
    /// })?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn push_struct(
        mut self,
        f: impl FnOnce(&mut StructBuilder<B, K>) -> Result<(), Error>,
    ) -> Result<(), Error> {
        self.kind.header(self.buf.borrow_mut())?;
        let mut encoder = StructBuilder::to_writer(self.buf, self.kind)?;
        f(&mut encoder)?;
        encoder.close()?;
        Ok(())
    }

    /// Encode an object.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = pod::array();
    /// pod.as_mut().push_object(10, 20, |obj| {
    ///     obj.property(1).push(1i32)?;
    ///     obj.property(2).push(2i32)?;
    ///     obj.property(3).push(3i32)?;
    ///     Ok(())
    /// })?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn push_object(
        mut self,
        object_type: impl RawId,
        object_id: impl RawId,
        f: impl FnOnce(&mut ObjectBuilder<B, K>) -> Result<(), Error>,
    ) -> Result<(), Error> {
        self.kind.header(self.buf.borrow_mut())?;
        let mut encoder = ObjectBuilder::to_writer(
            self.buf,
            self.kind,
            object_type.into_id(),
            object_id.into_id(),
        )?;
        f(&mut encoder)?;
        encoder.close()?;
        Ok(())
    }

    /// Encode a sequence.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = pod::array();
    /// pod.as_mut().push_sequence(|seq| {
    ///     seq.control().push(1i32)?;
    ///     seq.control().push(2i32)?;
    ///     seq.control().push(3i32)?;
    ///     Ok(())
    /// })?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn push_sequence(
        mut self,
        f: impl FnOnce(&mut SequenceBuilder<B, K>) -> Result<(), Error>,
    ) -> Result<(), Error> {
        self.kind.header(self.buf.borrow_mut())?;
        let mut encoder = SequenceBuilder::to_writer(self.buf, self.kind)?;
        f(&mut encoder)?;
        encoder.close()?;
        Ok(())
    }

    /// Encode a choice.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ChoiceType, Pod, Type};
    ///
    /// let mut pod = pod::array();
    /// pod.as_mut().push_choice(ChoiceType::RANGE, Type::INT, |choice| {
    ///     choice.child().push(1i32)?;
    ///     Ok(())
    /// })?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn push_choice(
        mut self,
        choice: ChoiceType,
        child_type: Type,
        f: impl FnOnce(&mut ChoiceBuilder<B, K>) -> Result<(), Error>,
    ) -> Result<(), Error> {
        self.kind.header(self.buf.borrow_mut())?;
        let mut encoder = ChoiceBuilder::to_writer(self.buf, self.kind, choice, child_type)?;
        f(&mut encoder)?;
        encoder.close()?;
        Ok(())
    }

    /// Encode a nested pod.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, TypedPod};
    ///
    /// let mut pod = pod::array();
    /// pod.as_mut().push_pod(|pod| {
    ///     pod.as_mut().push_struct(|st| {
    ///         st.field().push(1i32)?;
    ///         st.field().push(2i32)?;
    ///         st.field().push(3i32)?;
    ///         Ok(())
    ///     })
    /// })?;
    ///
    /// let pod = pod.as_ref().into_typed()?.next_pod()?;
    /// let mut st = pod.as_ref().next_struct()?;
    /// assert!(!st.is_empty());
    /// assert_eq!(st.field()?.next::<i32>()?, 1i32);
    /// assert_eq!(st.field()?.next::<i32>()?, 2i32);
    /// assert_eq!(st.field()?.next::<i32>()?, 3i32);
    /// assert!(st.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn push_pod(
        mut self,
        f: impl FnOnce(&mut Builder<B>) -> Result<(), Error>,
    ) -> Result<(), Error> {
        self.kind.header(self.buf.borrow_mut())?;

        // Reserve space for the header of the choice which includes its size
        // that will be determined later.
        let header = self.buf.reserve([0, Type::POD.into_u32()])?;

        let mut pod = Builder::new(self.buf);

        f(&mut pod)?;

        let size = pod
            .buf
            .distance_from(header)
            .wrapping_sub(mem::size_of::<[u32; 2]>());

        self.kind.check(Type::POD, size)?;

        let Ok(size) = u32::try_from(size) else {
            return Err(Error::new(ErrorKind::SizeOverflow));
        };

        pod.buf.write_at(header, [size, Type::POD.into_u32()])?;
        Ok(())
    }

    /// Coerce into envelope [`Builder`].
    pub fn into_envelope(mut self) -> Result<Builder<B>, Error> {
        self.kind.header(self.buf.borrow_mut())?;
        Ok(Builder::new(self.buf))
    }
}

/// [`Encode`] implementation for [`Builder`].
///
/// # Examples
///
/// ```
/// use pod::{Pod, Type};
///
/// let mut pod = pod::array();
/// pod.as_mut().push_object(10, 20, |obj| {
///     obj.property(1).flags(0b001).push(1i32)?;
///     obj.property(2).flags(0b010).push(2i32)?;
///     obj.property(3).flags(0b100).push(3i32)?;
///     Ok(())
/// })?;
///
/// let mut pod2 = pod::array();
/// pod2.as_mut().encode(pod)?;
///
/// let mut obj = pod2.as_ref().next_pod()?.next_object()?;
/// assert!(!obj.is_empty());
///
/// let p = obj.property()?;
/// assert_eq!(p.key(), 1);
/// assert_eq!(p.flags(), 0b001);
/// assert_eq!(p.value().next::<i32>()?, 1);
///
/// let p = obj.property()?;
/// assert_eq!(p.key(), 2);
/// assert_eq!(p.flags(), 0b010);
/// assert_eq!(p.value().next::<i32>()?, 2);
///
/// let p = obj.property()?;
/// assert_eq!(p.key(), 3);
/// assert_eq!(p.flags(), 0b100);
/// assert_eq!(p.value().next::<i32>()?, 3);
///
/// assert!(obj.is_empty());
/// # Ok::<_, pod::Error>(())
/// ```
impl<B> EncodeUnsized for Builder<B>
where
    B: AsReader,
{
    const TYPE: Type = Type::POD;

    #[inline]
    fn size(&self) -> usize {
        self.buf.as_reader().bytes_len()
    }

    #[inline]
    fn write_content(&self, mut writer: impl Writer) -> Result<(), Error> {
        writer.write_words(self.buf.as_reader().as_slice())
    }
}

crate::macros::encode_into_unsized!(impl [B] Builder<B> where B: AsReader);

impl<B, K> fmt::Debug for Builder<B, K>
where
    B: AsReader,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match TypedPod::from_reader(self.buf.as_reader()) {
            Ok(pod) => pod.fmt(f),
            Err(e) => e.fmt(f),
        }
    }
}
