pod::macros::id! {
    #[example = FORMAT]
    pub struct Param {
        UNKNOWN,
        PROP_INFO = 1,
        PROPS = 2,
        ENUM_FORMAT = 3,
        FORMAT = 4,
        BUFFERS = 5,
        META = 6,
        IO = 7,
        ENUM_PROFILE = 8,
        PROFILE = 9,
        ENUM_PORT_CONFIG = 10,
        PORT_CONFIG = 11,
        ENUM_ROUTE = 12,
        ROUTE = 13,
        CONTROL = 14,
        LATENCY = 15,
        PROCESS_LATENCY = 16,
        TAG = 17,
    }

    #[example = AUDIO]
    pub struct MediaType {
        UNKNOWN,
        AUDIO = 1,
        VIDEO = 2,
        IMAGE = 3,
        BINARY = 4,
        STREAM = 5,
        APPLICATION = 6,
    }

    #[example = OPUS]
    pub struct MediaSubType {
        UNKNOWN,
        RAW = 0x00001,
        DSP = 0x00002,
        IEC958 = 0x00003,
        DSD = 0x00004,
        START_AUDIO = 0x10000,
        MP3 = 0x10001,
        AAC = 0x10002,
        VORBIS = 0x10003,
        WMA = 0x10004,
        RA = 0x10005,
        SBC = 0x10006,
        ADPCM = 0x10007,
        G723 = 0x10008,
        G726 = 0x10009,
        G729 = 0x1000a,
        AMR = 0x1000b,
        GSM = 0x1000c,
        ALAC = 0x1000d,
        FLAC = 0x1000e,
        APE = 0x1000f,
        OPUS = 0x10010,
        START_VIDEO = 0x20000,
        H264 = 0x20001,
        MJPG = 0x20002,
        DV = 0x20003,
        MPEGTS = 0x20004,
        H263 = 0x20005,
        MPEG1 = 0x20006,
        MPEG2 = 0x20007,
        MPEG4 = 0x20008,
        XVID = 0x20009,
        VC1 = 0x2000a,
        VP8 = 0x2000b,
        VP9 = 0x2000c,
        BAYER = 0x2000d,
        START_IMAGE = 0x30000,
        JPEG = 0x30001,
        START_BINARY = 0x40000,
        START_STREAM = 0x50000,
        MIDI = 0x50001,
        START_APPLICATION = 0x60000,
        CONTROL = 0x60001,
    }

    /// These correspond to the values of `SPA_TYPE_OBJECT_*`.
    #[example = FORMAT]
    pub struct ObjectType {
        UNKNOWN,
        PROP_INFO = 0x40001,
        PROPS = 0x40002,
        FORMAT = 0x40003,
        PARAM_BUFFERS = 0x40004,
        PARAM_META = 0x40005,
        PARAM_IO = 0x40006,
        PARAM_PROFILE = 0x40007,
        PARAM_PORT_CONFIG = 0x40008,
        PARAM_ROUTE = 0x40009,
        PROFILER = 0x4000a,
        PARAM_LATENCY = 0x4000b,
        PARAM_PROCESS_LATENCY = 0x4000c,
        PARAM_TAG = 0x4000d,
    }

    #[example = FREQUENCY]
    pub struct Prop {
        UNKNOWN,
        START_DEVICE = 0x100,
        DEVICE = 0x101,
        DEVICE_NAME = 0x102,
        DEVICE_FD = 0x103,
        CARD = 0x104,
        CARD_NAME = 0x105,
        MIN_LATENCY = 0x106,
        MAX_LATENCY = 0x107,
        PERIODS = 0x108,
        PERIOD_SIZE = 0x109,
        PERIOD_EVENT = 0x10a,
        LIVE = 0x10b,
        RATE = 0x10c,
        QUALITY = 0x10d,
        BLUETOOTH_AUDIO_CODEC = 0x10e,
        BLUETOOTH_OFFLOAD_ACTIVE = 0x10f,
        START_AUDIO = 0x10000,
        WAVE_TYPE = 0x10001,
        FREQUENCY = 0x10002,
        /// A volume (Float), 0.0 silence, 1.0 no attenutation.
        VOLUME = 0x10003,
        /// Mute (Bool)
        MUTE = 0x10004,
        PATTERN_TYPE = 0x10005,
        DITHER_TYPE = 0x10006,
        TRUNCATE = 0x10007,
        /// A volume array, one (linear) volume per channel (Array of Float).
        /// 0.0 is silence, 1.0 is without attenuation. This is the effective
        /// volume that is applied. It can result in a hardware volume and
        /// software volume (see softVolumes)
        CHANNEL_VOLUMES = 0x10008,
        /// A volume base (Float)
        VOLUME_BASE = 0x10009,
        /// A volume step (Float)
        VOLUME_STEP = 0x1000a,
        /// A channelmap array (Array (Id enum spa_audio_channel)).
        CHANNEL_MAP = 0x1000b,
        /// mute (Bool)
        MONITOR_MUTE = 0x1000c,
        /// a volume array, one (linear) volume per channel (Array of Float).
        MONITOR_VOLUMES = 0x1000d,
        /// Delay adjustment.
        LATENCY_OFFSET_NSEC = 0x1000e,
        /// Mute (Bool) applied in software.
        SOFT_MUTE = 0x1000f,
        /// A volume array, one (linear) volume per channel
        /// (Array of Float). 0.0 is silence, 1.0 is without
        /// attenuation. This is the volume applied in
        /// software, there might be a part applied in
        /// hardware.
        SOFT_VOLUMES = 0x10010,
        /// Enabled IEC958 (S/PDIF) codecs (Array (Id enum spa_audio_iec958_codec).
        IEC958_CODECS = 0x10011,
        /// Samples to ramp the volume over.
        VOLUME_RAMP_SAMPLES = 0x10012,
        /// Step or incremental Samples to ramp the volume over.
        VOLUME_RAMP_STEP_SAMPLES = 0x10013,
        /// Time in millisec to ramp the volume over.
        VOLUME_RAMP_TIME = 0x10014,
        /// Step or incremental Time in nano seconds to ramp the.
        VOLUME_RAMP_STEP_TIME = 0x10015,
        /// The scale or graph to used to ramp the volume.
        VOLUME_RAMP_SCALE = 0x10016,
        /// Video related properties.
        START_VIDEO = 0x20000,
        BRIGHTNESS = 0x20001,
        CONTRAST = 0x20002,
        SATURATION = 0x20003,
        HUE = 0x20004,
        GAMMA = 0x20005,
        EXPOSURE = 0x20006,
        GAIN = 0x20007,
        SHARPNESS = 0x20008,
        /// Other properties.
        START_OTHER = 0x80000,
        /// simple control params (Struct((String: key, Pod: value)*)).
        PARAMS = 0x80001,
        START_CUSTOM = 0x1000000,
    }

    /// The representation of `enum spa_io_type`.
    #[example = FREQUENCY]
    pub struct IoType {
        UNKNOWN,
        /// Area to exchange buffers, `struct spa_io_buffers`.
        BUFFERS = 1,
        /// Expected byte range, `struct spa_io_range` (currently not used in
        /// PipeWire).
        RANGE = 2,
        /// Area to update clock information, `struct spa_io_clock`.
        CLOCK = 3,
        /// Latency reporting, `struct spa_io_latency` (currently not used in
        /// PipeWire). See `spa_param_latency`.
        LATENCY = 4,
        /// Area for control messages, `struct spa_io_sequence`.
        CONTROL = 5,
        /// Area for notify messages, `struct spa_io_sequence`.
        NOTIFY = 6,
        /// Position information in the graph, `struct spa_io_position`.
        POSITION = 7,
        /// Rate matching between nodes, `struct spa_io_rate_match`.
        RATE_MATCH = 8,
        /// Memory pointer, `struct spa_io_memory` (currently not used in
        /// PipeWire).
        MEMORY = 9,
        /// Async area to exchange buffers, `struct spa_io_async_buffers`.
        ASYNC_BUFFERS = 10,
    }

    #[example = MEDIA_TYPE]
    pub struct Format {
        UNKNOWN,
        /// media type (Id enum spa_media_type).
        MEDIA_TYPE = 1,
        /// media subtype (Id enum spa_media_subtype).
        MEDIA_SUB_TYPE = 2,
        /// audio format, (Id enum spa_audio_format).
        AUDIO_FORMAT = 0x10001,
        /// optional flags (Int).
        AUDIO_FLAGS = 0x10002,
        /// sample rate (Int).
        AUDIO_RATE = 0x10003,
        /// number of audio channels (Int).
        AUDIO_CHANNELS = 0x10004,
        /// channel positions (Id enum spa_audio_position).
        AUDIO_POSITION = 0x10005,
        /// codec used (IEC958) (Id enum spa_audio_iec958_codec).
        AUDIO_IEC958_CODEC = 0x10006,
        /// bit order (Id enum spa_param_bitorder).
        AUDIO_BITORDER = 0x10007,
        /// Interleave bytes (Int).
        AUDIO_INTERLEAVE = 0x10008,
        /// bit rate (Int).
        AUDIO_BITRATE = 0x10009,
        /// audio data block alignment (Int).
        AUDIO_BLOCK_ALIGN = 0x1000a,
        /// AAC stream format, (Id enum spa_audio_aac_stream_format).
        AUDIO_AAC_STREAM_FORMAT = 0x1000b,
        /// WMA profile (Id enum spa_audio_wma_profile).
        AUDIO_WMA_PROFILE = 0x1000c,
        /// AMR band mode (Id enum spa_audio_amr_band_mode).
        AUDIO_AMR_BAND_MODE = 0x1000d,
        /// video format (Id enum spa_video_format).
        VIDEO_FORMAT = 0x20001,
        /// format modifier (Long) use only with DMA-BUF and omit for other buffer types.
        VIDEO_MODIFIER = 0x20002,
        /// size (Rectangle).
        VIDEO_SIZE = 0x20003,
        /// frame rate (Fraction).
        VIDEO_FRAMERATE = 0x20004,
        /// maximum frame rate (Fraction).
        VIDEO_MAX_FRAMERATE = 0x20005,
        /// number of views (Int).
        VIDEO_VIEWS = 0x20006,
        /// (Id enum spa_video_interlace_mode).
        VIDEO_INTERLACE_MODE = 0x20007,
        /// (Rectangle).
        VIDEO_PIXEL_ASPECT_RATIO = 0x20008,
        /// (Id enum spa_video_multiview_mode).
        VIDEO_MULTIVIEW_MODE = 0x20009,
        /// (Id enum spa_video_multiview_flags).
        VIDEO_MULTIVIEW_FLAGS = 0x2000a,
        /// /Id enum spa_video_chroma_site).
        VIDEO_CHROMA_SITE = 0x2000b,
        /// /Id enum spa_video_color_range).
        VIDEO_COLOR_RANGE = 0x2000c,
        /// /Id enum spa_video_color_matrix).
        VIDEO_COLOR_MATRIX = 0x2000d,
        /// /Id enum spa_video_transfer_function).
        VIDEO_TRANSFER_FUNCTION = 0x2000e,
        /// /Id enum spa_video_color_primaries).
        VIDEO_COLOR_PRIMARIES = 0x2000f,
        /// (Int).
        VIDEO_PROFILE = 0x20010,
        /// (Int).
        VIDEO_LEVEL = 0x20011,
        /// (Id enum spa_h264_stream_format).
        VIDEO_H264_STREAM_FORMAT = 0x20012,
        /// (Id enum spa_h264_alignment).
        VIDEO_H264_ALIGNMENT = 0x20013,
        /// possible control types (flags choice Int, mask of enum spa_control_type).
        CONTROL_TYPES = 0x60001,
    }

    #[example = S16]
    pub struct AudioFormat {
        UNKNOWN,
        ENCODED = 1,
        S8 = 0x101,
        U8 = 0x102,
        S16_LE = 0x103,
        S16_BE = 0x104,
        U16_LE = 0x105,
        U16_BE = 0x106,
        S24_32_LE = 0x107,
        S24_32_BE = 0x108,
        U24_32_LE = 0x109,
        U24_32_BE = 0x10a,
        S32_LE = 0x10b,
        S32_BE = 0x10c,
        U32_LE = 0x10d,
        U32_BE = 0x10e,
        S24_LE = 0x10f,
        S24_BE = 0x110,
        U24_LE = 0x111,
        U24_BE = 0x112,
        S20_LE = 0x113,
        S20_BE = 0x114,
        U20_LE = 0x115,
        U20_BE = 0x116,
        S18_LE = 0x117,
        S18_BE = 0x118,
        U18_LE = 0x119,
        U18_BE = 0x11a,
        F32_LE = 0x11b,
        F32_BE = 0x11c,
        F64_LE = 0x11d,
        F64_BE = 0x11e,
        ULAW = 0x11f,
        ALAW = 0x120,
        U8P = 0x201,
        S16P = 0x202,
        S24_32P = 0x203,
        S32P = 0x204,
        S24P = 0x205,
        F32P = 0x206,
        F64P = 0x207,
        S8P = 0x208,
    }

    #[example = SUSPEND]
    pub struct NodeCommand {
        UNKNOWN,
        /// Suspend a node, this removes all configuredformats and closes any
        /// devices.
        SUSPEND = 0,
        /// Pause a node. this makes it stop emitting scheduling events.
        PAUSE = 1,
        /// Start a node, this makes it start emitting scheduling events.
        START = 2,
        ENABLE = 3,
        DISABLE = 4,
        FLUSH = 5,
        DRAIN = 6,
        MARKER = 7,
        /// Begin a set of parameter enumerations or configuration that require
        /// the device to remain opened, like query formats and then set a
        /// format.
        PARAM_BEGIN = 8,
        /// End a transaction.
        PARAM_END = 9,
        /// Sent to a driver when some other node emitted the RequestProcess
        /// event.
        REQUEST_PROCESS = 10,
    }

    #[example = NODE]
    pub struct CommandType {
        UNKNOWN,
        DEVICE = 0x30001,
        NODE = 0x30002,
    }
}

impl AudioFormat {
    /// The default signed 16-bit format (little endian).
    pub const S16: Self = crate::macros::endian!(Self::S16_LE, Self::S16_BE);
    pub const U16: Self = crate::macros::endian!(Self::U16_LE, Self::U16_BE);
    pub const S24_32: Self = crate::macros::endian!(Self::S24_32_LE, Self::S24_32_BE);
    pub const U24_32: Self = crate::macros::endian!(Self::U24_32_LE, Self::U24_32_BE);
    pub const S32: Self = crate::macros::endian!(Self::S32_LE, Self::S32_BE);
    pub const U32: Self = crate::macros::endian!(Self::U32_LE, Self::U32_BE);
    pub const S24: Self = crate::macros::endian!(Self::S24_LE, Self::S24_BE);
    pub const U24: Self = crate::macros::endian!(Self::U24_LE, Self::U24_BE);
    pub const S20: Self = crate::macros::endian!(Self::S20_LE, Self::S20_BE);
    pub const U20: Self = crate::macros::endian!(Self::U20_LE, Self::U20_BE);
    pub const S18: Self = crate::macros::endian!(Self::S18_LE, Self::S18_BE);
    pub const U18: Self = crate::macros::endian!(Self::U18_LE, Self::U18_BE);
    pub const F32: Self = crate::macros::endian!(Self::F32_LE, Self::F32_BE);
    pub const F64: Self = crate::macros::endian!(Self::F64_LE, Self::F64_BE);
    pub const S16_OE: Self = crate::macros::endian!(Self::S16_BE, Self::S16_LE);
    pub const U16_OE: Self = crate::macros::endian!(Self::U16_BE, Self::U16_LE);
    pub const S24_32_OE: Self = crate::macros::endian!(Self::S24_32_BE, Self::S24_32_LE);
    pub const U24_32_OE: Self = crate::macros::endian!(Self::U24_32_BE, Self::U24_32_LE);
    pub const S32_OE: Self = crate::macros::endian!(Self::S32_BE, Self::S32_LE);
    pub const U32_OE: Self = crate::macros::endian!(Self::U32_BE, Self::U32_LE);
    pub const S24_OE: Self = crate::macros::endian!(Self::S24_BE, Self::S24_LE);
    pub const U24_OE: Self = crate::macros::endian!(Self::U24_BE, Self::U24_LE);
    pub const S20_OE: Self = crate::macros::endian!(Self::S20_BE, Self::S20_LE);
    pub const U20_OE: Self = crate::macros::endian!(Self::U20_BE, Self::U20_LE);
    pub const S18_OE: Self = crate::macros::endian!(Self::S18_BE, Self::S18_LE);
    pub const U18_OE: Self = crate::macros::endian!(Self::U18_BE, Self::U18_LE);
    pub const F32_OE: Self = crate::macros::endian!(Self::F32_BE, Self::F32_LE);
    pub const F64_OE: Self = crate::macros::endian!(Self::F64_BE, Self::F64_LE);
    pub const DSP_S32: Self = Self::S24_32P;
    pub const DSP_F32: Self = Self::F32P;
    pub const DSP_F64: Self = Self::F64P;
}
