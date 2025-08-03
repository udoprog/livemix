#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

pub(crate) const PADDING: usize = core::mem::size_of::<u64>();

#[cfg(all(test, feature = "alloc"))]
mod tests;

pub mod __derives;
pub mod macros;

pub(crate) mod bstr;

mod into_raw;
#[doc(inline)]
pub use self::into_raw::IntoRaw;

mod pod;
#[doc(inline)]
pub use self::pod::Pod;

mod typed_pod;
pub use self::typed_pod::TypedPod;

mod pod_kind;
pub use self::pod_kind::{
    BuildPod, ChildPod, ControlPod, PackedPod, PaddedPod, PropertyPod, ReadPod,
};

pub(crate) mod ty;
pub use self::ty::Type;

pub mod utils;

mod id;
pub use self::id::{Id, RawId};

mod writable;
#[doc(inline)]
pub use self::writable::Writable;
#[doc(inline)]
/// See [`__derives`] for documentation.
pub use pod_macros::Writable;

mod readable;
#[doc(inline)]
pub use self::readable::Readable;
#[doc(inline)]
/// See [`__derives`] for documentation.
pub use pod_macros::Readable;

mod unsized_writable;
pub use self::unsized_writable::UnsizedWritable;

pub(crate) mod sized_writable;
pub use self::sized_writable::SizedWritable;

mod unsized_readable;
pub use self::unsized_readable::UnsizedReadable;

pub(crate) mod sized_readable;
pub use self::sized_readable::SizedReadable;

mod read;
pub use self::read::{Array, Choice, Object, Sequence, Struct};

pub mod buf;
#[cfg(feature = "alloc")]
#[doc(inline)]
pub use self::buf::DynamicBuf;
#[doc(inline)]
pub use self::buf::{ArrayBuf, Slice, WriterSlice};

mod writer;
pub use self::writer::Writer;

mod as_slice;
pub use self::as_slice::AsSlice;

mod split_reader;
pub use self::split_reader::SplitReader;

mod reader;
pub use self::reader::Reader;

mod visitor;
pub use self::visitor::Visitor;

mod error;
pub use self::error::Error;

mod rectangle;
pub use self::rectangle::Rectangle;

mod fraction;
pub use self::fraction::Fraction;

mod bitmap;
pub use self::bitmap::Bitmap;
#[cfg(feature = "alloc")]
pub use self::bitmap::OwnedBitmap;

mod property;
pub use self::property::Property;

mod control;
pub use self::control::Control;

mod pointer;
pub use self::pointer::Pointer;

mod fd;
pub use self::fd::Fd;

mod choice;
pub use self::choice::ChoiceType;

pub mod builder;
#[doc(inline)]
pub use self::builder::Builder;

mod pod_stream;
#[doc(inline)]
pub use self::pod_stream::PodStream;

mod pod_sink;
#[doc(inline)]
pub use self::pod_sink::PodSink;

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
pub fn array() -> Builder<ArrayBuf> {
    Builder::array()
}

/// Construct a new [`Pod`] with a 128 word-sized array buffer.
///
/// # Examples
///
/// ```
/// use pod::Reader;
///
/// let mut pod = pod::buf::slice(&[0x7f; 32]);
/// assert_eq!(pod.read::<u32>()?, 0x7f7f7f7f);
/// # Ok::<_, pod::Error>(())
/// ```
#[inline]
pub fn slice(data: &[u8]) -> Slice<'_> {
    Slice::new(data)
}

/// Construct a new [`Pod`] with a dynamically sized buffer.
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
#[cfg(feature = "alloc")]
pub fn dynamic() -> Builder<DynamicBuf> {
    Builder::dynamic()
}
