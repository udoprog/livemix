macro_rules! declare_id {
    (
        $(
            #[example = $example:ident]
            $ty_vis:vis enum $ty:ident {
                $default:ident = $default_value:expr,
                $(
                    $(#[$($field_meta:meta)*])* $field:ident = $field_value:expr
                ),* $(,)?
            }
        )*
    ) => {
        $(
            #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
            #[repr(u32)]
            $ty_vis enum $ty {
                $default = $default_value,
                $(
                    $(#[$($field_meta)*])* $field = $field_value,
                )*
            }

            impl self::sealed::Sealed for $ty {}

            impl $crate::en::encode::sealed::Sealed for $ty {}
            impl $crate::de::decode::sealed::Sealed for $ty {}

            #[doc = concat!(" Encode an [`", stringify!($ty), "`].")]
            ///
            /// # Examples
            ///
            /// ```
            /// use pod::Pod;
            #[doc = concat!(" use pod::id::", stringify!($ty), ";")]
            ///
            /// let mut pod = Pod::array();
            #[doc = concat!(" pod.encode(", stringify!($ty), "::", stringify!($example), ")?;")]
            /// # Ok::<_, pod::Error>(())
            /// ```
            impl $crate::Encode for $ty {
                const TYPE: $crate::Type = $crate::Type::ID;

                #[inline]
                fn size(&self) -> u32 {
                    4
                }

                #[inline]
                fn encode(&self, writer: impl $crate::Writer<u64>) -> Result<(), $crate::Error> {
                    $crate::Id(*self).encode(writer)
                }

                #[inline]
                fn write_content(&self, writer: impl $crate::Writer<u64>) -> Result<(), $crate::Error> {
                    $crate::Id(*self).write_content(writer)
                }
            }

            #[doc = concat!(" Decode an [`", stringify!($ty), "`].")]
            ///
            /// # Examples
            ///
            /// ```
            /// use pod::Pod;
            #[doc = concat!(" use pod::id::", stringify!($ty), ";")]
            ///
            /// let mut pod = Pod::array();
            ///
            #[doc = concat!(" pod.as_mut().encode(", stringify!($ty), "::", stringify!($example), ")?;")]
            ///
            #[doc = concat!(" let id = pod.decode::<", stringify!($ty), ">()?;")]
            #[doc = concat!(" assert_eq!(id, ", stringify!($ty), "::", stringify!($example), ");")]
            ///
            /// let mut pod = Pod::array();
            #[doc = concat!(" pod.as_mut().encode(", stringify!($ty), "::", stringify!($example), ")?;")]
            ///
            #[doc = concat!(" let id = pod.decode::<", stringify!($ty), ">()?;")]
            #[doc = concat!(" assert_eq!(id, ", stringify!($ty), "::", stringify!($example), ");")]
            /// # Ok::<_, pod::Error>(())
            /// ```
            ///
            #[doc = concat!(" Unknown identifiers will be decoded as the default value ", stringify!($default), ".")]
            ///
            /// ```
            /// use pod::{Pod, Id};
            #[doc = concat!(" use pod::id::", stringify!($ty), ";")]
            ///
            /// let mut pod = Pod::array();
            /// pod.as_mut().encode(Id(u32::MAX / 2))?;
            ///
            #[doc = concat!(" let id = pod.decode::<", stringify!($ty), ">()?;")]
            #[doc = concat!(" assert_eq!(id, ", stringify!($ty), "::", stringify!($default), ");")]
            /// # Ok::<_, pod::Error>(())
            /// ```
            impl<'de> $crate::Decode<'de> for $ty {
                const TYPE: $crate::Type = $crate::Type::ID;

                #[inline]
                fn read_content(reader: impl $crate::Reader<'de, u64>, len: u32) -> Result<Self, $crate::Error> {
                    let $crate::Id(id) = $crate::Id::<$ty>::read_content(reader, len)?;
                    Ok(id)
                }
            }

            impl $ty {
                /// Get the identifier value.
                #[inline]
                pub fn into_id(self) -> u32 {
                    self as u32
                }

                /// Convert an identifier value into the type.
                #[inline]
                pub fn from_id(value: u32) -> Self {
                    match value {
                        $($field_value => Self::$field,)*
                        _ => Self::$default,
                    }
                }
            }

            impl IntoId for $ty {
                #[inline]
                fn into_id(self) -> u32 {
                    <$ty>::into_id(self)
                }

                #[inline]
                fn from_id(value: u32) -> Self {
                    <$ty>::from_id(value)
                }
            }
        )*
    };
}

declare_id! {
    #[example = Format]
    pub enum Param {
        Invalid = 0,
        PropInfo = 1,
        Props = 2,
        EnumFormat = 3,
        Format = 4,
        Buffers = 5,
        Meta = 6,
        IO = 7,
        EnumProfile = 8,
        Profile = 9,
        EnumPortConfig = 10,
        PortConfig = 11,
        EnumRoute = 12,
        Route = 13,
        Control = 14,
        Latency = 15,
        ProcessLatency = 16,
        Tag = 17,
    }

    #[example = Opus]
    pub enum MediaSubType {
        Unknown = 0,
        Raw = 1,
        Dsp = 2,
        Iec958 = 3,
        Dsd = 4,
        StartAudio = 0x10000,
        Mp3 = 0x10001,
        Aac = 0x10002,
        Vorbis = 0x10003,
        Wma = 0x10004,
        Ra = 0x10005,
        Sbc = 0x10006,
        Adpcm = 0x10007,
        G723 = 0x10008,
        G726 = 0x10009,
        G729 = 0x1000a,
        Amr = 0x1000b,
        Gsm = 0x1000c,
        Alac = 0x1000d,
        Flac = 0x1000e,
        Ape = 0x1000f,
        Opus = 0x10010,
        StartVideo = 0x20000,
        H264 = 0x20001,
        Mjpg = 0x20002,
        Dv = 0x20003,
        Mpegts = 0x20004,
        H263 = 0x20005,
        Mpeg1 = 0x20006,
        Mpeg2 = 0x20007,
        Mpeg4 = 0x20008,
        Xvid = 0x20009,
        Vc1 = 0x2000a,
        Vp8 = 0x2000b,
        Vp9 = 0x2000c,
        Bayer = 0x2000d,
        StartImage = 0x30000,
        Jpeg = 0x30001,
        StartBinary = 0x40000,
        StartStream = 0x50000,
        Midi = 0x50001,
        StartApplication = 0x60000,
        Control = 0x60001,
    }

    #[example = Format]
    pub enum ObjectType {
        Invalid = 0x40000,
        PropInfo = 0x40001,
        Props = 0x40002,
        Format = 0x40003,
        ParamBuffers = 0x40004,
        ParamMeta = 0x40005,
        ParamIO = 0x40006,
        ParamProfile = 0x40007,
        ParamPortConfig = 0x40008,
        ParamRoute = 0x40009,
        Profiler = 0x4000a,
        ParamLatency = 0x4000b,
        ParamProcessLatency = 0x4000c,
        ParamTag = 0x4000d,
    }

    #[example = Format]
    pub enum Prop {
        Unknown = 0,

        StartDevice = 0x100,
        Device = 0x101,
        DeviceName = 0x102,
        DeviceFd = 0x103,
        Card = 0x104,
        CardName = 0x105,

        MinLatency = 0x106,
        MaxLatency = 0x107,
        Periods = 0x108,
        PeriodSize = 0x109,
        PeriodEvent = 0x10a,
        Live = 0x10b,
        Rate = 0x10c,
        Quality = 0x10d,
        BluetoothAudioCodec = 0x10e,
        BluetoothOffloadActive = 0x10f,

        StartAudio = 0x10000,
        WaveType = 0x10001,
        Frequency = 0x10002,
        /// A volume (Float), 0.0 silence, 1.0 no attenutation.
        Volume = 0x10003,
        /// Mute (Bool)
        Mute = 0x10004,
        PatternType = 0x10005,
        DitherType = 0x10006,
        Truncate = 0x10007,
        /// A volume array, one (linear) volume per channel (Array of Float).
        /// 0.0 is silence, 1.0 is without attenuation. This is the effective
        /// volume that is applied. It can result in a hardware volume and
        /// software volume (see softVolumes)
        ChannelVolumes = 0x10008,
        /// A volume base (Float)
        VolumeBase = 0x10009,
        /// A volume step (Float)
        VolumeStep = 0x1000a,
        /// A channelmap array (Array (Id enum spa_audio_channel)).
        ChannelMap = 0x1000b,
        /// mute (Bool)
        MonitorMute = 0x1000c,
        /// a volume array, one (linear) volume per channel (Array of Float).
        MonitorVolumes = 0x1000d,
        /// Delay adjustment.
        LatencyOffsetNsec = 0x1000e,
        /// Mute (Bool) applied in software.
        SoftMute = 0x1000f,
        /// A volume array, one (linear) volume per channel
        /// (Array of Float). 0.0 is silence, 1.0 is without
        /// attenuation. This is the volume applied in
        /// software, there might be a part applied in
        /// hardware.
        SoftVolumes = 0x10010,

        /// Enabled IEC958 (S/PDIF) codecs (Array (Id enum spa_audio_iec958_codec).
        Iec958Codecs = 0x10011,
        /// Samples to ramp the volume over.
        VolumeRampSamples = 0x10012,
        /// Step or incremental Samples to ramp the volume over.
        VolumeRampStepSamples = 0x10013,
        /// Time in millisec to ramp the volume over.
        VolumeRampTime = 0x10014,
        /// Step or incremental Time in nano seconds to ramp the.
        VolumeRampStepTime = 0x10015,
        /// The scale or graph to used to ramp the volume.
        VolumeRampScale = 0x10016,

        /// Video related properties.
        StartVideo = 0x20000,
        Brightness = 0x20001,
        Contrast = 0x20002,
        Saturation = 0x20003,
        Hue = 0x20004,
        Gamma = 0x20005,
        Exposure = 0x20006,
        Gain = 0x20007,
        Sharpness = 0x20008,

        /// Other properties.
        StartOther = 0x80000,
        /// simple control params (Struct((String: key, Pod: value)*)).
        Params = 0x80001,
        StartCustom = 0x1000000,
    }
}

mod sealed {
    pub trait Sealed {}
    impl Sealed for u32 {}
}

/// Helper trait to convert a type into an `Id`.
pub trait IntoId: Copy + self::sealed::Sealed {
    /// Convert into a numerical identifier.
    #[doc(hidden)]
    fn into_id(self) -> u32;

    /// Convert an `Id` into the underlying type.
    #[doc(hidden)]
    fn from_id(id: u32) -> Self
    where
        Self: Sized;
}

impl IntoId for u32 {
    #[inline]
    fn into_id(self) -> u32 {
        self
    }

    #[inline]
    fn from_id(id: u32) -> Self {
        id
    }
}

/// Helper type that can be used to encode and decode identifiers, including raw
/// ones based on `u32`.
///
/// # Examples
///
/// ```
/// use pod::{Pod, Id};
///
/// let mut pod = Pod::array();
/// pod.as_mut().encode(Id(142u32))?;
/// assert_eq!(pod.decode::<Id<u32>>()?, Id(142u32));
/// # Ok::<_, pod::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Id<T>(pub T);
