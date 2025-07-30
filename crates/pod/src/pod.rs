use core::fmt;

#[cfg(feature = "alloc")]
use alloc::boxed::Box;

#[cfg(feature = "alloc")]
use crate::DynamicBuf;
use crate::de::{Array, Choice, Object, Sequence, Struct};
use crate::{ArrayBuf, DecodeFrom};
use crate::{
    AsReader, Decode, DecodeUnsized, EncodeUnsized, Error, Reader, Type, TypedPod, Visitor, Writer,
};

/// A POD (Plain Old Data) handler.
///
/// This is a wrapper that can be used for encoding and decoding data.
pub struct Pod<B> {
    buf: B,
}

impl<B> Clone for Pod<B>
where
    B: Clone,
{
    #[inline]
    fn clone(&self) -> Self {
        Pod {
            buf: self.buf.clone(),
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

impl Pod<ArrayBuf> {
    /// Construct a new [`Pod`] with a 128 word-sized array buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut pod = pod::array();
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
        Pod { buf }
    }
}

impl<B> Pod<B> {
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

impl<'de, B> Pod<B>
where
    B: Reader<'de, u64>,
{
    /// Skip a value in the pod.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    /// let mut pod = pod::array();
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
    /// let mut pod = pod::array();
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
    /// let mut pod = pod::array();
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
    /// let mut pod = pod::array();
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
    /// let mut pod = pod::array();
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
    /// let mut pod = pod::array();
    /// pod.as_mut().push_none()?;
    /// assert!(pod.as_ref().next_option()?.is_none());
    ///
    /// let mut pod = pod::array();
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
    /// let mut pod = pod::array();
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
    /// let mut pod = pod::array();
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
    /// let mut pod = pod::array();
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
    /// let mut pod = pod::array();
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
    /// let mut pod = pod::array();
    /// pod.as_mut().push_object(10, 20, |obj| {
    ///     obj.property(1).flags(0b001).push(1i32)?;
    ///     obj.property(2).flags(0b010).push(2i32)?;
    ///     obj.property(3).flags(0b100).push(3i32)?;
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
    /// let mut pod = pod::array();
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
    /// let mut pod = pod::array();
    /// pod.as_mut().push_sequence(|seq| {
    ///     seq.control().offset(1).ty(10).push(1i32)?;
    ///     seq.control().offset(2).ty(20).push(2i32)?;
    ///     seq.control().offset(3).ty(30).push(3i32)?;
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
    /// let mut pod = pod::array();
    /// pod.as_mut().push_sequence(|_| Ok(()))?;
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
    /// let mut pod = pod::array();
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
    /// let mut pod = pod::array();
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

impl<B> Pod<B>
where
    B: AsReader<u64>,
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
        Pod {
            buf: Box::from(self.buf.as_reader().as_slice()),
        }
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
        Pod {
            buf: self.buf.as_reader(),
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

impl<B> fmt::Debug for Pod<B>
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
