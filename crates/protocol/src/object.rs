/// Structs which can bind to protocol objects.
use pod::{Readable, Writable};

use crate::id;

/// Some of the contents of the format parameter.
#[derive(Debug, Clone, PartialEq, Readable, Writable)]
#[pod(object(type = id::ObjectType::FORMAT, id = id::Param::FORMAT))]
pub struct Format {
    /// The media type of the format.
    #[pod(property(key = id::FormatKey::MEDIA_TYPE))]
    pub media_type: id::MediaType,
    /// The media type of the format.
    #[pod(property(key = id::FormatKey::MEDIA_SUB_TYPE))]
    pub media_sub_type: id::MediaSubType,
}

/// A raw audio format.
#[derive(Debug, Clone, PartialEq, Readable, Writable)]
#[pod(object(type = id::ObjectType::FORMAT, id = id::Param::FORMAT))]
pub struct AudioFormat {
    /// The media type of the format.
    #[pod(property(key = id::FormatKey::MEDIA_TYPE))]
    pub media_type: id::MediaType,
    /// The media type of the format.
    #[pod(property(key = id::FormatKey::MEDIA_SUB_TYPE))]
    pub media_sub_type: id::MediaSubType,
    /// The media type of the format.
    #[pod(property(key = id::FormatKey::AUDIO_FORMAT))]
    pub format: id::AudioFormat,
    /// The media type of the format.
    #[pod(property(key = id::FormatKey::AUDIO_CHANNELS))]
    pub channels: u32,
    /// The media type of the format.
    #[pod(property(key = id::FormatKey::AUDIO_RATE))]
    pub rate: u32,
}
