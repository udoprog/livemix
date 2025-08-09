//! Helper types for interacting with parameter objects.

use pod::{Readable, Writable};

use crate::id;

/// A [`PARAM_IO`] object type.
///
/// [`PARAM_IO`]: id::ObjectType::PARAM_IO
#[derive(Readable, Writable)]
#[pod(object(type = id::ObjectType::PARAM_IO, id = id::Param::IO))]
pub struct Io {
    #[pod(property(key = id::ParamIoKey::ID))]
    pub ty: id::IoType,
    #[pod(property(key = id::ParamIoKey::SIZE))]
    pub size: usize,
}

/// A [`PARAM_META`] object type.
///
/// [`PARAM_META`]: id::ObjectType::PARAM_META
#[derive(Readable, Writable)]
#[pod(object(type = id::ObjectType::PARAM_META, id = id::Param::META))]
pub struct Meta {
    #[pod(property(key = id::ParamMetaKey::TYPE))]
    pub ty: id::Meta,
    #[pod(property(key = id::ParamMetaKey::SIZE))]
    pub size: usize,
}
