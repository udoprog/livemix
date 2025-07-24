#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

#[cfg(all(test, feature = "alloc"))]
mod tests;

pub(crate) const WORD_SIZE: usize = 8;

mod pod;
pub use self::pod::Pod;

mod typed_pod;
pub use self::typed_pod::TypedPod;

pub(crate) mod ty;
pub use self::ty::Type;

pub(crate) mod utils;

pub mod id;
pub use self::id::{Id, IntoId};

mod en;
pub use self::en::{Encode, EncodeUnsized};

mod de;
pub use self::de::{Decode, DecodeUnsized};

mod array_buf;
pub use self::array_buf::ArrayBuf;

mod writer;
pub use self::writer::Writer;

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
