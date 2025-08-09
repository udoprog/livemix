use core::fmt;
use core::mem;

#[cfg(feature = "alloc")]
use crate::DynamicBuf;
use crate::Object;
use crate::PodSink;
use crate::ReadPod;
use crate::Slice;
use crate::SplitReader;
use crate::Struct;
use crate::WriterSlice;
#[cfg(feature = "alloc")]
use crate::buf::AllocError;
use crate::builder::{ArrayBuilder, ChoiceBuilder, ObjectBuilder, SequenceBuilder, StructBuilder};
use crate::error::ErrorKind;
use crate::{ArrayBuf, SizedWritable, Writable};
use crate::{
    AsSlice, BuildPod, ChildPod, ChoiceType, Embeddable, Error, PaddedPod, Pod, RawId, Type,
    TypedPod, UnsizedWritable, Writer,
};

/// A POD (Plain Old Data) handler.
///
/// This is a wrapper that can be used for encoding and decoding data.
pub struct Builder<B, P = PaddedPod> {
    buf: B,
    kind: P,
}

impl<B, P> Builder<B, P> {
    #[inline]
    pub(crate) fn with_kind(buf: B, kind: P) -> Self {
        Builder { buf, kind }
    }

    #[inline]
    pub(crate) fn as_kind_mut(&mut self) -> &mut P {
        &mut self.kind
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
impl<P> Builder<DynamicBuf, P> {
    /// Clear the current builder.
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

#[cfg(feature = "alloc")]
impl<P> Builder<DynamicBuf, P>
where
    P: Copy,
{
    /// Clear the current builder and return a mutable reference to it.
    ///
    /// This will clear the buffer and reset the pod to an empty state.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut pod = pod::dynamic();
    /// pod.as_mut().write(10i32)?;
    /// assert_eq!(pod.as_ref().read_sized::<i32>()?, 10i32);
    /// pod.clear_mut().write(20i32)?;
    /// assert_eq!(pod.as_ref().read_sized::<i32>()?, 20i32);
    /// # Ok::<_, pod::Error>(())
    /// ```
    pub fn clear_mut(&mut self) -> Builder<&mut DynamicBuf, P> {
        self.buf.clear();
        self.as_mut()
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
    /// pod.as_mut().write(10i32)?;
    /// assert_eq!(pod.as_ref().read_sized::<i32>()?, 10i32);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub const fn array() -> Self {
        Self::new(ArrayBuf::new())
    }
}

impl<const N: usize, P> Builder<ArrayBuf<N>, P>
where
    P: Copy,
{
    /// Clear the current builder and return a mutable reference to it.
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

    /// Clear the current builder and return a mutable reference to it.
    ///
    /// This will clear the buffer and reset the pod to an empty state.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut pod = pod::array();
    /// pod.as_mut().write(10i32)?;
    /// assert_eq!(pod.as_ref().read_sized::<i32>()?, 10i32);
    /// pod.clear_mut().write(20i32)?;
    /// assert_eq!(pod.as_ref().read_sized::<i32>()?, 20i32);
    /// # Ok::<_, pod::Error>(())
    /// ```
    pub fn clear_mut(&mut self) -> Builder<&mut ArrayBuf<N>, P> {
        self.buf.clear();
        self.as_mut()
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
            kind: PaddedPod,
        }
    }

    /// Coerce into a [`Builder`] with the current buffer.
    pub fn into_pod(self) -> Pod<B> {
        Pod::new(self.buf)
    }
}

impl<B, P> Builder<B, P>
where
    B: Writer,
    P: Copy,
{
    /// Borrow the current pod mutably, allowing multiple elements to be encoded
    /// into it or the pod immediately re-used.
    #[inline]
    pub fn as_mut(&mut self) -> Builder<B::Mut<'_>, P> {
        Builder::with_kind(self.buf.borrow_mut(), self.kind)
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
    /// pod.as_mut().write(10i32)?;
    /// assert_eq!(pod.take().read::<i32>()?, 10);
    ///
    /// pod.as_mut().write(42i32)?;
    /// assert_eq!(pod.take().read::<i32>()?, 42);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn take(&mut self) -> Pod<B::TakeReader<'_>> {
        Pod::new(self.buf.take_reader())
    }
}

impl<B> Builder<B>
where
    B: AsSlice,
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
    pub fn to_owned(&self) -> Result<Pod<DynamicBuf>, AllocError> {
        Ok(Pod::new(DynamicBuf::from_slice(
            self.buf.as_slice().as_bytes(),
        )?))
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
    pub fn as_ref(&self) -> Pod<Slice<'_>> {
        Pod::new(self.buf.as_slice())
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

impl<B, P> Builder<B, P>
where
    P: BuildPod,
{
    #[inline]
    pub(crate) fn new_with(buf: B, kind: P) -> Self
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

impl<B, P> Builder<B, P>
where
    B: Writer,
    P: BuildPod,
{
    /// Conveniently encode a value into the pod.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut pod = pod::array();
    /// pod.as_mut().write_struct(|st| st.write((10i32, "hello world", [1u32, 2u32])))?;
    ///
    /// let mut pod = pod.as_ref();
    /// let mut st = pod.read_struct()?;
    ///
    /// assert_eq!(st.field()?.read_sized::<i32>()?, 10i32);
    /// assert_eq!(st.field()?.read_unsized::<str>()?, "hello world");
    /// assert_eq!(st.field()?.read_sized::<u32>()?, 1);
    /// assert_eq!(st.field()?.read_sized::<u32>()?, 2);
    /// assert!(st.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    pub fn write<T>(mut self, value: T) -> Result<(), Error>
    where
        T: Writable,
    {
        value.write_into(&mut self)
    }

    /// Conveniently encode a value into the pod and return a read handle
    /// directly to it.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Readable, Writable};
    /// use protocol::id;
    ///
    /// #[derive(Debug, PartialEq, Readable, Writable)]
    /// #[pod(object(type = id::ObjectType::FORMAT, id = id::Param::FORMAT))]
    /// struct RawFormat {
    ///     #[pod(property(key = id::Format::MEDIA_TYPE))]
    ///     media_type: id::MediaType,
    ///     #[pod(property(key = id::Format::MEDIA_SUB_TYPE))]
    ///     media_sub_type: id::MediaSubType,
    ///     #[pod(property(key = id::Format::AUDIO_FORMAT))]
    ///     audio_format: id::AudioFormat,
    ///     #[pod(property(key = id::Format::AUDIO_CHANNELS))]
    ///     channels: u32,
    ///     #[pod(property(key = id::Format::AUDIO_RATE))]
    ///     audio_rate: u32,
    /// }
    ///
    /// let mut pod = pod::array();
    /// let object = pod.as_mut().embed(RawFormat {
    ///     media_type: id::MediaType::AUDIO,
    ///     media_sub_type: id::MediaSubType::DSP,
    ///     audio_format: id::AudioFormat::F32P,
    ///     channels: 2,
    ///     audio_rate: 48000,
    /// })?;
    ///
    /// assert_eq!(object.object_type::<id::ObjectType>(), id::ObjectType::FORMAT);
    /// assert_eq!(object.object_id::<id::Param>(), id::Param::FORMAT);
    ///
    /// let mut obj = object.as_ref();
    ///
    /// let p = obj.property()?;
    /// assert_eq!(p.key::<id::Format>(), id::Format::MEDIA_TYPE);
    /// assert_eq!(p.value().read::<id::MediaType>()?, id::MediaType::AUDIO);
    ///
    /// let p = obj.property()?;
    /// assert_eq!(p.key::<id::Format>(), id::Format::MEDIA_SUB_TYPE);
    /// assert_eq!(p.value().read::<id::MediaSubType>()?, id::MediaSubType::DSP);
    ///
    /// let p = obj.property()?;
    /// assert_eq!(p.key::<id::Format>(), id::Format::AUDIO_FORMAT);
    /// assert_eq!(p.value().read::<id::AudioFormat>()?, id::AudioFormat::F32P);
    ///
    /// let p = obj.property()?;
    /// assert_eq!(p.key::<id::Format>(), id::Format::AUDIO_CHANNELS);
    /// assert_eq!(p.value().read::<u32>()?, 2);
    ///
    /// let p = obj.property()?;
    /// assert_eq!(p.key::<id::Format>(), id::Format::AUDIO_RATE);
    /// assert_eq!(p.value().read::<u32>()?, 48000);
    /// # Ok::<_, pod::Error>(())
    /// ```
    pub fn embed<T>(self, value: T) -> Result<T::Embed<B>, Error>
    where
        T: Embeddable,
    {
        value.embed_into(self)
    }

    /// Write a sized value into the pod.
    ///
    /// This is a low-level API, consider using [`Builder::write`] instead.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut pod = pod::array();
    /// pod.write(10i32)?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    pub fn write_sized<T>(mut self, value: T) -> Result<(), Error>
    where
        T: SizedWritable,
    {
        self.kind.header(self.buf.borrow_mut())?;
        self.kind.write_sized(value, self.buf)
    }

    /// Write an unsized value into the pod.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut pod = pod::array();
    /// pod.write_unsized(&b"hello world"[..])?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    pub fn write_unsized<T>(mut self, value: &T) -> Result<(), Error>
    where
        T: ?Sized + UnsizedWritable,
    {
        self.kind.header(self.buf.borrow_mut())?;
        self.kind.write_unsized_into(value, self.buf)
    }

    /// Write a `None` value.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut pod = pod::array();
    /// pod.write_none()?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn write_none(mut self) -> Result<(), Error> {
        self.kind.check(Type::NONE, 0)?;
        self.buf.write(&[0, Type::NONE.into_u32()])?;
        Ok(())
    }

    /// Write an array with the given sized type.
    ///
    /// To encode an array with unsized types, use [`Pod::write_unsized_array`]
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
    /// assert!(pod.as_mut().write_array(Type::STRING, |_| Ok(())).is_err());
    ///
    /// let mut pod = pod::array();
    /// let mut array = pod.as_mut().write_array(Type::INT, |array| {
    ///     assert!(array.child().write(42.42f32).is_err());
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
    /// let mut array = pod.as_mut().write_array(Type::INT, |array| {
    ///     array.child().write(1i32)?;
    ///     array.child().write(2i32)?;
    ///     array.child().write(3i32)?;
    ///     Ok(())
    /// })?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn write_array(
        mut self,
        child_type: Type,
        f: impl FnOnce(&mut ArrayBuilder<B, P>) -> Result<(), Error>,
    ) -> Result<(), Error> {
        self.kind.header(self.buf.borrow_mut())?;
        let mut encoder = ArrayBuilder::to_writer(self.buf, self.kind, child_type)?;
        f(&mut encoder)?;
        encoder.close()?;
        Ok(())
    }

    /// Write an array with items of an unsized type.
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
    /// assert!(pod.write_unsized_array(Type::INT, 5, |_| Ok(())).is_err());
    ///
    /// let mut pod = pod::array();
    ///
    /// pod.write_unsized_array(Type::STRING, 4, |array| {
    ///     // Note: strings are null-terminated, so the length is 4.
    ///     array.child().write_unsized("foo")?;
    ///
    ///     assert!(array.child().write(1i32).is_err());
    ///     assert!(array.child().write_unsized("barbaz").is_err());
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
    /// pod.as_mut().write_unsized_array(Type::STRING, 4, |array| {
    ///     // Note: strings are null-terminated, so the length is 4.
    ///     array.child().write_unsized("foo")?;
    ///     array.child().write_unsized("bar")?;
    ///     array.child().write_unsized("baz")?;
    ///     Ok(())
    /// })?;
    ///
    /// let mut array = pod.as_ref().read_array()?;
    /// assert_eq!(array.next().unwrap().read_unsized::<str>()?, "foo");
    /// assert_eq!(array.next().unwrap().read_unsized::<str>()?, "bar");
    /// assert_eq!(array.next().unwrap().read_unsized::<str>()?, "baz");
    /// assert!(array.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn write_unsized_array(
        mut self,
        child_type: Type,
        child_size: usize,
        f: impl FnOnce(&mut ArrayBuilder<B, P>) -> Result<(), Error>,
    ) -> Result<(), Error> {
        self.kind.header(self.buf.borrow_mut())?;
        let mut array =
            ArrayBuilder::to_writer_unsized(self.buf, self.kind, child_size, child_type)?;
        f(&mut array)?;
        array.close()?;
        Ok(())
    }

    /// Write a struct.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = pod::array();
    /// pod.as_mut().write_struct(|st| {
    ///     st.field().write(1i32)?;
    ///     st.field().write(2i32)?;
    ///     st.field().write(3i32)?;
    ///     Ok(())
    /// })?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn write_struct(
        self,
        f: impl FnOnce(&mut StructBuilder<B, P>) -> Result<(), Error>,
    ) -> Result<(), Error> {
        _ = self.embed_struct(f)?;
        Ok(())
    }

    /// Write a struct and return a reference to it for immediate use.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = pod::array();
    /// let st = pod.as_mut().embed_struct(|st| {
    ///     st.field().write(1i32)?;
    ///     st.field().write(2i32)?;
    ///     st.field().write(3i32)?;
    ///     Ok(())
    /// })?;
    ///
    /// let mut st = st.as_ref();
    /// assert_eq!(st.field()?.read_sized::<i32>()?, 1);
    /// assert_eq!(st.field()?.read_sized::<i32>()?, 2);
    /// assert_eq!(st.field()?.read_sized::<i32>()?, 3);
    /// assert!(st.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn embed_struct(
        mut self,
        f: impl FnOnce(&mut StructBuilder<B, P>) -> Result<(), Error>,
    ) -> Result<Struct<impl AsSlice>, Error> {
        self.kind.header(self.buf.borrow_mut())?;
        let mut encoder = StructBuilder::to_writer(self.buf, self.kind)?;
        f(&mut encoder)?;
        let slice = encoder.close()?;
        Ok(Struct::new(slice))
    }

    /// Write an object.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = pod::array();
    /// pod.as_mut().write_object(10, 20, |obj| {
    ///     obj.property(1).write(1i32)?;
    ///     obj.property(2).write(2i32)?;
    ///     obj.property(3).write(3i32)?;
    ///     Ok(())
    /// })?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    ///
    /// Using the return value to immediately read the object back:
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = pod::array();
    ///
    /// let mut obj = pod.as_mut().write_object(10, 20, |obj| {
    ///     obj.property(1).write(2)?;
    ///     obj.property(3).write(4)?;
    ///     obj.property(5).write(6)?;
    ///     Ok(())
    /// })?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn write_object(
        self,
        object_type: impl RawId,
        object_id: impl RawId,
        f: impl FnOnce(&mut ObjectBuilder<B, P>) -> Result<(), Error>,
    ) -> Result<(), Error> {
        _ = self.embed_object(object_type, object_id, f)?;
        Ok(())
    }

    /// Write an object and return a reference to it for immediate use.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = pod::array();
    ///
    /// let obj = pod.as_mut().embed_object(10, 20, |obj| {
    ///     obj.property(1).write(2)?;
    ///     obj.property(3).write(4)?;
    ///     obj.property(5).write(6)?;
    ///     Ok(())
    /// })?;
    ///
    /// let mut obj = obj.as_ref();
    ///
    /// assert_eq!(obj.object_type::<u32>(), 10);
    /// assert_eq!(obj.object_id::<u32>(), 20);
    ///
    /// let mut p = obj.property()?;
    /// assert_eq!(p.key::<u32>(), 1);
    /// assert_eq!(p.flags(), 0);
    /// assert_eq!(p.value().read_sized::<i32>()?, 2);
    ///
    /// let mut p = obj.property()?;
    /// assert_eq!(p.key::<u32>(), 3);
    /// assert_eq!(p.flags(), 0);
    /// assert_eq!(p.value().read_sized::<i32>()?, 4);
    ///
    /// let mut p = obj.property()?;
    /// assert_eq!(p.key::<u32>(), 5);
    /// assert_eq!(p.flags(), 0);
    /// assert_eq!(p.value().read_sized::<i32>()?, 6);
    ///
    /// assert!(obj.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn embed_object(
        mut self,
        object_type: impl RawId,
        object_id: impl RawId,
        f: impl FnOnce(&mut ObjectBuilder<B, P>) -> Result<(), Error>,
    ) -> Result<Object<WriterSlice<B, 16>>, Error> {
        self.kind.header(self.buf.borrow_mut())?;
        let mut encoder = ObjectBuilder::to_writer(
            self.buf,
            self.kind,
            object_type.into_id(),
            object_id.into_id(),
        )?;
        f(&mut encoder)?;

        let as_slice = encoder.close()?;

        Ok(Object::new(
            as_slice,
            object_type.into_id(),
            object_id.into_id(),
        ))
    }

    /// Write a sequence.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = pod::array();
    /// pod.as_mut().write_sequence(|seq| {
    ///     seq.control().write(1i32)?;
    ///     seq.control().write(2i32)?;
    ///     seq.control().write(3i32)?;
    ///     Ok(())
    /// })?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn write_sequence(
        mut self,
        f: impl FnOnce(&mut SequenceBuilder<B, P>) -> Result<(), Error>,
    ) -> Result<(), Error> {
        self.kind.header(self.buf.borrow_mut())?;
        let mut encoder = SequenceBuilder::to_writer(self.buf, self.kind)?;
        f(&mut encoder)?;
        encoder.close()?;
        Ok(())
    }

    /// Write a choice.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{ChoiceType, Pod, Type};
    ///
    /// let mut pod = pod::array();
    /// pod.as_mut().write_choice(ChoiceType::RANGE, Type::INT, |choice| {
    ///     choice.child().write(1i32)?;
    ///     Ok(())
    /// })?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn write_choice(
        mut self,
        choice: ChoiceType,
        child_type: Type,
        f: impl FnOnce(&mut ChoiceBuilder<B, P>) -> Result<(), Error>,
    ) -> Result<(), Error> {
        self.kind.header(self.buf.borrow_mut())?;
        let mut encoder = ChoiceBuilder::to_writer(self.buf, self.kind, choice, child_type)?;
        f(&mut encoder)?;
        encoder.close()?;
        Ok(())
    }

    /// Write a nested pod.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, TypedPod};
    ///
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
    /// let pod = pod.as_ref().into_typed()?.read_pod()?;
    /// let mut st = pod.as_ref().read_struct()?;
    /// assert!(!st.is_empty());
    /// assert_eq!(st.field()?.read_sized::<i32>()?, 1i32);
    /// assert_eq!(st.field()?.read_sized::<i32>()?, 2i32);
    /// assert_eq!(st.field()?.read_sized::<i32>()?, 3i32);
    /// assert!(st.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn write_pod(
        mut self,
        f: impl FnOnce(&mut Builder<B>) -> Result<(), Error>,
    ) -> Result<(), Error> {
        self.kind.header(self.buf.borrow_mut())?;

        // Reserve space for the header of the choice which includes its size
        // that will be determined later.
        let header = self.buf.reserve(&[0, Type::POD.into_u32()])?;

        let mut pod = Builder::new(self.buf);

        f(&mut pod)?;

        let size = pod
            .buf
            .distance_from(&header)
            .wrapping_sub(mem::size_of::<[u32; 2]>());

        self.kind.check(Type::POD, size)?;

        let Ok(size) = u32::try_from(size) else {
            return Err(Error::new(ErrorKind::SizeOverflow));
        };

        pod.buf.write_at(header, &[size, Type::POD.into_u32()])?;
        Ok(())
    }
}

impl<B, P> PodSink for Builder<B, P>
where
    B: Writer,
    P: BuildPod,
{
    type Writer<'this>
        = B::Mut<'this>
    where
        Self: 'this;
    type BuildPod = P;

    /// Get the next pod from the stream.
    #[inline]
    fn next(&mut self) -> Result<Builder<Self::Writer<'_>, Self::BuildPod>, Error> {
        Ok(Builder::with_kind(self.buf.borrow_mut(), self.kind))
    }
}

/// [`UnsizedWritable`] implementation for [`Builder`].
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
impl<B, P> UnsizedWritable for Builder<B, P>
where
    B: AsSlice,
    P: BuildPod,
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

crate::macros::encode_into_unsized!(impl [B, P] Builder<B, P> where B: AsSlice, P: BuildPod);

impl<B, P> Clone for Builder<B, P>
where
    B: Clone,
    P: Copy,
{
    #[inline]
    fn clone(&self) -> Self {
        Builder {
            buf: self.buf.clone(),
            kind: self.kind,
        }
    }
}

impl<B, P> fmt::Debug for Builder<B, P>
where
    B: AsSlice,
    P: BuildPod + ReadPod,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match TypedPod::from_reader(self.buf.as_slice(), self.kind) {
            Ok(pod) => pod.fmt(f),
            Err(e) => e.fmt(f),
        }
    }
}
