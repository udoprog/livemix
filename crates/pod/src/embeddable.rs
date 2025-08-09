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
/// use protocol::id::{Format, ObjectType, Param, MediaSubType, MediaType, AudioFormat};
///
/// #[derive(Debug, PartialEq, Readable, Writable)]
/// #[pod(object(type = ObjectType::FORMAT, id = Param::FORMAT))]
/// struct RawFormat {
///     #[pod(property(key = Format::MEDIA_TYPE))]
///     media_type: MediaType,
///     #[pod(property(key = Format::MEDIA_SUB_TYPE))]
///     media_sub_type: MediaSubType,
///     #[pod(property(key = Format::AUDIO_FORMAT))]
///     audio_format: AudioFormat,
///     #[pod(property(key = Format::AUDIO_CHANNELS))]
///     channels: u32,
///     #[pod(property(key = Format::AUDIO_RATE))]
///     audio_rate: u32,
/// }
///
/// let mut pod = pod::array();
/// let object = pod.as_mut().embed(RawFormat {
///     media_type: MediaType::AUDIO,
///     media_sub_type: MediaSubType::DSP,
///     audio_format: AudioFormat::F32P,
///     channels: 2,
///     audio_rate: 48000,
/// })?;
///
/// assert_eq!(object.object_type::<ObjectType>(), ObjectType::FORMAT);
/// assert_eq!(object.object_id::<Param>(), Param::FORMAT);
///
/// let mut obj = object.as_ref();
///
/// let p = obj.property()?;
/// assert_eq!(p.key::<Format>(), Format::MEDIA_TYPE);
/// assert_eq!(p.value().read::<MediaType>()?, MediaType::AUDIO);
///
/// let p = obj.property()?;
/// assert_eq!(p.key::<Format>(), Format::MEDIA_SUB_TYPE);
/// assert_eq!(p.value().read::<MediaSubType>()?, MediaSubType::DSP);
///
/// let p = obj.property()?;
/// assert_eq!(p.key::<Format>(), Format::AUDIO_FORMAT);
/// assert_eq!(p.value().read::<AudioFormat>()?, AudioFormat::F32P);
///
/// let p = obj.property()?;
/// assert_eq!(p.key::<Format>(), Format::AUDIO_CHANNELS);
/// assert_eq!(p.value().read::<u32>()?, 2);
///
/// let p = obj.property()?;
/// assert_eq!(p.key::<Format>(), Format::AUDIO_RATE);
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
