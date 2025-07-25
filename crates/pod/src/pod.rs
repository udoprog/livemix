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
pub struct Unlimited;

/// A pod limited for a specific child type and size.
#[derive(Clone, Copy, Debug)]
pub struct ChildLimit {
    size: u32,
    ty: Type,
}

impl PodLimit for ChildLimit {
    const ENVELOPE: bool = false;

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

    #[inline]
    fn check_unsized(&self, _: Type) -> Result<(), Error> {
        Err(Error::new(ErrorKind::ChildUnsizedMismatch {
            expected: self.size,
        }))
    }
}

pub trait PodLimit: Clone {
    const ENVELOPE: bool;

    /// Check if the type [`Type`] of the given size is allowed.
    fn check(&self, ty: Type, size: u32) -> Result<(), Error>;

    /// Check a type of unknown size.
    fn check_unsized(&self, ty: Type) -> Result<(), Error>;
}

impl PodLimit for Unlimited {
    const ENVELOPE: bool = true;

    #[inline]
    fn check(&self, _: Type, _: u32) -> Result<(), Error> {
        Ok(())
    }

    #[inline]
    fn check_unsized(&self, _: Type) -> Result<(), Error> {
        Ok(())
    }
}

/// A POD (Plain Old Data) handler.
///
/// This is a wrapper that can be used for encoding and decoding data.
pub struct Pod<B, L = Unlimited>
where
    L: PodLimit,
{
    buf: B,
    limit: L,
}

impl<B> Clone for Pod<B>
where
    B: Clone,
{
    #[inline]
    fn clone(&self) -> Self {
        Pod {
            buf: self.buf.clone(),
            limit: self.limit.clone(),
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
    /// pod.encode(10i32)?;
    ///
    /// assert_eq!(pod.decode::<i32>()?, 10i32);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub const fn array() -> Self {
        Pod {
            buf: Array::with_size(),
            limit: Unlimited,
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
    /// pod.encode(10i32)?;
    ///
    /// assert_eq!(pod.decode::<i32>()?, 10i32);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub const fn with_size<const U: usize>(self) -> Pod<Array<U>> {
        Pod {
            buf: Array::with_size(),
            limit: self.limit,
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
            limit: Unlimited,
        }
    }

    /// Coerce into the underlying buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::Pod;
    ///
    /// let mut pod = Pod::array();
    /// pod.encode(10i32)?;
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

impl<B> Pod<B, ChildLimit> {
    /// Construct a new pod with a child limit.
    pub(crate) const fn new_child(buf: B, size: u32, ty: Type) -> Self {
        Pod {
            buf,
            limit: ChildLimit { size, ty },
        }
    }
}

impl<B, I> Pod<B, I>
where
    B: Writer,
    I: PodLimit,
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
    pub fn encode<T>(&mut self, value: T) -> Result<(), Error>
    where
        T: Encode,
    {
        self.limit.check(T::TYPE, value.size())?;

        if I::ENVELOPE {
            value.encode(self.buf.borrow_mut())
        } else {
            value.write_content(self.buf.borrow_mut())
        }
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
    pub fn encode_unsized<T>(&mut self, value: &T) -> Result<(), Error>
    where
        T: ?Sized + EncodeUnsized,
    {
        self.limit.check(T::TYPE, value.size())?;

        if I::ENVELOPE {
            value.encode_unsized(self.buf.borrow_mut())
        } else {
            value.write_content(self.buf.borrow_mut())
        }
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
    pub fn encode_none(&mut self) -> Result<(), Error> {
        self.limit.check(Type::NONE, 0)?;

        if I::ENVELOPE {
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
    /// assert!(pod.encode_array(Type::STRING).is_err());
    ///
    /// let mut pod = Pod::array();
    /// let mut array = pod.encode_array(Type::INT)?;
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
    /// let mut array = pod.encode_array(Type::INT)?;
    ///
    /// array.push()?.encode(1i32)?;
    /// array.push()?.encode(2i32)?;
    /// array.push()?.encode(3i32)?;
    ///
    /// array.close()?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn encode_array(&mut self, child_type: Type) -> Result<ArrayEncoder<B::Mut<'_>>, Error> {
        self.limit.check_unsized(Type::ARRAY)?;
        ArrayEncoder::to_writer(self.buf.borrow_mut(), child_type)
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
    /// let mut array = pod.encode_unsized_array(Type::STRING, 4)?;
    ///
    /// // Note: strings are null-terminated, so the length is 4.
    /// array.push()?.encode_unsized("foo")?;
    /// array.push()?.encode_unsized("bar")?;
    /// array.push()?.encode_unsized("baz")?;
    ///
    /// array.close()?;
    ///
    /// let buf = pod.into_buf();
    /// assert_eq!(buf.as_slice().len(), 5);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn encode_unsized_array(
        &mut self,
        child_type: Type,
        len: u32,
    ) -> Result<ArrayEncoder<B::Mut<'_>>, Error> {
        self.limit.check_unsized(Type::ARRAY)?;
        ArrayEncoder::to_writer_unsized(self.buf.borrow_mut(), len, child_type)
    }

    /// Encode a struct.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = Pod::array();
    /// let mut st = pod.encode_struct()?;
    ///
    /// st.field()?.encode(1i32)?;
    /// st.field()?.encode(2i32)?;
    /// st.field()?.encode(3i32)?;
    ///
    /// st.close()?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn encode_struct(&mut self) -> Result<StructEncoder<B::Mut<'_>>, Error> {
        self.limit.check_unsized(Type::STRUCT)?;
        StructEncoder::to_writer(self.buf.borrow_mut())
    }

    /// Encode an object.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = Pod::array();
    /// let mut obj = pod.encode_object(10, 20)?;
    ///
    /// obj.property(1, 0)?.encode(1i32)?;
    /// obj.property(2, 0)?.encode(2i32)?;
    /// obj.property(3, 0)?.encode(3i32)?;
    ///
    /// obj.close()?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn encode_object(
        &mut self,
        object_type: u32,
        object_id: u32,
    ) -> Result<ObjectEncoder<B::Mut<'_>>, Error> {
        self.limit.check_unsized(Type::OBJECT)?;
        ObjectEncoder::to_writer(self.buf.borrow_mut(), object_type, object_id)
    }

    /// Encode a sequence.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = Pod::array();
    /// let mut seq = pod.encode_sequence()?;
    ///
    /// seq.control(1, 0)?.encode(1i32)?;
    /// seq.control(2, 0)?.encode(2i32)?;
    /// seq.control(3, 0)?.encode(3i32)?;
    ///
    /// seq.close()?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn encode_sequence(&mut self) -> Result<SequenceEncoder<B::Mut<'_>>, Error> {
        self.limit.check_unsized(Type::SEQUENCE)?;
        SequenceEncoder::to_writer(self.buf.borrow_mut())
    }

    /// Encode a choice.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Choice, Pod, Type};
    ///
    /// let mut pod = Pod::array();
    /// let mut choice = pod.encode_choice(Choice::RANGE, Type::INT)?;
    ///
    /// choice.entry()?.encode(1i32)?;
    ///
    /// choice.close()?;
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn encode_choice(
        &mut self,
        choice: Choice,
        child_type: Type,
    ) -> Result<ChoiceEncoder<B::Mut<'_>>, Error> {
        self.limit.check_unsized(Type::SEQUENCE)?;
        ChoiceEncoder::to_writer(self.buf.borrow_mut(), choice, child_type)
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
    /// let mut array = pod.encode_array(Type::INT)?;
    ///
    /// array.push()?.encode(10i32)?;
    /// array.push()?.encode(20i32)?;
    ///
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
    /// pod.encode(10i32)?;
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
    /// pod.encode_unsized(&b"hello world"[..])?;
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
    ///
    /// pod.encode_unsized(&b"hello world"[..])?;
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
    /// pod.encode_none()?;
    ///
    /// assert!(pod.decode_option()?.is_none());
    ///
    /// let mut pod = Pod::array();
    /// pod.encode(true)?;
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
    /// let mut array = pod.encode_array(Type::INT)?;
    ///
    /// array.push()?.encode(1i32)?;
    /// array.push()?.encode(2i32)?;
    /// array.push()?.encode(3i32)?;
    ///
    /// array.close()?;
    ///
    /// let mut array = pod.decode_array()?;
    ///
    /// assert!(!array.is_empty());
    /// assert_eq!(array.len(), 3);
    ///
    /// assert_eq!(array.decode::<i32>()?, 1i32);
    /// assert_eq!(array.decode::<i32>()?, 2i32);
    /// assert_eq!(array.decode::<i32>()?, 3i32);
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
    /// let mut st = pod.encode_struct()?;
    ///
    /// st.field()?.encode(1i32)?;
    /// st.field()?.encode(2i32)?;
    /// st.field()?.encode(3i32)?;
    ///
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
    /// let mut obj = pod.encode_object(10, 20)?;
    ///
    /// obj.property(1, 10)?.encode(1i32)?;
    /// obj.property(2, 20)?.encode(2i32)?;
    /// obj.property(3, 30)?.encode(3i32)?;
    ///
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
    /// let mut seq = pod.encode_sequence()?;
    ///
    /// seq.control(1, 10)?.encode(1i32)?;
    /// seq.control(2, 20)?.encode(2i32)?;
    /// seq.control(3, 30)?.encode(3i32)?;
    ///
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
    /// let mut choice = pod.encode_choice(Choice::RANGE, Type::INT)?;
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
    #[inline]
    pub fn decode_choice(self) -> Result<ChoiceDecoder<B>, Error> {
        self.into_typed()?.decode_choice()
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
