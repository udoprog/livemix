use core::fmt;
use core::mem;

#[cfg(feature = "alloc")]
use alloc::boxed::Box;

#[cfg(feature = "alloc")]
use crate::DynamicBuf;
use crate::builder::{ArrayBuilder, ChoiceBuilder, ObjectBuilder, SequenceBuilder, StructBuilder};
use crate::de::{Array, Choice, Object, Sequence, Struct};
use crate::error::ErrorKind;
use crate::{ArrayBuf, DecodeFrom, Encode, EncodeInto};
use crate::{
    AsReader, ChoiceType, Decode, DecodeUnsized, EncodeUnsized, Error, RawId, Reader, Type,
    TypedPod, Visitor, Writer,
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

mod sealed {
    use super::{ChildPod, EnvelopePod};

    pub trait Sealed {}
    impl Sealed for EnvelopePod {}
    impl Sealed for ChildPod {}
}

pub trait PodKind: Copy + self::sealed::Sealed {
    const ENVELOPE: bool;

    fn push<T>(&self, value: T, buf: impl Writer<u64>) -> Result<(), Error>
    where
        T: Encode;

    fn push_unsized<T>(&self, value: &T, buf: impl Writer<u64>) -> Result<(), Error>
    where
        T: ?Sized + EncodeUnsized;

    fn check(&self, ty: Type, size: usize) -> Result<(), Error>;

    #[inline]
    fn check_size<W>(self, ty: Type, writer: &W, header: W::Pos) -> Result<u32, Error>
    where
        W: ?Sized + Writer<u64>,
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
    fn push<T>(&self, value: T, buf: impl Writer<u64>) -> Result<(), Error>
    where
        T: Encode,
    {
        self.check(T::TYPE, T::SIZE)?;
        value.write_content(buf)
    }

    #[inline]
    fn push_unsized<T>(&self, value: &T, buf: impl Writer<u64>) -> Result<(), Error>
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
    fn push<T>(&self, value: T, mut buf: impl Writer<u64>) -> Result<(), Error>
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
    fn push_unsized<T>(&self, value: &T, mut buf: impl Writer<u64>) -> Result<(), Error>
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
pub struct Pod<B, K = EnvelopePod> {
    buf: B,
    kind: K,
}

impl<B, K> Clone for Pod<B, K>
where
    B: Clone,
    K: PodKind,
{
    #[inline]
    fn clone(&self) -> Self {
        Pod {
            buf: self.buf.clone(),
            kind: self.kind,
        }
    }
}

#[cfg(feature = "alloc")]
impl Pod<DynamicBuf> {
    /// Construct a new [`Pod`] with a 128 word-sized array buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::Pod;
    ///
    /// let mut pod = Pod::dynamic();
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
    /// use pod::Pod;
    ///
    /// let mut pod = Pod::dynamic();
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

impl Pod<ArrayBuf> {
    /// Construct a new [`Pod`] with a 128 word-sized array buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::Pod;
    ///
    /// let mut pod = Pod::array();
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
    /// use pod::Pod;
    ///
    /// let mut pod = Pod::array();
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

impl<B> Pod<B> {
    /// Construct a new [`Pod`] arround the specified buffer `B`.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Pod};
    ///
    /// let mut buf = ArrayBuf::<u64>::new();
    /// _ = Pod::new(&mut buf);
    ///
    /// _ = Pod::new(ArrayBuf::<u64, 16>::new());
    /// ```
    #[inline]
    pub const fn new(buf: B) -> Self {
        Pod {
            buf,
            kind: EnvelopePod,
        }
    }
}

impl<B> Pod<B, ChildPod> {
    /// Construct a new child pod.
    pub(crate) const fn new_child(buf: B, size: usize, ty: Type) -> Self {
        Pod {
            buf,
            kind: ChildPod { size, ty },
        }
    }
}

impl<B, K> Pod<B, K> {
    /// Access the underlying buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::Pod;
    ///
    /// let mut pod = Pod::array();
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
    /// use pod::Pod;
    ///
    /// let mut pod = Pod::array();
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

impl<B, K> Pod<B, K>
where
    B: Writer<u64>,
    K: PodKind,
{
    /// Conveniently encode a value into the pod.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::Pod;
    ///
    /// let mut pod = Pod::array();
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
    /// use pod::Pod;
    ///
    /// let mut pod = Pod::array();
    /// pod.push(10i32)?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    pub fn push<T>(self, value: T) -> Result<(), Error>
    where
        T: Encode,
    {
        self.kind.push(value, self.buf)
    }

    /// Encode an unsized value from the pod.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::Pod;
    ///
    /// let mut pod = Pod::array();
    /// pod.push_unsized(&b"hello world"[..])?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    pub fn push_unsized<T>(self, value: &T) -> Result<(), Error>
    where
        T: ?Sized + EncodeUnsized,
    {
        self.kind.push_unsized(value, self.buf)
    }

    /// Encode a `None` value.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::Pod;
    ///
    /// let mut pod = Pod::array();
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
    /// let mut pod = Pod::array();
    /// assert!(pod.as_mut().push_array(Type::STRING, |_| Ok(())).is_err());
    ///
    /// let mut pod = Pod::array();
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
    /// let mut pod = Pod::array();
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
        self,
        child_type: Type,
        f: impl FnOnce(&mut ArrayBuilder<B, K>) -> Result<(), Error>,
    ) -> Result<(), Error> {
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
    /// let mut pod = Pod::array();
    /// assert!(pod.push_unsized_array(Type::INT, 5, |_| Ok(())).is_err());
    ///
    /// let mut pod = Pod::array();
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
    /// let mut pod = Pod::array();
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
        self,
        child_type: Type,
        child_size: usize,
        f: impl FnOnce(&mut ArrayBuilder<B, K>) -> Result<(), Error>,
    ) -> Result<(), Error> {
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
    /// let mut pod = Pod::array();
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
        self,
        f: impl FnOnce(&mut StructBuilder<B, K>) -> Result<(), Error>,
    ) -> Result<(), Error> {
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
    /// let mut pod = Pod::array();
    /// pod.as_mut().push_object(10, 20, |obj| {
    ///     obj.property(1)?.push(1i32)?;
    ///     obj.property(2)?.push(2i32)?;
    ///     obj.property(3)?.push(3i32)?;
    ///     Ok(())
    /// })?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn push_object(
        self,
        object_type: impl RawId,
        object_id: impl RawId,
        f: impl FnOnce(&mut ObjectBuilder<B, K>) -> Result<(), Error>,
    ) -> Result<(), Error> {
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
    /// let mut pod = Pod::array();
    /// pod.as_mut().encode_sequence(|seq| {
    ///     seq.control(1, 0)?.push(1i32)?;
    ///     seq.control(2, 0)?.push(2i32)?;
    ///     seq.control(3, 0)?.push(3i32)?;
    ///     Ok(())
    /// })?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn encode_sequence(
        self,
        f: impl FnOnce(&mut SequenceBuilder<B, K>) -> Result<(), Error>,
    ) -> Result<(), Error> {
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
    /// let mut pod = Pod::array();
    /// pod.as_mut().push_choice(ChoiceType::RANGE, Type::INT, |choice| {
    ///     choice.child().push(1i32)?;
    ///     Ok(())
    /// })?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn push_choice(
        self,
        choice: ChoiceType,
        child_type: Type,
        f: impl FnOnce(&mut ChoiceBuilder<B, K>) -> Result<(), Error>,
    ) -> Result<(), Error> {
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
    /// let mut pod = Pod::array();
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
        f: impl FnOnce(&mut Pod<B>) -> Result<(), Error>,
    ) -> Result<(), Error> {
        // Reserve space for the header of the choice which includes its size
        // that will be determined later.
        let header = self.buf.reserve([0, Type::POD.into_u32()])?;

        let mut pod = Pod::new(self.buf);

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

    /// Borrow the current pod mutably, allowing multiple elements to be encoded
    /// into it or the pod immediately re-used.
    #[inline]
    pub fn as_mut(&mut self) -> Pod<B::Mut<'_>> {
        Pod::new(self.buf.borrow_mut())
    }
}

impl<'de, B, K> Pod<B, K>
where
    B: Reader<'de, u64>,
    K: PodKind,
{
    /// Skip a value in the pod.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    /// let mut pod = Pod::array();
    ///
    /// let mut array = pod.as_mut().push_array(Type::INT, |array| {
    ///     array.child().push(10i32)?;
    ///     array.child().push(20i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let pod = pod.as_ref();
    /// let mut array = pod.next_array()?;
    /// assert!(!array.is_empty());
    /// array.next().unwrap();
    /// assert_eq!(array.next().unwrap().next::<i32>()?, 20i32);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn skip(self) -> Result<(), Error> {
        self.into_typed()?.skip()
    }

    /// Conveniently decode a value from the pod.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::Pod;
    ///
    /// let mut pod = Pod::array();
    /// pod.as_mut().encode((10i32, "hello world", [1u32, 2u32]))?;
    ///
    /// let (a, s, [c, d]) = pod.as_ref().decode::<(i32, String, [u32; 2])>()?;
    ///
    /// assert_eq!(a, 10i32);
    /// assert_eq!(s, "hello world");
    /// assert_eq!(c, 1u32);
    /// assert_eq!(d, 2u32);
    /// # Ok::<_, pod::Error>(())
    /// ```
    pub fn decode<T>(self) -> Result<T, Error>
    where
        T: DecodeFrom<'de>,
    {
        T::decode_from(self)
    }

    /// Encode a value from the pod.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::Pod;
    ///
    /// let mut pod = Pod::array();
    /// pod.as_mut().push(10i32)?;
    /// assert_eq!(pod.as_ref().next::<i32>()?, 10i32);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn next<T>(self) -> Result<T, Error>
    where
        T: Decode<'de>,
    {
        self.into_typed()?.next::<T>()
    }

    /// Read the next unsized value from the pod.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::Pod;
    ///
    /// let mut pod = Pod::array();
    /// pod.as_mut().push_unsized(&b"hello world"[..])?;
    /// assert_eq!(pod.as_ref().next_unsized::<[u8], _>(<[u8]>::to_owned)?, b"hello world");
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn next_unsized<T, V>(self, visitor: V) -> Result<V::Ok, Error>
    where
        T: ?Sized + DecodeUnsized<'de>,
        V: Visitor<'de, T>,
    {
        self.into_typed()?.next_unsized(visitor)
    }

    /// Read the next unsized value from the pod.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::Pod;
    ///
    /// let mut pod = Pod::array();
    /// pod.as_mut().push_unsized(&b"hello world"[..])?;
    ///
    /// let pod = pod.as_ref();
    /// assert_eq!(pod.next_borrowed::<[u8]>()?, b"hello world");
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn next_borrowed<T>(self) -> Result<&'de T, Error>
    where
        T: ?Sized + DecodeUnsized<'de>,
    {
        self.into_typed()?.next_borrowed()
    }

    /// Read the next optional value from the pod.
    ///
    /// This returns [`None`] if the encoded value is [`None`], otherwise a pod
    /// for the value is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::Pod;
    ///
    /// let mut pod = Pod::array();
    /// pod.as_mut().push_none()?;
    /// assert!(pod.as_ref().next_option()?.is_none());
    ///
    /// let mut pod = Pod::array();
    /// pod.as_mut().push(true)?;
    ///
    /// let Some(mut pod) = pod.as_ref().next_option()? else {
    ///     panic!("expected some value");
    /// };
    ///
    /// assert!(pod.as_ref().next::<bool>()?);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn next_option(self) -> Result<Option<Self>, Error> {
        let [_, ty] = self.buf.peek::<[u32; 2]>()?;

        match Type::new(ty) {
            Type::NONE => Ok(None),
            _ => Ok(Some(self)),
        }
    }

    /// Read the next array.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = Pod::array();
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
    /// Encoding an empty array:
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = Pod::array();
    /// pod.as_mut().push_array(Type::INT, |_| Ok(()))?;
    ///
    /// let mut array = pod.as_ref().next_array()?;
    ///
    /// assert!(array.is_empty());
    /// assert_eq!(array.len(), 0);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn next_array(self) -> Result<Array<B>, Error> {
        self.into_typed()?.next_array()
    }

    /// Read the next struct.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, TypedPod};
    ///
    /// let mut pod = Pod::array();
    /// pod.as_mut().push_struct(|st| {
    ///     st.field().push(1i32)?;
    ///     st.field().push(2i32)?;
    ///     st.field().push(3i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let mut st = pod.as_ref().next_struct()?;
    /// assert!(!st.is_empty());
    /// assert_eq!(st.field()?.next::<i32>()?, 1i32);
    /// assert_eq!(st.field()?.next::<i32>()?, 2i32);
    /// assert_eq!(st.field()?.next::<i32>()?, 3i32);
    /// assert!(st.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    ///
    /// Decoding an empty struct:
    ///
    /// ```
    /// use pod::{Pod, TypedPod};
    ///
    /// let mut pod = Pod::array();
    /// pod.as_mut().push_struct(|_| Ok(()))?;
    ///
    /// let st = pod.as_ref().next_struct()?;
    /// assert!(st.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn next_struct(self) -> Result<Struct<B>, Error> {
        self.into_typed()?.next_struct()
    }

    /// Read the next object.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = Pod::array();
    /// pod.as_mut().push_object(10, 20, |obj| {
    ///     obj.property_with_flags(1, 0b001)?.push(1i32)?;
    ///     obj.property_with_flags(2, 0b010)?.push(2i32)?;
    ///     obj.property_with_flags(3, 0b100)?.push(3i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let mut obj = pod.as_ref().next_object()?;
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
    ///
    /// Decoding an empty object:
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = Pod::array();
    /// pod.as_mut().push_object(10, 20, |_| Ok(()))?;
    ///
    /// let obj = pod.as_ref().next_object()?;
    /// assert!(obj.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn next_object(self) -> Result<Object<B>, Error> {
        self.into_typed()?.next_object()
    }

    /// Read the next sequence.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = Pod::array();
    /// pod.as_mut().encode_sequence(|seq| {
    ///     seq.control(1, 10)?.push(1i32)?;
    ///     seq.control(2, 20)?.push(2i32)?;
    ///     seq.control(3, 30)?.push(3i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let mut seq = pod.as_ref().next_sequence()?;
    /// assert!(!seq.is_empty());
    ///
    /// let c = seq.control()?;
    /// assert_eq!(c.offset(), 1);
    /// assert_eq!(c.ty(), 10);
    /// assert_eq!(c.value().next::<i32>()?, 1);
    ///
    /// let c = seq.control()?;
    /// assert_eq!(c.offset(), 2);
    /// assert_eq!(c.ty(), 20);
    /// assert_eq!(c.value().next::<i32>()?, 2);
    ///
    /// let c = seq.control()?;
    /// assert_eq!(c.offset(), 3);
    /// assert_eq!(c.ty(), 30);
    /// assert_eq!(c.value().next::<i32>()?, 3);
    ///
    /// assert!(seq.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    ///
    /// Encoding an empty sequence:
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = Pod::array();
    /// pod.as_mut().encode_sequence(|_| Ok(()))?;
    ///
    /// let seq = pod.as_ref().next_sequence()?;
    /// assert!(seq.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn next_sequence(self) -> Result<Sequence<B>, Error> {
        self.into_typed()?.next_sequence()
    }

    /// Read the next choice.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ChoiceType, Pod, Type};
    ///
    /// let mut pod = Pod::array();
    /// pod.as_mut().push_choice(ChoiceType::RANGE, Type::INT, |choice| {
    ///     choice.child().push(10i32)?;
    ///     choice.child().push(0i32)?;
    ///     choice.child().push(30i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let mut choice = pod.as_ref().next_choice()?;
    /// assert!(!choice.is_empty());
    /// assert_eq!(choice.next().unwrap().next::<i32>()?, 10);
    /// assert_eq!(choice.next().unwrap().next::<i32>()?, 0);
    /// assert_eq!(choice.next().unwrap().next::<i32>()?, 30);
    /// assert!(choice.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    ///
    /// Encoding an empty choice:
    ///
    /// ```
    /// use pod::{ChoiceType, Pod, Type};
    ///
    /// let mut pod = Pod::array();
    /// pod.as_mut().push_choice(ChoiceType::RANGE, Type::INT, |_| Ok(()))?;
    ///
    /// let mut choice = pod.as_ref().next_choice()?;
    /// assert!(choice.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn next_choice(self) -> Result<Choice<B>, Error> {
        self.into_typed()?.next_choice()
    }

    /// Read the next nested pod.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, TypedPod};
    ///
    /// let mut pod = Pod::array();
    /// pod.as_mut().push_pod(|pod| {
    ///     pod.as_mut().push_struct(|st| {
    ///         st.field().push(1i32)?;
    ///         st.field().push(2i32)?;
    ///         st.field().push(3i32)?;
    ///         Ok(())
    ///     })
    /// })?;
    ///
    /// let pod = pod.as_ref().next_pod()?;
    /// let mut st = pod.as_ref().next_struct()?;
    /// assert!(!st.is_empty());
    /// assert_eq!(st.field()?.next::<i32>()?, 1i32);
    /// assert_eq!(st.field()?.next::<i32>()?, 2i32);
    /// assert_eq!(st.field()?.next::<i32>()?, 3i32);
    /// assert!(st.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn next_pod(self) -> Result<Pod<B>, Error> {
        self.into_typed()?.next_pod()
    }

    /// Borrow the current pod mutably, allowing multiple elements to be encoded
    /// into it or the pod immediately re-used.
    #[inline]
    pub fn as_read_mut(&mut self) -> Pod<B::Mut<'_>> {
        Pod::new(self.buf.borrow_mut())
    }

    /// Convert the [`Pod`] into a [`TypedPod`] taking ownership of the current
    /// buffer.
    ///
    /// A typed pod knows about the size and type of the data it contains,
    /// allowing it to be inspected through the relevant [`TypedPod::size`] and
    /// [`TypedPod::ty`] APIs.
    ///
    /// # Errors
    ///
    /// This errors if the pod does not wrap a buffer containing a valid pod.
    #[inline]
    pub fn into_typed(self) -> Result<TypedPod<B>, Error> {
        TypedPod::from_reader(self.buf)
    }

    /// Convert the [`Pod`] into a [`TypedPod`] mutably borrowing the current
    /// buffer.
    ///
    /// A typed pod knows about the size and type of the data it contains,
    /// allowing it to be inspected through the relevant [`TypedPod::size`] and
    /// [`TypedPod::ty`] APIs.
    ///
    /// # Errors
    ///
    /// This errors if the pod does not wrap a buffer containing a valid pod.
    #[inline]
    pub fn as_typed_mut(&mut self) -> Result<TypedPod<B::Mut<'_>>, Error> {
        TypedPod::from_reader(self.buf.borrow_mut())
    }
}

impl<B, K> Pod<B, K>
where
    B: AsReader<u64>,
    K: Copy,
{
    /// Coerce any pod into an owned pod.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::Pod;
    ///
    /// let mut pod = Pod::array();
    /// pod.as_mut().push(10i32)?;
    ///
    /// let pod = pod.to_owned();
    ///
    /// assert_eq!(pod.as_ref().next::<i32>()?, 10i32);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[cfg(feature = "alloc")]
    pub fn to_owned(&self) -> Pod<Box<[u64]>, K> {
        Pod {
            buf: Box::from(self.buf.as_reader().as_slice()),
            kind: self.kind,
        }
    }

    /// Coerce an owned pod into a borrowed pod which can be used for reading.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::Pod;
    ///
    /// let mut pod = Pod::array();
    /// pod.as_mut().push(10i32)?;
    ///
    /// let pod = pod.to_owned();
    ///
    /// assert_eq!(pod.as_ref().next::<i32>()?, 10i32);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn as_ref(&self) -> Pod<B::AsReader<'_>, K> {
        Pod {
            buf: self.buf.as_reader(),
            kind: self.kind,
        }
    }
}

/// [`Encode`] implementation for [`Pod`].
///
/// # Examples
///
/// ```
/// use pod::{Pod, Type};
///
/// let mut pod = Pod::array();
/// pod.as_mut().push_object(10, 20, |obj| {
///     obj.property_with_flags(1, 0b001)?.push(1i32)?;
///     obj.property_with_flags(2, 0b010)?.push(2i32)?;
///     obj.property_with_flags(3, 0b100)?.push(3i32)?;
///     Ok(())
/// })?;
///
/// let mut pod2 = Pod::array();
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
impl<B> EncodeUnsized for Pod<B>
where
    B: AsReader<u64>,
{
    const TYPE: Type = Type::POD;

    #[inline]
    fn size(&self) -> usize {
        self.buf.as_reader().bytes_len()
    }

    #[inline]
    fn write_content(&self, mut writer: impl Writer<u64>) -> Result<(), Error> {
        writer.write_words(self.buf.as_reader().as_slice())
    }
}

crate::macros::encode_into_unsized!(impl [B] Pod<B> where B: AsReader<u64>);

impl<B, K> fmt::Debug for Pod<B, K>
where
    B: AsReader<u64>,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match TypedPod::from_reader(self.buf.as_reader()) {
            Ok(pod) => pod.fmt(f),
            Err(e) => e.fmt(f),
        }
    }
}
