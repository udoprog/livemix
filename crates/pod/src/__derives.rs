//! Derive for types which can be converted from and to a pod.
//!
//! # Examples
//!
//! Using lifetimes:
//!
//! ```
//! use pod::{Readable, Writable};
//!
//! #[derive(Debug, PartialEq, Readable, Writable)]
//! struct Struct<'de> {
//!     a: &'de [u8],
//!     b: &'de str,
//! }
//!
//! let mut pod = pod::array();
//! pod.as_mut().write(Struct {
//!     a: &b"hello"[..],
//!     b: "world",
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
//! use pod::{Readable, Writable};
//!
//! #[derive(Readable, Writable)]
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
//! use pod::{Readable, Writable};
//!
//! #[derive(Readable, Writable)]
//! #[pod(crate = ::pod)]
//! struct AudioMeta {
//!     pub channels: u32,
//! }
//! ```
//!
//! #### `#[pod(object(type = <type>, id = <id>))` and `#[pod(property(key = <key>))]`
//!
//! Indicates that the struct should be encoded as an object with the specified
//! type.
//!
//! Object pods are special as in they have named properties that can be bound
//! to fields.
//!
//! ```
//! use pod::{Readable, Writable};
//! use protocol::id::{FormatKey, ObjectType, Param, MediaSubType, MediaType, AudioFormat};
//!
//! #[derive(Debug, PartialEq, Readable, Writable)]
//! #[pod(object(type = ObjectType::FORMAT, id = Param::FORMAT))]
//! struct RawFormat {
//!     #[pod(property(key = FormatKey::MEDIA_TYPE))]
//!     media_type: MediaType,
//!     #[pod(property(key = FormatKey::MEDIA_SUB_TYPE))]
//!     media_sub_type: MediaSubType,
//!     #[pod(property(key = FormatKey::AUDIO_FORMAT))]
//!     audio_format: AudioFormat,
//!     #[pod(property(key = FormatKey::AUDIO_CHANNELS))]
//!     channels: u32,
//!     #[pod(property(key = FormatKey::AUDIO_RATE))]
//!     audio_rate: u32,
//! }
//! ```
//!
//! Note that if a choice is encountered while decoding a pod, the value of the
//! choice will only be extracted if it has the type `NONE`.
