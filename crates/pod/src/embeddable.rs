use crate::{BuildPod, Builder, Error, Writer};

/// Helper trait to more easily write value to a [`Builder`] and return a handle
/// to the written value that can be immediately used.
///
/// This is used through the [`Builder::embed`] and similar methods.
///
/// This is implemented for many types, including tuples and arrays. When tuples
/// and arrays are used, they write each "contained" value in sequence. For
/// structs this means each field, for choices each choice, and so forth.
///
/// [`Builder`]: crate::Builder
/// [`Builder::embed`]: crate::Builder::embed
///
/// # Examples
///
/// ```
/// use pod::{Readable, Writable};
/// use protocol::id;
///
/// #[derive(Debug, PartialEq, Readable, Writable)]
/// #[pod(object(type = id::ObjectType::FORMAT, id = id::Param::FORMAT))]
/// struct RawFormat {
///     #[pod(property(key = id::Format::MEDIA_TYPE))]
///     media_type: id::MediaType,
///     #[pod(property(key = id::Format::MEDIA_SUB_TYPE))]
///     media_sub_type: id::MediaSubType,
///     #[pod(property(key = id::Format::AUDIO_FORMAT))]
///     audio_format: id::AudioFormat,
///     #[pod(property(key = id::Format::AUDIO_CHANNELS))]
///     channels: u32,
///     #[pod(property(key = id::Format::AUDIO_RATE))]
///     audio_rate: u32,
/// }
///
/// let mut pod = pod::array();
/// let object = pod.as_mut().embed(RawFormat {
///     media_type: id::MediaType::AUDIO,
///     media_sub_type: id::MediaSubType::DSP,
///     audio_format: id::AudioFormat::F32P,
///     channels: 2,
///     audio_rate: 48000,
/// })?;
///
/// assert_eq!(object.object_type::<id::ObjectType>(), id::ObjectType::FORMAT);
/// assert_eq!(object.object_id::<id::Param>(), id::Param::FORMAT);
///
/// let mut obj = object.as_ref();
///
/// let p = obj.property()?;
/// assert_eq!(p.key::<id::Format>(), id::Format::MEDIA_TYPE);
/// assert_eq!(p.value().read::<id::MediaType>()?, id::MediaType::AUDIO);
///
/// let p = obj.property()?;
/// assert_eq!(p.key::<id::Format>(), id::Format::MEDIA_SUB_TYPE);
/// assert_eq!(p.value().read::<id::MediaSubType>()?, id::MediaSubType::DSP);
///
/// let p = obj.property()?;
/// assert_eq!(p.key::<id::Format>(), id::Format::AUDIO_FORMAT);
/// assert_eq!(p.value().read::<id::AudioFormat>()?, id::AudioFormat::F32P);
///
/// let p = obj.property()?;
/// assert_eq!(p.key::<id::Format>(), id::Format::AUDIO_CHANNELS);
/// assert_eq!(p.value().read::<u32>()?, 2);
///
/// let p = obj.property()?;
/// assert_eq!(p.key::<id::Format>(), id::Format::AUDIO_RATE);
/// assert_eq!(p.value().read::<u32>()?, 48000);
/// # Ok::<_, pod::Error>(())
/// ```
pub trait Embeddable {
    /// The type of the embedded value.
    type Embed<W>
    where
        W: Writer;

    #[doc(hidden)]
    fn embed_into<W, P>(&self, pod: Builder<W, P>) -> Result<Self::Embed<W>, Error>
    where
        W: Writer,
        P: BuildPod;
}

impl<T> Embeddable for &T
where
    T: ?Sized + Embeddable,
{
    type Embed<W>
        = T::Embed<W>
    where
        W: Writer;

    #[inline]
    fn embed_into<W, P>(&self, pod: Builder<W, P>) -> Result<Self::Embed<W>, Error>
    where
        W: Writer,
        P: BuildPod,
    {
        (*self).embed_into(pod)
    }
}
