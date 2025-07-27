#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

#[cfg(all(test, feature = "alloc"))]
mod tests;

pub mod macros;

pub(crate) const WORD_SIZE: u32 = core::mem::size_of::<u64>() as u32;

pub(crate) mod bstr;

pub mod pod;
#[doc(inline)]
pub use self::pod::Pod;

mod typed_pod;
pub use self::typed_pod::TypedPod;

pub(crate) mod ty;
pub use self::ty::Type;

pub mod utils;

mod id;
pub use self::id::{Id, RawId};

mod en;
pub use self::en::{Encode, EncodeUnsized};

mod de;
pub use self::de::{Array, Choice, Decode, DecodeUnsized, Object, Sequence, Struct};

mod array;
pub use self::array::Buf;

mod writer;
pub use self::writer::Writer;

mod as_reader;
pub use self::as_reader::AsReader;

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
