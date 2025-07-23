#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

#[cfg(test)]
mod tests;

pub(crate) const WORD_SIZE: usize = 4;
pub(crate) const DWORD_SIZE: usize = 8;

pub(crate) mod ty;
pub use self::ty::Type;

pub(crate) mod utils;

pub mod id;
pub use self::id::{Id, IntoId};

mod en;
pub use self::en::{Encode, EncodeUnsized, Encoder};

mod de;
pub use self::de::{Decode, DecodeUnsized, Decoder};

mod array_buf;
pub use self::array_buf::ArrayBuf;

mod slice;
pub use self::slice::Slice;

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
