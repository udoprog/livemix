#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

#[cfg(all(test, feature = "alloc"))]
mod tests;

pub mod macros;

pub(crate) mod bstr;

mod pod;
#[doc(inline)]
pub use self::pod::Pod;

mod typed_pod;
pub use self::typed_pod::TypedPod;

pub(crate) mod ty;
pub use self::ty::Type;

pub mod utils;

mod id;
pub use self::id::{Id, RawId};

mod encode_into;
pub use self::encode_into::EncodeInto;

mod decode_from;
pub use self::decode_from::DecodeFrom;

mod en;
pub use self::en::{Encode, EncodeUnsized};

mod de;
pub use self::de::{Array, Choice, Decode, DecodeUnsized, Object, Sequence, Struct};

pub mod buf;
#[doc(inline)]
pub use self::buf::ArrayBuf;
#[cfg(feature = "alloc")]
#[doc(inline)]
pub use self::buf::DynamicBuf;

mod writer;
pub use self::writer::Writer;

mod as_reader;
pub use self::as_reader::AsReader;

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
pub fn array() -> Builder<ArrayBuf<u64>> {
    Builder::array()
}

/// Construct a new [`Pod`] with a dynamically sized buffer.
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
#[cfg(feature = "alloc")]
pub fn dynamic() -> Builder<DynamicBuf<u64>> {
    Builder::dynamic()
}
