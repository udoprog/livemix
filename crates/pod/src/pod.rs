use core::fmt;

use crate::de::{ArrayDecoder, ChoiceDecoder, ObjectDecoder, SequenceDecoder, StructDecoder};
use crate::en::{ArrayEncoder, ChoiceEncoder, ObjectEncoder, SequenceEncoder, StructEncoder};
use crate::error::ErrorKind;
use crate::{
    Array, Choice, Decode, DecodeUnsized, Encode, EncodeUnsized, Error, Reader, Type, TypedPod,
    Visitor, Writer,
};

/// An unlimited pod.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct EnvelopePod;

/// A pod limited for a specific child type and size.
#[derive(Clone, Copy, Debug)]
pub struct ChildPod {
    size: u32,
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

    fn encode<T>(&self, value: T, buf: impl Writer) -> Result<(), Error>
    where
        T: Encode;

    fn encode_unsized<T>(&self, value: &T, buf: impl Writer) -> Result<(), Error>
    where
        T: ?Sized + EncodeUnsized;

    fn check(&self, ty: Type, size: u32) -> Result<(), Error>;
}

impl PodKind for ChildPod {
    const ENVELOPE: bool = false;

    #[inline]
    fn encode<T>(&self, value: T, buf: impl Writer) -> Result<(), Error>
    where
        T: Encode,
    {
        self.check(T::TYPE, value.size())?;
        value.write_content(buf)
    }

    #[inline]
    fn encode_unsized<T>(&self, value: &T, buf: impl Writer) -> Result<(), Error>
    where
        T: ?Sized + EncodeUnsized,
    {
        self.check(T::TYPE, value.size())?;
        value.write_content(buf)
    }

    #[inline]
    fn check(&self, ty: Type, size: u32) -> Result<(), Error> {
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
    fn encode<T>(&self, value: T, buf: impl Writer) -> Result<(), Error>
    where
        T: Encode,
    {
        value.encode(buf)
    }

    #[inline]
    fn encode_unsized<T>(&self, value: &T, buf: impl Writer) -> Result<(), Error>
    where
        T: ?Sized + EncodeUnsized,
    {
        value.encode_unsized(buf)
    }

    #[inline]
    fn check(&self, _: Type, _: u32) -> Result<(), Error> {
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

impl Pod<Array<256>> {
    /// Construct a new [`Pod`] with a 256 word-sized array buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::Pod;
    ///
    /// let mut pod = Pod::array();
    /// pod.as_mut().encode(10i32)?;
    /// assert_eq!(pod.decode::<i32>()?, 10i32);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub const fn array() -> Self {
        Pod {
            buf: Array::with_size(),
            kind: EnvelopePod,
        }
    }
}

impl<const N: usize> Pod<Array<N>> {
    /// Modify the size of the array buffer used by the pod.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::Pod;
    ///
    /// let mut pod = Pod::array().with_size::<16>();
    /// pod.as_mut().encode(10i32)?;
    ///
    /// assert_eq!(pod.decode::<i32>()?, 10i32);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub const fn with_size<const U: usize>(self) -> Pod<Array<U>> {
        Pod {
            buf: Array::with_size(),
            kind: self.kind,
        }
    }
}

impl<B> Pod<B> {
    /// Construct a new [`Pod`] arround the specified buffer `B`.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Array, Pod};
    ///
    /// let mut buf = Array::new();
    /// let mut pod = Pod::new(&mut buf);
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
    pub(crate) const fn new_child(buf: B, size: u32, ty: Type) -> Self {
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
    /// pod.as_mut().encode(10i32)?;
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
    /// pod.as_mut().encode(10i32)?;
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
    B: Writer,
    K: PodKind,
{
    /// Encode a value into the pod.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::Pod;
    ///
    /// let mut pod = Pod::array();
    /// pod.encode(10i32)?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    pub fn encode<T>(self, value: T) -> Result<(), Error>
    where
        T: Encode,
    {
        self.kind.encode(value, self.buf)
    }

    /// Encode an unsized value into the pod.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::Pod;
    ///
    /// let mut pod = Pod::array();
    /// pod.encode_unsized(&b"hello world"[..])?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    pub fn encode_unsized<T>(self, value: &T) -> Result<(), Error>
    where
        T: ?Sized + EncodeUnsized,
    {
        self.kind.encode_unsized(value, self.buf)
    }

    /// Encode a `None` value.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::Pod;
    ///
    /// let mut pod = Pod::array();
    /// pod.encode_none()?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn encode_none(mut self) -> Result<(), Error> {
        self.kind.check(Type::NONE, 0)?;

        if K::ENVELOPE {
            self.buf.write([0, Type::NONE.into_u32()])?;
        }

        Ok(())
    }

    /// Encode an array with the given sized type.
    ///
    /// To encode an array with unsized types, use [`Pod::encode_unsized_array`]
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
    /// assert!(pod.as_mut().encode_array(Type::STRING).is_err());
    ///
    /// let mut pod = Pod::array();
    /// let mut array = pod.as_mut().encode_array(Type::INT)?;
    /// assert!(array.push()?.encode(42.42f32).is_err());
    /// # Ok::<_, pod::Error>(())
    /// ```
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = Pod::array();
    /// let mut array = pod.as_mut().encode_array(Type::INT)?;
    /// array.push()?.encode(1i32)?;
    /// array.push()?.encode(2i32)?;
    /// array.push()?.encode(3i32)?;
    /// array.close()?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn encode_array(self, child_type: Type) -> Result<ArrayEncoder<B, K>, Error> {
        ArrayEncoder::to_writer(self.buf, self.kind, child_type)
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
    /// assert!(pod.encode_unsized_array(Type::INT, 5).is_err());
    ///
    /// let mut pod = Pod::array();
    /// let mut array = pod.encode_unsized_array(Type::STRING, 4)?;
    ///
    /// // Note: strings are null-terminated, so the length is 4.
    /// array.push()?.encode_unsized("foo")?;
    ///
    /// assert!(array.push()?.encode(1i32).is_err());
    /// assert!(array.push()?.encode_unsized("barbaz").is_err());
    /// # Ok::<_, pod::Error>(())
    /// ```
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = Pod::array();
    /// let mut array = pod.as_mut().encode_unsized_array(Type::STRING, 4)?;
    /// // Note: strings are null-terminated, so the length is 4.
    /// array.push()?.encode_unsized("foo")?;
    /// array.push()?.encode_unsized("bar")?;
    /// array.push()?.encode_unsized("baz")?;
    /// array.close()?;
    ///
    /// let buf = pod.into_buf();
    /// assert_eq!(buf.as_slice().len(), 5);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn encode_unsized_array(
        self,
        child_type: Type,
        child_size: u32,
    ) -> Result<ArrayEncoder<B, K>, Error> {
        ArrayEncoder::to_writer_unsized(self.buf, self.kind, child_size, child_type)
    }

    /// Encode a struct.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = Pod::array();
    /// let mut st = pod.as_mut().encode_struct()?;
    /// st.field()?.encode(1i32)?;
    /// st.field()?.encode(2i32)?;
    /// st.field()?.encode(3i32)?;
    /// st.close()?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn encode_struct(self) -> Result<StructEncoder<B, K>, Error> {
        StructEncoder::to_writer(self.buf, self.kind)
    }

    /// Encode an object.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = Pod::array();
    /// let mut obj = pod.as_mut().encode_object(10, 20)?;
    /// obj.property(1, 0)?.encode(1i32)?;
    /// obj.property(2, 0)?.encode(2i32)?;
    /// obj.property(3, 0)?.encode(3i32)?;
    /// obj.close()?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn encode_object(
        self,
        object_type: u32,
        object_id: u32,
    ) -> Result<ObjectEncoder<B, K>, Error> {
        ObjectEncoder::to_writer(self.buf, self.kind, object_type, object_id)
    }

    /// Encode a sequence.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = Pod::array();
    /// let mut seq = pod.as_mut().encode_sequence()?;
    /// seq.control(1, 0)?.encode(1i32)?;
    /// seq.control(2, 0)?.encode(2i32)?;
    /// seq.control(3, 0)?.encode(3i32)?;
    /// seq.close()?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn encode_sequence(self) -> Result<SequenceEncoder<B, K>, Error> {
        SequenceEncoder::to_writer(self.buf, self.kind)
    }

    /// Encode a choice.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Choice, Pod, Type};
    ///
    /// let mut pod = Pod::array();
    /// let mut choice = pod.as_mut().encode_choice(Choice::RANGE, Type::INT)?;
    ///
    /// choice.entry()?.encode(1i32)?;
    ///
    /// choice.close()?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn encode_choice(
        self,
        choice: Choice,
        child_type: Type,
    ) -> Result<ChoiceEncoder<B, K>, Error> {
        ChoiceEncoder::to_writer(self.buf, self.kind, choice, child_type)
    }

    /// Borrow the current pod mutably, allowing multiple elements to be encoded
    /// into it or the pod immediately re-used.
    #[inline]
    pub fn as_mut(&mut self) -> Pod<B::Mut<'_>> {
        Pod::new(self.buf.borrow_mut())
    }
}

impl<'de, B> Pod<B>
where
    B: Reader<'de>,
{
    /// Skip a value in the pod.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    /// let mut pod = Pod::array();
    ///
    /// let mut array = pod.as_mut().encode_array(Type::INT)?;
    /// array.push()?.encode(10i32)?;
    /// array.push()?.encode(20i32)?;
    /// array.close()?;
    ///
    /// let pod = pod.typed()?;
    /// let mut array = pod.decode_array()?;
    /// assert!(!array.is_empty());
    /// array.item()?.skip()?;
    /// assert_eq!(array.item()?.decode::<i32>()?, 20i32);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn skip(self) -> Result<(), Error> {
        self.into_typed()?.skip()
    }

    /// Encode a value into the pod.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::Pod;
    ///
    /// let mut pod = Pod::array();
    /// pod.as_mut().encode(10i32)?;
    ///
    /// assert_eq!(pod.decode::<i32>()?, 10i32);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn decode<T>(self) -> Result<T, Error>
    where
        T: Decode<'de>,
    {
        self.into_typed()?.decode::<T>()
    }

    /// Decode an unsized value into the pod.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::Pod;
    ///
    /// let mut pod = Pod::array();
    /// pod.as_mut().encode_unsized(&b"hello world"[..])?;
    ///
    /// let pod = pod.typed()?;
    /// assert_eq!(pod.decode_borrowed::<[u8]>()?, b"hello world");
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn decode_unsized<T, V>(self, visitor: V) -> Result<V::Ok, Error>
    where
        T: ?Sized + DecodeUnsized<'de>,
        V: Visitor<'de, T>,
    {
        self.into_typed()?.decode_unsized(visitor)
    }

    /// Decode an unsized value into the pod.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::Pod;
    ///
    /// let mut pod = Pod::array();
    /// pod.as_mut().encode_unsized(&b"hello world"[..])?;
    ///
    /// let pod = pod.typed()?;
    /// assert_eq!(pod.decode_borrowed::<[u8]>()?, b"hello world");
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn decode_borrowed<T>(self) -> Result<&'de T, Error>
    where
        T: ?Sized + DecodeUnsized<'de>,
    {
        self.into_typed()?.decode_borrowed()
    }

    /// Decode an optional value.
    ///
    /// This returns `None` if the encoded value is `None`, otherwise a pod
    /// for the value is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::Pod;
    ///
    /// let mut pod = Pod::array();
    /// pod.as_mut().encode_none()?;
    ///
    /// assert!(pod.decode_option()?.is_none());
    ///
    /// let mut pod = Pod::array();
    /// pod.as_mut().encode(true)?;
    ///
    /// let Some(mut pod) = pod.decode_option()? else {
    ///     panic!("expected some value");
    /// };
    ///
    /// assert!(pod.decode::<bool>()?);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn decode_option(self) -> Result<Option<TypedPod<B>>, Error> {
        self.into_typed()?.decode_option()
    }

    /// Decode an array.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = Pod::array();
    /// let mut array = pod.as_mut().encode_array(Type::INT)?;
    /// array.push()?.encode(1i32)?;
    /// array.push()?.encode(2i32)?;
    /// array.push()?.encode(3i32)?;
    /// array.close()?;
    ///
    /// let mut array = pod.decode_array()?;
    ///
    /// assert!(!array.is_empty());
    /// assert_eq!(array.len(), 3);
    ///
    /// assert_eq!(array.item()?.decode::<i32>()?, 1i32);
    /// assert_eq!(array.item()?.decode::<i32>()?, 2i32);
    /// assert_eq!(array.item()?.decode::<i32>()?, 3i32);
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
    /// pod.as_mut().encode_array(Type::INT)?;
    ///
    /// let mut array = pod.decode_array()?;
    ///
    /// assert!(array.is_empty());
    /// assert_eq!(array.len(), 0);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn decode_array(self) -> Result<ArrayDecoder<B>, Error> {
        self.into_typed()?.decode_array()
    }

    /// Decode a struct.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, TypedPod};
    ///
    /// let mut pod = Pod::array();
    /// let mut st = pod.as_mut().encode_struct()?;
    /// st.field()?.encode(1i32)?;
    /// st.field()?.encode(2i32)?;
    /// st.field()?.encode(3i32)?;
    /// st.close()?;
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
    ///
    /// Decoding an empty struct:
    ///
    /// ```
    /// use pod::{Pod, TypedPod};
    ///
    /// let mut pod = Pod::array();
    /// pod.as_mut().encode_struct()?;
    ///
    /// let st = pod.decode_struct()?;
    /// assert!(st.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn decode_struct(self) -> Result<StructDecoder<B>, Error> {
        self.into_typed()?.decode_struct()
    }

    /// Decode an object.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = Pod::array();
    /// let mut obj = pod.as_mut().encode_object(10, 20)?;
    /// obj.property(1, 10)?.encode(1i32)?;
    /// obj.property(2, 20)?.encode(2i32)?;
    /// obj.property(3, 30)?.encode(3i32)?;
    /// obj.close()?;
    ///
    /// let mut obj = pod.decode_object()?;
    ///
    /// assert!(!obj.is_empty());
    ///
    /// let p = obj.property()?;
    /// assert_eq!(p.key(), 1);
    /// assert_eq!(p.flags(), 10);
    /// assert_eq!(p.value().decode::<i32>()?, 1);
    ///
    /// let p = obj.property()?;
    /// assert_eq!(p.key(), 2);
    /// assert_eq!(p.flags(), 20);
    /// assert_eq!(p.value().decode::<i32>()?, 2);
    ///
    /// let p = obj.property()?;
    /// assert_eq!(p.key(), 3);
    /// assert_eq!(p.flags(), 30);
    /// assert_eq!(p.value().decode::<i32>()?, 3);
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
    /// pod.as_mut().encode_object(10, 20)?;
    ///
    /// let obj = pod.decode_object()?;
    /// assert!(obj.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn decode_object(self) -> Result<ObjectDecoder<B>, Error> {
        self.into_typed()?.decode_object()
    }

    /// Decode a sequence.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = Pod::array();
    /// let mut seq = pod.as_mut().encode_sequence()?;
    /// seq.control(1, 10)?.encode(1i32)?;
    /// seq.control(2, 20)?.encode(2i32)?;
    /// seq.control(3, 30)?.encode(3i32)?;
    /// seq.close()?;
    ///
    /// let mut seq = pod.decode_sequence()?;
    ///
    /// assert!(!seq.is_empty());
    ///
    /// let c = seq.control()?;
    /// assert_eq!(c.offset(), 1);
    /// assert_eq!(c.ty(), 10);
    /// assert_eq!(c.value().decode::<i32>()?, 1);
    ///
    /// let c = seq.control()?;
    /// assert_eq!(c.offset(), 2);
    /// assert_eq!(c.ty(), 20);
    /// assert_eq!(c.value().decode::<i32>()?, 2);
    ///
    /// let c = seq.control()?;
    /// assert_eq!(c.offset(), 3);
    /// assert_eq!(c.ty(), 30);
    /// assert_eq!(c.value().decode::<i32>()?, 3);
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
    /// pod.as_mut().encode_sequence()?;
    ///
    /// let seq = pod.decode_sequence()?;
    /// assert!(seq.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn decode_sequence(self) -> Result<SequenceDecoder<B>, Error> {
        self.into_typed()?.decode_sequence()
    }

    /// Decode a choice.
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
    /// assert!(!choice.is_empty());
    ///
    /// assert_eq!(choice.entry()?.decode::<i32>()?, 10);
    /// assert_eq!(choice.entry()?.decode::<i32>()?, 0);
    /// assert_eq!(choice.entry()?.decode::<i32>()?, 30);
    ///
    /// assert!(choice.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    ///
    /// Encoding an empty choice:
    ///
    /// ```
    /// use pod::{Choice, Pod, Type};
    ///
    /// let mut pod = Pod::array();
    /// let mut choice = pod.as_mut().encode_choice(Choice::RANGE, Type::INT)?;
    ///
    /// let mut choice = pod.decode_choice()?;
    ///
    /// assert!(choice.is_empty());
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn decode_choice(self) -> Result<ChoiceDecoder<B>, Error> {
        self.into_typed()?.decode_choice()
    }

    /// Convert the [`Pod`] into a one borrowing from but without modifying the
    /// current buffer.
    #[inline]
    pub fn as_ref(&self) -> Pod<B::Clone<'_>> {
        Pod::new(self.buf.clone_reader())
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

    /// Convert the [`Pod`] into a [`TypedPod`] borrowing from but without
    /// modifying the current buffer.
    ///
    /// A typed pod knows about the size and type of the data it contains,
    /// allowing it to be inspected through the relevant [`TypedPod::size`] and
    /// [`TypedPod::ty`] APIs.
    ///
    /// # Errors
    ///
    /// This errors if the pod does not wrap a buffer containing a valid pod.
    #[inline]
    pub fn typed(&self) -> Result<TypedPod<B::Clone<'_>>, Error> {
        TypedPod::from_reader(self.buf.clone_reader())
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

impl<'de, B> fmt::Debug for Pod<B>
where
    B: Reader<'de>,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let pod = TypedPod::from_reader(self.buf.clone_reader()).map_err(|_| fmt::Error)?;
        pod.debug_fmt_with_type(f)
    }
}
