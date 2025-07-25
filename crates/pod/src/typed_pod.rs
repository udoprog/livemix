use core::ffi::CStr;
use core::fmt;

use crate::bstr::BStr;
use crate::de::{ArrayDecoder, ChoiceDecoder, ObjectDecoder, SequenceDecoder, StructDecoder};
use crate::error::ErrorKind;
use crate::{
    Bitmap, Decode, DecodeUnsized, Error, Fd, Fraction, Id, Pointer, Reader, Rectangle, Type,
    Visitor, WORD_SIZE,
};

/// A POD (Plain Old Data) handler.
///
/// This is a wrapper that can be used for encoding and decoding data.
pub struct TypedPod<B> {
    size: u32,
    ty: Type,
    buf: B,
}

impl<B> TypedPod<B> {
    /// Construct a new [`TypedPod`] arround the specified buffer `B`.
    #[inline]
    pub(crate) const fn new(size: u32, ty: Type, buf: B) -> Self {
        TypedPod { size, ty, buf }
    }

    /// Get the type of the pod.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, Type};
    ///
    /// let mut pod = Pod::array();
    /// pod.as_mut().encode(10i32)?;
    ///
    /// let pod = pod.typed()?;
    /// assert_eq!(pod.ty(), Type::INT);
    /// assert_eq!(pod.size(), 4);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub const fn ty(&self) -> Type {
        self.ty
    }

    /// Get the size of the pod.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, TypedPod, Type};
    ///
    /// let mut pod = Pod::array();
    /// pod.as_mut().encode(10i32)?;
    ///
    /// let pod = pod.typed()?;
    /// assert_eq!(pod.ty(), Type::INT);
    /// assert_eq!(pod.size(), 4);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub const fn size(&self) -> u32 {
        self.size
    }

    /// Get the size of the padded pod including the header.
    #[inline]
    pub(crate) fn size_with_header(&self) -> Option<u32> {
        self.size.next_multiple_of(WORD_SIZE).checked_add(WORD_SIZE)
    }
}

impl<'de, B> TypedPod<B>
where
    B: Reader<'de, u64>,
{
    /// Construct a new [`TypedPod`] by reading and advancing the given buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, TypedPod};
    ///
    /// let mut pod = Pod::array();
    /// pod.as_mut().encode(10i32)?;
    ///
    /// let pod = pod.typed()?;
    /// assert_eq!(pod.decode::<i32>()?, 10i32);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn from_reader(mut buf: B) -> Result<Self, Error> {
        let (size, ty) = buf.header()?;
        Ok(TypedPod { size, ty, buf })
    }

    /// Skip a value in the pod.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, TypedPod, Type};
    ///
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
    pub fn skip(mut self) -> Result<(), Error> {
        self.buf.skip(self.size)?;
        Ok(())
    }

    /// Encode a value into the pod.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, TypedPod};
    ///
    /// let mut pod = Pod::array();
    /// pod.as_mut().encode(10i32)?;
    ///
    /// let pod = pod.typed()?;
    /// assert_eq!(pod.decode::<i32>()?, 10i32);
    /// # Ok::<_, pod::Error>(())
    /// ```
    #[inline]
    pub fn decode<T>(self) -> Result<T, Error>
    where
        T: Decode<'de>,
    {
        if T::TYPE != self.ty {
            return Err(Error::new(ErrorKind::Expected {
                expected: T::TYPE,
                actual: self.ty,
            }));
        }

        T::read_content(self.buf, self.size)
    }

    /// Decode an unsized value into the pod.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, TypedPod};
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
        if T::TYPE != self.ty {
            return Err(Error::new(ErrorKind::Expected {
                expected: self.ty,
                actual: T::TYPE,
            }));
        }

        T::read_content(self.buf, visitor, self.size)
    }

    /// Decode an unsized value into the pod.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, TypedPod};
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
        if T::TYPE != self.ty {
            return Err(Error::new(ErrorKind::Expected {
                expected: T::TYPE,
                actual: self.ty,
            }));
        }

        T::read_borrowed(self.buf, self.size)
    }

    /// Decode an optional value.
    ///
    /// This returns `None` if the encoded value is `None`, otherwise a pod
    /// for the value is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, TypedPod};
    ///
    /// let mut pod = Pod::array();
    /// pod.as_mut().encode_none()?;
    ///
    /// let pod = pod.typed()?;
    /// assert!(pod.decode_option()?.is_none());
    ///
    /// let mut pod = Pod::array();
    /// pod.as_mut().encode(true)?;
    ///
    /// let pod = pod.typed()?;
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
        match self.ty {
            Type::NONE => Ok(None),
            _ => Ok(Some(TypedPod::new(self.size, self.ty, self.buf))),
        }
    }

    /// Decode an array.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, TypedPod, Type};
    ///
    /// let mut pod = Pod::array();
    /// let mut array = pod.as_mut().encode_array(Type::INT)?;
    /// array.push()?.encode(1i32)?;
    /// array.push()?.encode(2i32)?;
    /// array.push()?.encode(3i32)?;
    /// array.close()?;
    ///
    /// let pod = pod.typed()?;
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
    #[inline]
    pub fn decode_array(self) -> Result<ArrayDecoder<B>, Error> {
        match self.ty {
            Type::ARRAY => ArrayDecoder::from_reader(self.buf, self.size),
            _ => Err(Error::new(ErrorKind::Expected {
                expected: Type::ARRAY,
                actual: self.ty,
            })),
        }
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
    /// let pod = pod.typed()?;
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
        match self.ty {
            Type::STRUCT => Ok(StructDecoder::new(self.buf, self.size)),
            _ => Err(Error::new(ErrorKind::Expected {
                expected: Type::STRUCT,
                actual: self.ty,
            })),
        }
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
    #[inline]
    pub fn decode_object(self) -> Result<ObjectDecoder<B>, Error> {
        match self.ty {
            Type::OBJECT => ObjectDecoder::from_reader(self.buf, self.size),
            _ => Err(Error::new(ErrorKind::Expected {
                expected: Type::OBJECT,
                actual: self.ty,
            })),
        }
    }

    /// Decode a sequence.
    ///
    /// # Examples
    ///
    /// ```
    /// use pod::{Pod, TypedPod, Type};
    ///
    /// let mut pod = Pod::array();
    /// let mut seq = pod.as_mut().encode_sequence()?;
    /// seq.control(1, 10)?.encode(1i32)?;
    /// seq.control(2, 20)?.encode(2i32)?;
    /// seq.control(3, 30)?.encode(3i32)?;
    /// seq.close()?;
    ///
    /// let mut pod = pod.typed()?;
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
        match self.ty {
            Type::SEQUENCE => SequenceDecoder::from_reader(self.buf, self.size),
            _ => Err(Error::new(ErrorKind::Expected {
                expected: Type::SEQUENCE,
                actual: self.ty,
            })),
        }
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
    #[inline]
    pub fn decode_choice(self) -> Result<ChoiceDecoder<B>, Error> {
        match self.ty {
            Type::CHOICE => ChoiceDecoder::from_reader(self.buf, self.size),
            _ => Err(Error::new(ErrorKind::Expected {
                expected: Type::CHOICE,
                actual: self.ty,
            })),
        }
    }

    pub(crate) fn debug_fmt_with_type(self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let container = matches!(
            self.ty,
            Type::ARRAY | Type::STRUCT | Type::OBJECT | Type::SEQUENCE | Type::CHOICE
        );
        write!(f, "{}", self.ty)?;

        if !container {
            write!(f, "(")?;
        }

        self.debug_fmt(f)?;

        if !container {
            write!(f, ")")?;
        }

        Ok(())
    }

    fn debug_fmt(self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        macro_rules! tri {
            ($expr:expr) => {
                match $expr {
                    Ok(value) => value,
                    Err(e) => return write!(f, "{{{e}}}"),
                }
            };
        }

        macro_rules! decode {
            ($ty:ty, $pat:pat => $expr:expr) => {
                match self.decode::<$ty>() {
                    Ok($pat) => $expr,
                    Err(e) => write!(f, "{{{e}}}"),
                }
            };
        }

        macro_rules! decode_unsized {
            ($ty:ty, $pat:pat => $expr:expr) => {{
                let mut outer = Ok(());

                let result = self.decode_unsized(FunctionVisitor(|$pat: &$ty| {
                    outer = $expr;
                }));

                if let Err(e) = result {
                    return write!(f, "{{{e}}}");
                }

                outer
            }};
        }

        match self.ty {
            Type::NONE => f.write_str("None"),
            Type::BOOL => {
                decode!(bool, value => write!(f, "{value:?}"))
            }
            Type::ID => {
                decode!(Id<u32>, value => write!(f, "{value:?}"))
            }
            Type::INT => {
                decode!(i32, value => write!(f, "{value:?}"))
            }
            Type::LONG => {
                decode!(i64, value => write!(f, "{value:?}"))
            }
            Type::FLOAT => {
                decode!(f32, value => write!(f, "{value:?}"))
            }
            Type::DOUBLE => {
                decode!(f64, value => write!(f, "{value:?}"))
            }
            Type::STRING => {
                decode_unsized!(CStr, value => write!(f, "{value:?}"))
            }
            Type::BYTES => {
                decode_unsized!([u8], value => write!(f, "{:?}", BStr::new(value)))
            }
            Type::RECTANGLE => {
                decode!(Rectangle, value => write!(
                    f,
                    "{{width: {:?}, height: {:?}}}",
                    value.width, value.height
                ))
            }
            Type::FRACTION => {
                decode!(Fraction, value => write!(f, "{{num: {:?}, denom: {:?}}}", value.num, value.denom))
            }
            Type::BITMAP => {
                decode_unsized!(Bitmap, value => write!(f, "{:?}", BStr::new(value.as_bytes())))
            }
            Type::ARRAY => {
                let mut array = tri!(self.decode_array());
                write!(f, "[{:?}](", array.child_type())?;

                while !array.is_empty() {
                    match array.item() {
                        Ok(item) => item.debug_fmt(f)?,
                        Err(e) => write!(f, "{{{e}}}")?,
                    }

                    if !array.is_empty() {
                        write!(f, ", ")?;
                    }
                }

                write!(f, ")")?;
                Ok(())
            }
            Type::STRUCT => {
                let mut st = tri!(self.decode_struct());
                write!(f, "{{")?;

                while !st.is_empty() {
                    match st.field() {
                        Ok(pod) => {
                            write!(f, "{:?}: ", pod.ty())?;
                            pod.debug_fmt(f)?;
                        }
                        Err(e) => {
                            write!(f, "?: {{{e}}}")?;
                        }
                    }

                    if !st.is_empty() {
                        write!(f, ", ")?;
                    }
                }

                write!(f, "}}")?;
                Ok(())
            }
            Type::OBJECT => {
                let mut st = tri!(self.decode_object());
                write!(f, "[{}, {}]{{", st.object_type(), st.object_id())?;

                while !st.is_empty() {
                    match st.property() {
                        Ok(prop) => {
                            if prop.flags() != 0 {
                                write!(
                                    f,
                                    "{{key: {}, flags: 0b{:b}}}: ",
                                    prop.key(),
                                    prop.flags()
                                )?;
                            } else {
                                write!(f, "{:?}: ", prop.key())?;
                            }

                            prop.value().debug_fmt_with_type(f)?;
                        }
                        Err(e) => {
                            write!(f, "?: {{{e}}}")?;
                        }
                    }

                    if !st.is_empty() {
                        write!(f, ", ")?;
                    }
                }

                write!(f, "}}")?;
                Ok(())
            }
            Type::SEQUENCE => {
                let mut st = tri!(self.decode_sequence());
                write!(f, "[{}, {}]{{", st.unit(), st.pad())?;

                while !st.is_empty() {
                    match st.control() {
                        Ok(c) => {
                            write!(f, "{{offset: {:?}, type: {:?}}}: ", c.offset(), c.ty())?;
                            c.value().debug_fmt_with_type(f)?;
                        }
                        Err(e) => {
                            write!(f, "?: {{{e}}}")?;
                        }
                    }

                    if !st.is_empty() {
                        write!(f, ", ")?;
                    }
                }

                write!(f, "}}")?;
                Ok(())
            }
            Type::POINTER => {
                decode!(Pointer, p => {
                    if p.ty() != 0 {
                        write!(f, "{{pointer: {:?}, type: {:?}}}", p.pointer(), p.ty())
                    } else {
                        write!(f, "{:?}", p.pointer())
                    }
                })
            }
            Type::FD => {
                decode!(Fd, fd => write!(f, "{:?}", fd.fd()))
            }
            Type::CHOICE => {
                let mut choice = tri!(self.decode_choice());
                write!(f, "[{:?}, {:?}](", choice.ty(), choice.child_type())?;

                while !choice.is_empty() {
                    match choice.entry() {
                        Ok(e) => e.debug_fmt(f)?,
                        Err(e) => write!(f, "{{{e}}}")?,
                    }

                    if !choice.is_empty() {
                        write!(f, ", ")?;
                    }
                }

                write!(f, ")")?;
                Ok(())
            }
            ty => {
                if let Err(e) = self.skip() {
                    write!(f, "{e}")
                } else {
                    write!(f, "{{{ty:?}}}")
                }
            }
        }
    }

    #[inline]
    fn typed(&self) -> TypedPod<B::Clone<'_>> {
        TypedPod::new(self.size, self.ty, self.buf.clone_reader())
    }
}

impl<B> Clone for TypedPod<B>
where
    B: Clone,
{
    #[inline]
    fn clone(&self) -> Self {
        TypedPod {
            size: self.size,
            ty: self.ty,
            buf: self.buf.clone(),
        }
    }
}

impl<'de, B> fmt::Debug for TypedPod<B>
where
    B: Reader<'de, u64>,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.typed().debug_fmt_with_type(f)
    }
}

struct FunctionVisitor<F>(F);

impl<'de, F, T> Visitor<'de, T> for FunctionVisitor<F>
where
    F: FnOnce(&T),
    T: ?Sized,
{
    type Ok = ();

    #[inline]
    fn visit_ref(self, value: &T) -> Result<Self::Ok, Error>
    where
        Self: Sized,
    {
        (self.0)(value);
        Ok(())
    }
}
