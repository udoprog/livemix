use core::fmt;

#[cfg(feature = "alloc")]
use crate::buf::AllocError;
use crate::{
    Array, ArrayBuf, AsSlice, BufferUnderflow, Choice, Error, Object, PackedPod, PodStream,
    ReadPod, Readable, Reader, Sequence, SizedReadable, Slice, Struct, Type, UnsizedReadable,
    UnsizedWritable, Value, Visitor, Writer,
};
#[cfg(feature = "alloc")]
use crate::{DynamicBuf, PaddedPod};

/// A POD (Plain Old Data) handler.
///
/// This is a wrapper that can be used for encoding and decoding data.
pub struct Pod<B, P = PaddedPod> {
    buf: B,
    kind: P,
}

impl<B, P> Pod<B, P> {
    #[inline]
    pub(crate) const fn with_kind(buf: B, kind: P) -> Self {
        Pod { buf, kind }
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
    /// let mut buf = ArrayBuf::default();
    /// _ = Pod::new(&mut buf);
    ///
    /// _ = Pod::new(ArrayBuf::<16>::new());
    /// ```
    #[inline]
    pub const fn new(buf: B) -> Self {
        Pod {
            buf,
            kind: PaddedPod,
        }
    }
}

impl<B> Pod<B, PackedPod> {
    /// Construct a new [`Pod`] arround the specified buffer `B`.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ArrayBuf, Pod};
    ///
    /// let mut buf = ArrayBuf::default();
    /// _ = Pod::new(&mut buf);
    ///
    /// _ = Pod::new(ArrayBuf::<16>::new());
    /// ```
    #[inline]
    pub const fn packed(buf: B) -> Self {
        Pod {
            buf,
            kind: PackedPod,
        }
    }
}

impl<B, P> Clone for Pod<B, P>
where
    B: Clone,
    P: Copy,
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
    /// let mut pod = pod::dynamic();
    /// pod.as_mut().write(10i32)?;
    /// assert_eq!(pod.as_ref().read_sized::<i32>()?, 10i32);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub const fn dynamic() -> Self {
        Self::new(DynamicBuf::new())
    }
}

#[cfg(feature = "alloc")]
impl<P> Pod<DynamicBuf, P>
where
    P: ReadPod,
{
    /// Clear the current builder.
    ///
    /// This will clear the buffer and reset the pod to an empty state.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut pod = pod::dynamic();
    /// pod.as_mut().write(10i32)?;
    /// assert_eq!(pod.as_ref().read_sized::<i32>()?, 10i32);
    /// pod.clear();
    /// pod.as_mut().write(20i32)?;
    /// assert_eq!(pod.as_ref().read_sized::<i32>()?, 20i32);
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
    /// pod.as_mut().write(10i32)?;
    /// assert_eq!(pod.as_ref().read_sized::<i32>()?, 10i32);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub const fn array() -> Self {
        Self::new(ArrayBuf::new())
    }
}

impl<P> Pod<ArrayBuf, P>
where
    P: ReadPod,
{
    /// Clear the current builder.
    ///
    /// This will clear the buffer and reset the pod to an empty state.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut pod = pod::array();
    /// pod.as_mut().write(10i32)?;
    /// assert_eq!(pod.as_ref().read_sized::<i32>()?, 10i32);
    /// pod.clear();
    /// pod.as_mut().write(20i32)?;
    /// assert_eq!(pod.as_ref().read_sized::<i32>()?, 20i32);
    /// # Ok::<_, pod::Error>(())
    /// ```
    pub fn clear(&mut self) {
        self.buf.clear();
    }
}

impl<B, P> Pod<B, P> {
    /// Access the underlying buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut pod = pod::array();
    /// pod.as_mut().write(10i32)?;
    ///
    /// let buf = pod.as_buf();
    /// assert_eq!(buf.as_bytes().len(), 16);
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
    /// pod.as_mut().write(10i32)?;
    ///
    /// let buf = pod.into_buf();
    /// assert_eq!(buf.as_bytes().len(), 16);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn into_buf(self) -> B {
        self.buf
    }
}

impl<'de, B, P> Pod<B, P>
where
    B: Reader<'de>,
    P: ReadPod,
{
    /// Skip a value in the pod and return the number of bytes skipped.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut pod = pod::array();
    ///
    /// pod.as_mut().write((1, 2, "hello world", 4));
    ///
    /// let mut pod = pod.as_ref();
    /// assert_eq!(pod.as_mut().read_sized::<i32>()?, 1);
    /// assert_eq!(pod.as_mut().read_sized::<i32>()?, 2);
    /// assert_eq!(pod.as_mut().skip()?, 12);
    /// assert_eq!(pod.as_mut().read_sized::<i32>()?, 4);
    /// assert!(pod.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn skip(self) -> Result<usize, Error> {
        self.into_value()?.skip()
    }

    /// Conveniently decode a value from the pod.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut pod = pod::array();
    /// pod.as_mut().write((10i32, "hello world", [1u32, 2u32]))?;
    ///
    /// let (a, s, [c, d]) = pod.as_ref().read::<(i32, String, [u32; 2])>()?;
    ///
    /// assert_eq!(a, 10i32);
    /// assert_eq!(s, "hello world");
    /// assert_eq!(c, 1u32);
    /// assert_eq!(d, 2u32);
    /// # Ok::<_, pod::Error>(())
    /// ```
    pub fn read<T>(mut self) -> Result<T, Error>
    where
        T: Readable<'de>,
    {
        T::read_from(&mut self)
    }

    /// Read a sized value from the pod.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut pod = pod::array();
    /// pod.as_mut().write(10i32)?;
    /// assert_eq!(pod.as_ref().read_sized::<i32>()?, 10i32);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn read_sized<T>(self) -> Result<T, Error>
    where
        T: SizedReadable<'de>,
    {
        self.into_value()?.read_sized::<T>()
    }

    /// Read an unsized value from the pod.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut pod = pod::array();
    /// pod.as_mut().write_unsized(&b"hello world"[..])?;
    ///
    /// let pod = pod.as_ref();
    /// assert_eq!(pod.read_unsized::<[u8]>()?, b"hello world");
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn read_unsized<T>(self) -> Result<&'de T, Error>
    where
        T: ?Sized + UnsizedReadable<'de>,
    {
        self.into_value()?.read_unsized()
    }

    /// Read an unsized value from the pod.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut pod = pod::array();
    /// pod.as_mut().write_unsized(&b"hello world"[..])?;
    /// assert_eq!(pod.as_ref().visit_unsized::<[u8], _>(<[u8]>::to_owned)?, b"hello world");
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn visit_unsized<T, V>(self, visitor: V) -> Result<V::Ok, Error>
    where
        T: ?Sized + UnsizedReadable<'de>,
        V: Visitor<'de, T>,
    {
        self.into_value()?.visit_unsized(visitor)
    }

    /// Read an optional value from the pod.
    ///
    /// This returns [`None`] if the encoded value is [`None`], otherwise a pod
    /// for the value is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut pod = pod::array();
    /// pod.as_mut().write_none()?;
    /// assert!(pod.as_ref().read_option()?.is_none());
    ///
    /// let mut pod = pod::array();
    /// pod.as_mut().write(true)?;
    ///
    /// let Some(mut pod) = pod.as_ref().read_option()? else {
    ///     panic!("expected some value");
    /// };
    ///
    /// assert!(pod.as_ref().read_sized::<bool>()?);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn read_option(self) -> Result<Option<Self>, Error> {
        let [_, ty] = self.buf.peek::<[u32; 2]>()?;

        match Type::new(ty) {
            Type::NONE => Ok(None),
            _ => Ok(Some(self)),
        }
    }

    /// Read an array.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = pod::array();
    ///
    /// pod.as_mut().write_array(Type::INT, |array| {
    ///     array.child().write(1i32)?;
    ///     array.child().write(2i32)?;
    ///     array.child().write(3i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let mut array = pod.as_ref().read_array()?;
    ///
    /// assert!(!array.is_empty());
    /// assert_eq!(array.len(), 3);
    ///
    /// assert_eq!(array.next()?.unwrap().read_sized::<i32>()?, 1i32);
    /// assert_eq!(array.next()?.unwrap().read_sized::<i32>()?, 2i32);
    /// assert_eq!(array.next()?.unwrap().read_sized::<i32>()?, 3i32);
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
    /// pod.as_mut().write_array(Type::INT, |_| Ok(()))?;
    ///
    /// let mut array = pod.as_ref().read_array()?;
    ///
    /// assert!(array.is_empty());
    /// assert_eq!(array.len(), 0);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn read_array(self) -> Result<Array<Slice<'de>>, Error> {
        self.into_value()?.read_array()
    }

    /// Read a struct.
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
    /// assert!(!st.is_empty());
    /// assert_eq!(st.field()?.read_sized::<i32>()?, 1i32);
    /// assert_eq!(st.field()?.read_sized::<i32>()?, 2i32);
    /// assert_eq!(st.field()?.read_sized::<i32>()?, 3i32);
    /// assert!(st.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    ///
    /// Decoding an empty struct:
    ///
    /// ```
    /// let mut pod = pod::array();
    /// pod.as_mut().write_struct(|_| Ok(()))?;
    ///
    /// let st = pod.as_ref().read_struct()?;
    /// assert!(st.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn read_struct(self) -> Result<Struct<Slice<'de>>, Error> {
        self.into_value()?.read_struct()
    }

    /// Read an object.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = pod::array();
    /// pod.as_mut().write_object(10, 20, |obj| {
    ///     obj.property(1).flags(0b001).write(1i32)?;
    ///     obj.property(2).flags(0b010).write(2i32)?;
    ///     obj.property(3).flags(0b100).write(3i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let mut obj = pod.as_ref().read_object()?;
    /// assert!(!obj.is_empty());
    ///
    /// let p = obj.property()?;
    /// assert_eq!(p.key::<u32>(), 1);
    /// assert_eq!(p.flags(), 0b001);
    /// assert_eq!(p.value().read_sized::<i32>()?, 1);
    ///
    /// let p = obj.property()?;
    /// assert_eq!(p.key::<u32>(), 2);
    /// assert_eq!(p.flags(), 0b010);
    /// assert_eq!(p.value().read_sized::<i32>()?, 2);
    ///
    /// let p = obj.property()?;
    /// assert_eq!(p.key::<u32>(), 3);
    /// assert_eq!(p.flags(), 0b100);
    /// assert_eq!(p.value().read_sized::<i32>()?, 3);
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
    /// pod.as_mut().write_object(10, 20, |_| Ok(()))?;
    ///
    /// let obj = pod.as_ref().read_object()?;
    /// assert!(obj.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn read_object(self) -> Result<Object<Slice<'de>>, Error> {
        self.into_value()?.read_object()
    }

    /// Read a sequence.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = pod::array();
    /// pod.as_mut().write_sequence(|seq| {
    ///     seq.control().offset(1).ty(10).write(1i32)?;
    ///     seq.control().offset(2).ty(20).write(2i32)?;
    ///     seq.control().offset(3).ty(30).write(3i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let mut seq = pod.as_ref().read_sequence()?;
    /// assert!(!seq.is_empty());
    ///
    /// let c = seq.control()?;
    /// assert_eq!(c.offset(), 1);
    /// assert_eq!(c.ty(), 10);
    /// assert_eq!(c.value().read_sized::<i32>()?, 1);
    ///
    /// let c = seq.control()?;
    /// assert_eq!(c.offset(), 2);
    /// assert_eq!(c.ty(), 20);
    /// assert_eq!(c.value().read_sized::<i32>()?, 2);
    ///
    /// let c = seq.control()?;
    /// assert_eq!(c.offset(), 3);
    /// assert_eq!(c.ty(), 30);
    /// assert_eq!(c.value().read_sized::<i32>()?, 3);
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
    /// pod.as_mut().write_sequence(|_| Ok(()))?;
    ///
    /// let seq = pod.as_ref().read_sequence()?;
    /// assert!(seq.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn read_sequence(self) -> Result<Sequence<Slice<'de>>, Error> {
        self.into_value()?.read_sequence()
    }

    /// Read a choice.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ChoiceType, Pod, Type};
    ///
    /// let mut pod = pod::array();
    /// pod.as_mut().write_choice(ChoiceType::RANGE, Type::INT, |choice| {
    ///     choice.child().write(10i32)?;
    ///     choice.child().write(0i32)?;
    ///     choice.child().write(30i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let mut choice = pod.as_ref().read_choice()?;
    /// assert!(!choice.is_empty());
    /// assert_eq!(choice.next().unwrap().read_sized::<i32>()?, 10);
    /// assert_eq!(choice.next().unwrap().read_sized::<i32>()?, 0);
    /// assert_eq!(choice.next().unwrap().read_sized::<i32>()?, 30);
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
    /// pod.as_mut().write_choice(ChoiceType::RANGE, Type::INT, |_| Ok(()))?;
    ///
    /// let mut choice = pod.as_ref().read_choice()?;
    /// assert!(choice.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn read_choice(self) -> Result<Choice<Slice<'de>>, Error> {
        self.into_value()?.read_choice()
    }

    /// Read a nested pod.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut pod = pod::array();
    /// pod.as_mut().write_pod(|pod| {
    ///     pod.as_mut().write_struct(|st| {
    ///         st.field().write(1i32)?;
    ///         st.field().write(2i32)?;
    ///         st.field().write(3i32)?;
    ///         Ok(())
    ///     })
    /// })?;
    ///
    /// let pod = pod.as_ref().read_pod()?;
    /// let mut st = pod.as_ref().read_struct()?;
    /// assert!(!st.is_empty());
    /// assert_eq!(st.field()?.read_sized::<i32>()?, 1i32);
    /// assert_eq!(st.field()?.read_sized::<i32>()?, 2i32);
    /// assert_eq!(st.field()?.read_sized::<i32>()?, 3i32);
    /// assert!(st.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn read_pod(self) -> Result<Pod<Slice<'de>, PackedPod>, Error> {
        self.into_value()?.read_pod()
    }

    /// Borrow the current pod mutably, allowing multiple elements to be encoded
    /// into it or the pod immediately re-used.
    #[inline]
    pub fn as_mut(&mut self) -> Pod<B::Mut<'_>, P>
    where
        P: Copy,
    {
        Pod::with_kind(self.buf.borrow_mut(), self.kind)
    }

    /// Convert the [`Pod`] into a [`Value`].
    ///
    /// A [`Value`] is an opaque representation of the value inside of a pod,
    /// where we know about its size and type. It can be read using associated
    /// methods.
    ///
    /// # Errors
    ///
    /// This errors if the pod does not wrap a buffer containing a valid pod.
    #[inline]
    pub fn into_value(self) -> Result<Value<Slice<'de>>, Error> {
        let (pod, buf) = Value::from_reader(self.buf)?;
        self.kind.unpad(buf)?;
        Ok(pod)
    }
}

impl<B, P> Pod<B, P>
where
    B: AsSlice,
{
    /// Test if the typed pod is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    /// let mut pod = pod::array();
    ///
    /// pod.as_mut().write(1);
    ///
    /// let mut pod = pod.as_ref();
    /// assert!(!pod.is_empty());
    /// assert_eq!(pod.as_mut().read_sized::<i32>()?, 1);
    /// assert!(pod.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    pub fn is_empty(&self) -> bool {
        self.buf.as_slice().is_empty()
    }
}

impl<B, P> Pod<B, P>
where
    B: AsSlice,
    P: Copy,
{
    /// Coerce any pod into an owned pod.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut pod = pod::array();
    /// pod.as_mut().write(10i32)?;
    ///
    /// let pod = pod.to_owned()?;
    ///
    /// assert_eq!(pod.as_ref().read_sized::<i32>()?, 10i32);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[cfg(feature = "alloc")]
    pub fn to_owned(&self) -> Result<Pod<DynamicBuf, P>, AllocError> {
        Ok(Pod::with_kind(
            DynamicBuf::from_slice(self.buf.as_slice().as_bytes())?,
            self.kind,
        ))
    }

    /// Coerce an owned pod into a borrowed pod which can be used for reading.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut pod = pod::array();
    /// pod.as_mut().write(10i32)?;
    ///
    /// let pod = pod.to_owned()?;
    /// assert_eq!(pod.as_ref().read_sized::<i32>()?, 10i32);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn as_ref(&self) -> Pod<Slice<'_>, P> {
        Pod::with_kind(self.buf.as_slice(), self.kind)
    }
}

impl<'de, B, P> PodStream<'de> for Pod<B, P>
where
    B: Reader<'de>,
    P: ReadPod,
{
    type Item = Value<Slice<'de>>;

    #[inline]
    fn next(&mut self) -> Result<Self::Item, Error> {
        let (size, ty) = self.buf.header()?;
        let buf = self.buf.split(size).ok_or(BufferUnderflow)?;
        self.kind.unpad(self.buf.borrow_mut())?;
        Ok(Value::new(buf, size, ty))
    }
}

/// [`UnsizedWritable`] implementation for [`Pod`].
///
/// # Examples
///
/// ```
/// use pod::{Pod, Type};
///
/// let mut pod = pod::array();
/// pod.as_mut().write_object(10, 20, |obj| {
///     obj.property(1).flags(0b001).write(1i32)?;
///     obj.property(2).flags(0b010).write(2i32)?;
///     obj.property(3).flags(0b100).write(3i32)?;
///     Ok(())
/// })?;
///
/// let mut pod2 = pod::array();
/// pod2.as_mut().write(pod)?;
///
/// let mut obj = pod2.as_ref().read_pod()?.read_object()?;
/// assert!(!obj.is_empty());
///
/// let p = obj.property()?;
/// assert_eq!(p.key::<u32>(), 1);
/// assert_eq!(p.flags(), 0b001);
/// assert_eq!(p.value().read_sized::<i32>()?, 1);
///
/// let p = obj.property()?;
/// assert_eq!(p.key::<u32>(), 2);
/// assert_eq!(p.flags(), 0b010);
/// assert_eq!(p.value().read_sized::<i32>()?, 2);
///
/// let p = obj.property()?;
/// assert_eq!(p.key::<u32>(), 3);
/// assert_eq!(p.flags(), 0b100);
/// assert_eq!(p.value().read_sized::<i32>()?, 3);
///
/// assert!(obj.is_empty());
/// # Ok::<_, pod::Error>(())
/// ```
impl<B, P> UnsizedWritable for Pod<B, P>
where
    B: AsSlice,
    P: ReadPod,
{
    const TYPE: Type = Type::POD;

    #[inline]
    fn size(&self) -> Option<usize> {
        Some(self.buf.as_slice().len())
    }

    #[inline]
    fn write_unsized(&self, mut writer: impl Writer) -> Result<(), Error> {
        writer.write(self.buf.as_slice().as_bytes())
    }
}

crate::macros::encode_into_unsized!(impl [B, P] Pod<B, P> where B: AsSlice, P: ReadPod);

impl<B, P> fmt::Debug for Pod<B, P>
where
    B: AsSlice,
    P: ReadPod,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.as_ref().into_value() {
            Ok(pod) => pod.fmt(f),
            Err(e) => e.fmt(f),
        }
    }
}
