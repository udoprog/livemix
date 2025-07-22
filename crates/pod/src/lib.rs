#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

#[cfg(test)]
mod tests;

pub(crate) mod ty;
pub(crate) mod utils;

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
