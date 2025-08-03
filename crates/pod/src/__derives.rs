//! Derive for types which can be converted from and to a pod.
//!
//! # Examples
//!
//! Using lifetimes:
//!
//! ```
//! #[derive(Debug, PartialEq, Readable, Writable)]
//! struct Struct<'de> {
//!     a: &'de [u8],
//!     b: &'de str,
//! }
//!
//! let mut pod = pod::array();
//! pod.as_mut().write(Struct {
//!     a: &b"hello"[..],
//!     b: &b"world"[..],
//! })?;
//!
//! let read = pod.as_ref().read::<Struct>()?;
//! assert_eq!(read, Struct {
//!     a: &b"hello"[..],
//!     b: "world",
//! });
//! # Ok::<_, pod::Error>(())
//! ```
//!
//! # Attributes
//!
//! ## Container attributes
//!
//! Container attributes types which are added to the container of the generated
//! type.
//!
//! ```
//! use pod::Pod;
//!
//! #[derive(Pod)]
//! #[pod(crate = pod)]
//! struct AudioMeta {
//!     pub channels: u32,
//! }
//! ```
//!
//! #### `#[pod(crate [ = <path>])`
//!
//! Specify the path to where pod types are located. The default is `::pod`.
//! Omitting the `<path>` argument loads types from the current crate.
//!
//! ```
//! use pod::Readable;
//!
//! #[derive(Readable)]
//! #[pod(crate = ::pod)]
//! struct AudioMeta {
//!     pub channels: u32,
//! }
//! ```
