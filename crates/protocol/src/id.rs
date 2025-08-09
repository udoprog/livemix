pod::macros::id! {
    #[example = FORMAT]
    #[module = protocol::id]
    pub struct Param {
        UNKNOWN,
        #[constant = libspa_sys::SPA_PARAM_PropInfo]
        PROP_INFO = 1,
        #[constant = libspa_sys::SPA_PARAM_Props]
        PROPS = 2,
        #[constant = libspa_sys::SPA_PARAM_EnumFormat]
        ENUM_FORMAT = 3,
        #[constant = libspa_sys::SPA_PARAM_Format]
        FORMAT = 4,
        #[constant = libspa_sys::SPA_PARAM_Buffers]
        BUFFERS = 5,
        #[constant = libspa_sys::SPA_PARAM_Meta]
        META = 6,
        #[constant = libspa_sys::SPA_PARAM_IO]
        IO = 7,
        #[constant = libspa_sys::SPA_PARAM_EnumProfile]
        ENUM_PROFILE = 8,
        #[constant = libspa_sys::SPA_PARAM_Profile]
        PROFILE = 9,
        #[constant = libspa_sys::SPA_PARAM_EnumPortConfig]
        ENUM_PORT_CONFIG = 10,
        #[constant = libspa_sys::SPA_PARAM_PortConfig]
        PORT_CONFIG = 11,
        #[constant = libspa_sys::SPA_PARAM_EnumRoute]
        ENUM_ROUTE = 12,
        #[constant = libspa_sys::SPA_PARAM_Route]
        ROUTE = 13,
        #[constant = libspa_sys::SPA_PARAM_Control]
        CONTROL = 14,
        #[constant = libspa_sys::SPA_PARAM_Latency]
        LATENCY = 15,
        #[constant = libspa_sys::SPA_PARAM_ProcessLatency]
        PROCESS_LATENCY = 16,
        #[constant = libspa_sys::SPA_PARAM_Tag]
        TAG = 17,
    }

    #[example = AUDIO]
    #[module = protocol::id]
    pub struct MediaType {
        UNKNOWN,
        #[constant = libspa_sys::SPA_MEDIA_TYPE_audio]
        AUDIO = 1,
        #[constant = libspa_sys::SPA_MEDIA_TYPE_video]
        VIDEO = 2,
        #[constant = libspa_sys::SPA_MEDIA_TYPE_image]
        IMAGE = 3,
        #[constant = libspa_sys::SPA_MEDIA_TYPE_binary]
        BINARY = 4,
        #[constant = libspa_sys::SPA_MEDIA_TYPE_stream]
        STREAM = 5,
        #[constant = libspa_sys::SPA_MEDIA_TYPE_application]
        APPLICATION = 6,
    }

    #[example = OPUS]
    #[module = protocol::id]
    pub struct MediaSubType {
        UNKNOWN,
        #[constant = libspa_sys::SPA_MEDIA_SUBTYPE_raw]
        RAW = 0x00001,
        #[constant = libspa_sys::SPA_MEDIA_SUBTYPE_dsp]
        DSP = 0x00002,
        #[constant = libspa_sys::SPA_MEDIA_SUBTYPE_iec958]
        IEC958 = 0x00003,
        #[constant = libspa_sys::SPA_MEDIA_SUBTYPE_dsd]
        DSD = 0x00004,
        #[constant = libspa_sys::SPA_MEDIA_SUBTYPE_mp3]
        MP3 = 0x10001,
        #[constant = libspa_sys::SPA_MEDIA_SUBTYPE_aac]
        AAC = 0x10002,
        #[constant = libspa_sys::SPA_MEDIA_SUBTYPE_vorbis]
        VORBIS = 0x10003,
        #[constant = libspa_sys::SPA_MEDIA_SUBTYPE_wma]
        WMA = 0x10004,
        #[constant = libspa_sys::SPA_MEDIA_SUBTYPE_ra]
        RA = 0x10005,
        #[constant = libspa_sys::SPA_MEDIA_SUBTYPE_sbc]
        SBC = 0x10006,
        #[constant = libspa_sys::SPA_MEDIA_SUBTYPE_adpcm]
        ADPCM = 0x10007,
        #[constant = libspa_sys::SPA_MEDIA_SUBTYPE_g723]
        G723 = 0x10008,
        #[constant = libspa_sys::SPA_MEDIA_SUBTYPE_g726]
        G726 = 0x10009,
        #[constant = libspa_sys::SPA_MEDIA_SUBTYPE_g729]
        G729 = 0x1000a,
        #[constant = libspa_sys::SPA_MEDIA_SUBTYPE_amr]
        AMR = 0x1000b,
        #[constant = libspa_sys::SPA_MEDIA_SUBTYPE_gsm]
        GSM = 0x1000c,
        #[constant = libspa_sys::SPA_MEDIA_SUBTYPE_alac]
        ALAC = 0x1000d,
        #[constant = libspa_sys::SPA_MEDIA_SUBTYPE_flac]
        FLAC = 0x1000e,
        #[constant = libspa_sys::SPA_MEDIA_SUBTYPE_ape]
        APE = 0x1000f,
        #[constant = libspa_sys::SPA_MEDIA_SUBTYPE_opus]
        OPUS = 0x10010,
        #[constant = libspa_sys::SPA_MEDIA_SUBTYPE_h264]
        H264 = 0x20001,
        #[constant = libspa_sys::SPA_MEDIA_SUBTYPE_mjpg]
        MJPG = 0x20002,
        #[constant = libspa_sys::SPA_MEDIA_SUBTYPE_dv]
        DV = 0x20003,
        #[constant = libspa_sys::SPA_MEDIA_SUBTYPE_mpegts]
        MPEGTS = 0x20004,
        #[constant = libspa_sys::SPA_MEDIA_SUBTYPE_h263]
        H263 = 0x20005,
        #[constant = libspa_sys::SPA_MEDIA_SUBTYPE_mpeg1]
        MPEG1 = 0x20006,
        #[constant = libspa_sys::SPA_MEDIA_SUBTYPE_mpeg2]
        MPEG2 = 0x20007,
        #[constant = libspa_sys::SPA_MEDIA_SUBTYPE_mpeg4]
        MPEG4 = 0x20008,
        #[constant = libspa_sys::SPA_MEDIA_SUBTYPE_xvid]
        XVID = 0x20009,
        #[constant = libspa_sys::SPA_MEDIA_SUBTYPE_vc1]
        VC1 = 0x2000a,
        #[constant = libspa_sys::SPA_MEDIA_SUBTYPE_vp8]
        VP8 = 0x2000b,
        #[constant = libspa_sys::SPA_MEDIA_SUBTYPE_vp9]
        VP9 = 0x2000c,
        #[constant = libspa_sys::SPA_MEDIA_SUBTYPE_bayer]
        BAYER = 0x2000d,
        #[constant = libspa_sys::SPA_MEDIA_SUBTYPE_jpeg]
        JPEG = 0x30001,
        #[constant = libspa_sys::SPA_MEDIA_SUBTYPE_START_Binary]
        START_BINARY = 0x40000,
        #[constant = libspa_sys::SPA_MEDIA_SUBTYPE_START_Stream]
        START_STREAM = 0x50000,
        #[constant = libspa_sys::SPA_MEDIA_SUBTYPE_midi]
        MIDI = 0x50001,
        #[constant = libspa_sys::SPA_MEDIA_SUBTYPE_START_Application]
        START_APPLICATION = 0x60000,
        #[constant = libspa_sys::SPA_MEDIA_SUBTYPE_control]
        CONTROL = 0x60001,
    }

    /// These correspond to the values of `SPA_TYPE_OBJECT_*`.
    #[example = FORMAT]
    #[module = protocol::id]
    pub struct ObjectType {
        UNKNOWN,
        #[constant = libspa_sys::SPA_TYPE_OBJECT_PropInfo]
        PROP_INFO = 0x40001,
        #[constant = libspa_sys::SPA_TYPE_OBJECT_Props]
        PROPS = 0x40002,
        #[constant = libspa_sys::SPA_TYPE_OBJECT_Format]
        FORMAT = 0x40003,
        #[constant = libspa_sys::SPA_TYPE_OBJECT_ParamBuffers]
        PARAM_BUFFERS = 0x40004,
        #[constant = libspa_sys::SPA_TYPE_OBJECT_ParamMeta]
        PARAM_META = 0x40005,
        #[constant = libspa_sys::SPA_TYPE_OBJECT_ParamIO]
        PARAM_IO = 0x40006,
        #[constant = libspa_sys::SPA_TYPE_OBJECT_ParamProfile]
        PARAM_PROFILE = 0x40007,
        #[constant = libspa_sys::SPA_TYPE_OBJECT_ParamPortConfig]
        PARAM_PORT_CONFIG = 0x40008,
        #[constant = libspa_sys::SPA_TYPE_OBJECT_ParamRoute]
        PARAM_ROUTE = 0x40009,
        #[constant = libspa_sys::SPA_TYPE_OBJECT_Profiler]
        PROFILER = 0x4000a,
        #[constant = libspa_sys::SPA_TYPE_OBJECT_ParamLatency]
        PARAM_LATENCY = 0x4000b,
        #[constant = libspa_sys::SPA_TYPE_OBJECT_ParamProcessLatency]
        PARAM_PROCESS_LATENCY = 0x4000c,
        #[constant = libspa_sys::SPA_TYPE_OBJECT_ParamTag]
        PARAM_TAG = 0x4000d,
    }

    #[example = FREQUENCY]
    #[module = protocol::id]
    pub struct Prop {
        UNKNOWN,
        #[constant = libspa_sys::SPA_PROP_START_Device]
        START_DEVICE = 0x100,
        #[constant = libspa_sys::SPA_PROP_device]
        DEVICE = 0x101,
        #[constant = libspa_sys::SPA_PROP_deviceName]
        DEVICE_NAME = 0x102,
        #[constant = libspa_sys::SPA_PROP_deviceFd]
        DEVICE_FD = 0x103,
        #[constant = libspa_sys::SPA_PROP_card]
        CARD = 0x104,
        #[constant = libspa_sys::SPA_PROP_cardName]
        CARD_NAME = 0x105,
        #[constant = libspa_sys::SPA_PROP_minLatency]
        MIN_LATENCY = 0x106,
        #[constant = libspa_sys::SPA_PROP_maxLatency]
        MAX_LATENCY = 0x107,
        #[constant = libspa_sys::SPA_PROP_periods]
        PERIODS = 0x108,
        #[constant = libspa_sys::SPA_PROP_periodSize]
        PERIOD_SIZE = 0x109,
        #[constant = libspa_sys::SPA_PROP_periodEvent]
        PERIOD_EVENT = 0x10a,
        #[constant = libspa_sys::SPA_PROP_live]
        LIVE = 0x10b,
        #[constant = libspa_sys::SPA_PROP_rate]
        RATE = 0x10c,
        #[constant = libspa_sys::SPA_PROP_quality]
        QUALITY = 0x10d,
        #[constant = libspa_sys::SPA_PROP_bluetoothAudioCodec]
        BLUETOOTH_AUDIO_CODEC = 0x10e,
        #[constant = libspa_sys::SPA_PROP_bluetoothOffloadActive]
        BLUETOOTH_OFFLOAD_ACTIVE = 0x10f,
        START_AUDIO = 0x10000,
        #[constant = libspa_sys::SPA_PROP_waveType]
        WAVE_TYPE = 0x10001,
        #[constant = libspa_sys::SPA_PROP_frequency]
        FREQUENCY = 0x10002,
        /// A volume (Float), 0.0 silence, 1.0 no attenutation.
        #[constant = libspa_sys::SPA_PROP_volume]
        VOLUME = 0x10003,
        /// Mute (Bool)
        #[constant = libspa_sys::SPA_PROP_mute]
        MUTE = 0x10004,
        #[constant = libspa_sys::SPA_PROP_patternType]
        PATTERN_TYPE = 0x10005,
        #[constant = libspa_sys::SPA_PROP_ditherType]
        DITHER_TYPE = 0x10006,
        #[constant = libspa_sys::SPA_PROP_truncate]
        TRUNCATE = 0x10007,
        /// A volume array, one (linear) volume per channel (Array of Float).
        /// 0.0 is silence, 1.0 is without attenuation. This is the effective
        /// volume that is applied. It can result in a hardware volume and
        /// software volume (see softVolumes)
        #[constant = libspa_sys::SPA_PROP_channelVolumes]
        CHANNEL_VOLUMES = 0x10008,
        /// A volume base (Float)
        #[constant = libspa_sys::SPA_PROP_volumeBase]
        VOLUME_BASE = 0x10009,
        /// A volume step (Float)
        #[constant = libspa_sys::SPA_PROP_volumeStep]
        VOLUME_STEP = 0x1000a,
        /// A channelmap array (Array (Id enum spa_audio_channel)).
        #[constant = libspa_sys::SPA_PROP_channelMap]
        CHANNEL_MAP = 0x1000b,
        /// Mute (Bool)
        #[constant = libspa_sys::SPA_PROP_monitorMute]
        MONITOR_MUTE = 0x1000c,
        /// A volume array, one (linear) volume per channel (Array of Float).
        #[constant = libspa_sys::SPA_PROP_monitorVolumes]
        MONITOR_VOLUMES = 0x1000d,
        /// Delay adjustment.
        #[constant = libspa_sys::SPA_PROP_latencyOffsetNsec]
        LATENCY_OFFSET_NSEC = 0x1000e,
        /// Mute (Bool) applied in software.
        #[constant = libspa_sys::SPA_PROP_softMute]
        SOFT_MUTE = 0x1000f,
        /// A volume array, one (linear) volume per channel
        /// (Array of Float). 0.0 is silence, 1.0 is without
        /// attenuation. This is the volume applied in
        /// software, there might be a part applied in
        /// hardware.
        #[constant = libspa_sys::SPA_PROP_softVolumes]
        SOFT_VOLUMES = 0x10010,
        /// Enabled IEC958 (S/PDIF) codecs (Array (Id enum spa_audio_iec958_codec).
        #[constant = libspa_sys::SPA_PROP_iec958Codecs]
        IEC958_CODECS = 0x10011,
        /// Samples to ramp the volume over.
        #[constant = libspa_sys::SPA_PROP_volumeRampSamples]
        VOLUME_RAMP_SAMPLES = 0x10012,
        /// Step or incremental Samples to ramp the volume over.
        #[constant = libspa_sys::SPA_PROP_volumeRampStepSamples]
        VOLUME_RAMP_STEP_SAMPLES = 0x10013,
        /// Time in millisec to ramp the volume over.
        #[constant = libspa_sys::SPA_PROP_volumeRampTime]
        VOLUME_RAMP_TIME = 0x10014,
        /// Step or incremental Time in nano seconds to ramp the.
        #[constant = libspa_sys::SPA_PROP_volumeRampStepTime]
        VOLUME_RAMP_STEP_TIME = 0x10015,
        /// The scale or graph to used to ramp the volume.
        #[constant = libspa_sys::SPA_PROP_volumeRampScale]
        VOLUME_RAMP_SCALE = 0x10016,
        /// Video related properties.
        #[constant = libspa_sys::SPA_PROP_brightness]
        BRIGHTNESS = 0x20001,
        #[constant = libspa_sys::SPA_PROP_contrast]
        CONTRAST = 0x20002,
        #[constant = libspa_sys::SPA_PROP_saturation]
        SATURATION = 0x20003,
        #[constant = libspa_sys::SPA_PROP_hue]
        HUE = 0x20004,
        #[constant = libspa_sys::SPA_PROP_gamma]
        GAMMA = 0x20005,
        #[constant = libspa_sys::SPA_PROP_exposure]
        EXPOSURE = 0x20006,
        #[constant = libspa_sys::SPA_PROP_gain]
        GAIN = 0x20007,
        #[constant = libspa_sys::SPA_PROP_sharpness]
        SHARPNESS = 0x20008,
        /// Other properties.
        /// simple control params (Struct((String: key, Pod: value)*)).
        #[constant = libspa_sys::SPA_PROP_params]
        PARAMS = 0x80001,
    }

    /// Different IO area types.
    ///
    /// Represents `enum spa_io_type`.
    #[example = BUFFERS]
    #[module = protocol::id]
    pub struct IoType {
        UNKNOWN,
        /// Area to exchange buffers, `struct spa_io_buffers`.
        ///
        #[constant = libspa_sys::SPA_IO_Buffers]
        BUFFERS = 1,
        /// Expected byte range, `struct spa_io_range` (currently not used in
        /// PipeWire).
        #[constant = libspa_sys::SPA_IO_Range]
        RANGE = 2,
        /// Area to update clock information, `struct spa_io_clock`.
        #[constant = libspa_sys::SPA_IO_Clock]
        CLOCK = 3,
        /// Latency reporting, `struct spa_io_latency` (currently not used in
        /// PipeWire). See `spa_param_latency`.
        #[constant = libspa_sys::SPA_IO_Latency]
        LATENCY = 4,
        /// Area for control messages, `struct spa_io_sequence`.
        #[constant = libspa_sys::SPA_IO_Control]
        CONTROL = 5,
        /// Area for notify messages, `struct spa_io_sequence`.
        #[constant = libspa_sys::SPA_IO_Notify]
        NOTIFY = 6,
        /// Position information in the graph, `struct spa_io_position`.
        #[constant = libspa_sys::SPA_IO_Position]
        POSITION = 7,
        /// Rate matching between nodes, `struct spa_io_rate_match`.
        #[constant = libspa_sys::SPA_IO_RateMatch]
        RATE_MATCH = 8,
        /// Memory pointer, `struct spa_io_memory` (currently not used in
        /// PipeWire).
        #[constant = libspa_sys::SPA_IO_Memory]
        MEMORY = 9,
        /// Async area to exchange buffers, `struct spa_io_async_buffers`.
        #[constant = libspa_sys::SPA_IO_AsyncBuffers]
        ASYNC_BUFFERS = 10,
    }

    /// Properties for audio `SPA_TYPE_OBJECT_Format`.
    ///
    /// Represents `enum spa_format`.
    #[example = MEDIA_TYPE]
    #[module = protocol::id]
    pub struct FormatKey {
        UNKNOWN,
        /// media type (Id enum spa_media_type).
        #[constant = libspa_sys::SPA_FORMAT_mediaType]
        MEDIA_TYPE = 1,
        /// media subtype (Id enum spa_media_subtype).
        #[constant = libspa_sys::SPA_FORMAT_mediaSubtype]
        MEDIA_SUB_TYPE = 2,
        /// audio format, (Id enum spa_audio_format).
        #[constant = libspa_sys::SPA_FORMAT_AUDIO_format]
        AUDIO_FORMAT = 0x10001,
        /// optional flags (Int).
        #[constant = libspa_sys::SPA_FORMAT_AUDIO_flags]
        AUDIO_FLAGS = 0x10002,
        /// sample rate (Int).
        #[constant = libspa_sys::SPA_FORMAT_AUDIO_rate]
        AUDIO_RATE = 0x10003,
        /// number of audio channels (Int).
        #[constant = libspa_sys::SPA_FORMAT_AUDIO_channels]
        AUDIO_CHANNELS = 0x10004,
        /// channel positions (Id enum spa_audio_position).
        #[constant = libspa_sys::SPA_FORMAT_AUDIO_position]
        AUDIO_POSITION = 0x10005,
        /// codec used (IEC958) (Id enum spa_audio_iec958_codec).
        #[constant = libspa_sys::SPA_FORMAT_AUDIO_iec958Codec]
        AUDIO_IEC958_CODEC = 0x10006,
        /// bit order (Id enum spa_param_bitorder).
        #[constant = libspa_sys::SPA_FORMAT_AUDIO_bitorder]
        AUDIO_BITORDER = 0x10007,
        /// Interleave bytes (Int).
        #[constant = libspa_sys::SPA_FORMAT_AUDIO_interleave]
        AUDIO_INTERLEAVE = 0x10008,
        /// bit rate (Int).
        #[constant = libspa_sys::SPA_FORMAT_AUDIO_bitrate]
        AUDIO_BITRATE = 0x10009,
        /// audio data block alignment (Int).
        #[constant = libspa_sys::SPA_FORMAT_AUDIO_blockAlign]
        AUDIO_BLOCK_ALIGN = 0x1000a,
        /// AAC stream format, (Id enum spa_audio_aac_stream_format).
        #[constant = libspa_sys::SPA_FORMAT_AUDIO_AAC_streamFormat]
        AUDIO_AAC_STREAM_FORMAT = 0x1000b,
        /// WMA profile (Id enum spa_audio_wma_profile).
        #[constant = libspa_sys::SPA_FORMAT_AUDIO_WMA_profile]
        AUDIO_WMA_PROFILE = 0x1000c,
        /// AMR band mode (Id enum spa_audio_amr_band_mode).
        #[constant = libspa_sys::SPA_FORMAT_AUDIO_AMR_bandMode]
        AUDIO_AMR_BAND_MODE = 0x1000d,
        /// video format (Id enum spa_video_format).
        #[constant = libspa_sys::SPA_FORMAT_VIDEO_format]
        VIDEO_FORMAT = 0x20001,
        /// format modifier (Long) use only with DMA-BUF and omit for other buffer types.
        #[constant = libspa_sys::SPA_FORMAT_VIDEO_modifier]
        VIDEO_MODIFIER = 0x20002,
        /// size (Rectangle).
        #[constant = libspa_sys::SPA_FORMAT_VIDEO_size]
        VIDEO_SIZE = 0x20003,
        /// frame rate (Fraction).
        #[constant = libspa_sys::SPA_FORMAT_VIDEO_framerate]
        VIDEO_FRAMERATE = 0x20004,
        /// maximum frame rate (Fraction).
        #[constant = libspa_sys::SPA_FORMAT_VIDEO_maxFramerate]
        VIDEO_MAX_FRAMERATE = 0x20005,
        /// number of views (Int).
        #[constant = libspa_sys::SPA_FORMAT_VIDEO_views]
        VIDEO_VIEWS = 0x20006,
        /// (Id enum spa_video_interlace_mode).
        #[constant = libspa_sys::SPA_FORMAT_VIDEO_interlaceMode]
        VIDEO_INTERLACE_MODE = 0x20007,
        /// (Rectangle).
        #[constant = libspa_sys::SPA_FORMAT_VIDEO_pixelAspectRatio]
        VIDEO_PIXEL_ASPECT_RATIO = 0x20008,
        /// (Id enum spa_video_multiview_mode).
        #[constant = libspa_sys::SPA_FORMAT_VIDEO_multiviewMode]
        VIDEO_MULTIVIEW_MODE = 0x20009,
        /// (Id enum spa_video_multiview_flags).
        #[constant = libspa_sys::SPA_FORMAT_VIDEO_multiviewFlags]
        VIDEO_MULTIVIEW_FLAGS = 0x2000a,
        /// /Id enum spa_video_chroma_site).
        #[constant = libspa_sys::SPA_FORMAT_VIDEO_chromaSite]
        VIDEO_CHROMA_SITE = 0x2000b,
        /// /Id enum spa_video_color_range).
        #[constant = libspa_sys::SPA_FORMAT_VIDEO_colorRange]
        VIDEO_COLOR_RANGE = 0x2000c,
        /// /Id enum spa_video_color_matrix).
        #[constant = libspa_sys::SPA_FORMAT_VIDEO_colorMatrix]
        VIDEO_COLOR_MATRIX = 0x2000d,
        /// /Id enum spa_video_transfer_function).
        #[constant = libspa_sys::SPA_FORMAT_VIDEO_transferFunction]
        VIDEO_TRANSFER_FUNCTION = 0x2000e,
        /// /Id enum spa_video_color_primaries).
        #[constant = libspa_sys::SPA_FORMAT_VIDEO_colorPrimaries]
        VIDEO_COLOR_PRIMARIES = 0x2000f,
        /// (Int).
        #[constant = libspa_sys::SPA_FORMAT_VIDEO_profile]
        VIDEO_PROFILE = 0x20010,
        /// (Int).
        #[constant = libspa_sys::SPA_FORMAT_VIDEO_level]
        VIDEO_LEVEL = 0x20011,
        /// (Id enum spa_h264_stream_format).
        #[constant = libspa_sys::SPA_FORMAT_VIDEO_H264_streamFormat]
        VIDEO_H264_STREAM_FORMAT = 0x20012,
        /// (Id enum spa_h264_alignment).
        #[constant = libspa_sys::SPA_FORMAT_VIDEO_H264_alignment]
        VIDEO_H264_ALIGNMENT = 0x20013,
        /// possible control types (flags choice Int, mask of enum spa_control_type).
        #[constant = libspa_sys::SPA_FORMAT_CONTROL_types]
        CONTROL_TYPES = 0x60001,
    }

    #[example = S16]
    #[module = protocol::id]
    pub struct AudioFormat {
        UNKNOWN,
        #[constant = libspa_sys::SPA_AUDIO_FORMAT_ENCODED]
        ENCODED = 1,
        #[constant = libspa_sys::SPA_AUDIO_FORMAT_S8]
        S8 = 0x101,
        #[constant = libspa_sys::SPA_AUDIO_FORMAT_U8]
        U8 = 0x102,
        #[constant = libspa_sys::SPA_AUDIO_FORMAT_S16_LE]
        S16_LE = 0x103,
        #[constant = libspa_sys::SPA_AUDIO_FORMAT_S16_BE]
        S16_BE = 0x104,
        #[constant = libspa_sys::SPA_AUDIO_FORMAT_U16_LE]
        U16_LE = 0x105,
        #[constant = libspa_sys::SPA_AUDIO_FORMAT_U16_BE]
        U16_BE = 0x106,
        #[constant = libspa_sys::SPA_AUDIO_FORMAT_S24_32_LE]
        S24_32_LE = 0x107,
        #[constant = libspa_sys::SPA_AUDIO_FORMAT_S24_32_BE]
        S24_32_BE = 0x108,
        #[constant = libspa_sys::SPA_AUDIO_FORMAT_U24_32_LE]
        U24_32_LE = 0x109,
        #[constant = libspa_sys::SPA_AUDIO_FORMAT_U24_32_BE]
        U24_32_BE = 0x10a,
        #[constant = libspa_sys::SPA_AUDIO_FORMAT_S32_LE]
        S32_LE = 0x10b,
        #[constant = libspa_sys::SPA_AUDIO_FORMAT_S32_BE]
        S32_BE = 0x10c,
        #[constant = libspa_sys::SPA_AUDIO_FORMAT_U32_LE]
        U32_LE = 0x10d,
        #[constant = libspa_sys::SPA_AUDIO_FORMAT_U32_BE]
        U32_BE = 0x10e,
        #[constant = libspa_sys::SPA_AUDIO_FORMAT_S24_LE]
        S24_LE = 0x10f,
        #[constant = libspa_sys::SPA_AUDIO_FORMAT_S24_BE]
        S24_BE = 0x110,
        #[constant = libspa_sys::SPA_AUDIO_FORMAT_U24_LE]
        U24_LE = 0x111,
        #[constant = libspa_sys::SPA_AUDIO_FORMAT_U24_BE]
        U24_BE = 0x112,
        #[constant = libspa_sys::SPA_AUDIO_FORMAT_S20_LE]
        S20_LE = 0x113,
        #[constant = libspa_sys::SPA_AUDIO_FORMAT_S20_BE]
        S20_BE = 0x114,
        #[constant = libspa_sys::SPA_AUDIO_FORMAT_U20_LE]
        U20_LE = 0x115,
        #[constant = libspa_sys::SPA_AUDIO_FORMAT_U20_BE]
        U20_BE = 0x116,
        #[constant = libspa_sys::SPA_AUDIO_FORMAT_S18_LE]
        S18_LE = 0x117,
        #[constant = libspa_sys::SPA_AUDIO_FORMAT_S18_BE]
        S18_BE = 0x118,
        #[constant = libspa_sys::SPA_AUDIO_FORMAT_U18_LE]
        U18_LE = 0x119,
        #[constant = libspa_sys::SPA_AUDIO_FORMAT_U18_BE]
        U18_BE = 0x11a,
        #[constant = libspa_sys::SPA_AUDIO_FORMAT_F32_LE]
        F32_LE = 0x11b,
        #[constant = libspa_sys::SPA_AUDIO_FORMAT_F32_BE]
        F32_BE = 0x11c,
        #[constant = libspa_sys::SPA_AUDIO_FORMAT_F64_LE]
        F64_LE = 0x11d,
        #[constant = libspa_sys::SPA_AUDIO_FORMAT_F64_BE]
        F64_BE = 0x11e,
        #[constant = libspa_sys::SPA_AUDIO_FORMAT_ULAW]
        ULAW = 0x11f,
        #[constant = libspa_sys::SPA_AUDIO_FORMAT_ALAW]
        ALAW = 0x120,
        #[constant = libspa_sys::SPA_AUDIO_FORMAT_U8P]
        U8P = 0x201,
        #[constant = libspa_sys::SPA_AUDIO_FORMAT_S16P]
        S16P = 0x202,
        #[constant = libspa_sys::SPA_AUDIO_FORMAT_S24_32P]
        S24_32P = 0x203,
        #[constant = libspa_sys::SPA_AUDIO_FORMAT_S32P]
        S32P = 0x204,
        #[constant = libspa_sys::SPA_AUDIO_FORMAT_S24P]
        S24P = 0x205,
        #[constant = libspa_sys::SPA_AUDIO_FORMAT_F32P]
        F32P = 0x206,
        #[constant = libspa_sys::SPA_AUDIO_FORMAT_F64P]
        F64P = 0x207,
        #[constant = libspa_sys::SPA_AUDIO_FORMAT_S8P]
        S8P = 0x208,
    }

    #[example = SUSPEND]
    #[module = protocol::id]
    pub struct NodeCommand {
        UNKNOWN,
        /// Suspend a node, this removes all configuredformats and closes any
        /// devices.
        #[constant = libspa_sys::SPA_NODE_COMMAND_Suspend]
        SUSPEND = 0,
        /// Pause a node. this makes it stop emitting scheduling events.
        #[constant = libspa_sys::SPA_NODE_COMMAND_Pause]
        PAUSE = 1,
        /// Start a node, this makes it start emitting scheduling events.
        #[constant = libspa_sys::SPA_NODE_COMMAND_Start]
        START = 2,
        #[constant = libspa_sys::SPA_NODE_COMMAND_Enable]
        ENABLE = 3,
        #[constant = libspa_sys::SPA_NODE_COMMAND_Disable]
        DISABLE = 4,
        #[constant = libspa_sys::SPA_NODE_COMMAND_Flush]
        FLUSH = 5,
        #[constant = libspa_sys::SPA_NODE_COMMAND_Drain]
        DRAIN = 6,
        #[constant = libspa_sys::SPA_NODE_COMMAND_Marker]
        MARKER = 7,
        /// Begin a set of parameter enumerations or configuration that require
        /// the device to remain opened, like query formats and then set a
        /// format.
        #[constant = libspa_sys::SPA_NODE_COMMAND_ParamBegin]
        PARAM_BEGIN = 8,
        /// End a transaction.
        #[constant = libspa_sys::SPA_NODE_COMMAND_ParamEnd]
        PARAM_END = 9,
        /// Sent to a driver when some other node emitted the RequestProcess
        /// event.
        #[constant = libspa_sys::SPA_NODE_COMMAND_RequestProcess]
        REQUEST_PROCESS = 10,
    }

    #[example = NODE]
    #[module = protocol::id]
    pub struct CommandType {
        UNKNOWN,
        #[constant = libspa_sys::SPA_TYPE_COMMAND_Device]
        DEVICE = 0x30001,
        #[constant = libspa_sys::SPA_TYPE_COMMAND_Node]
        NODE = 0x30002,
    }

    /// Represents `enum spa_data_type`.
    #[example = MEM_FD]
    #[module = protocol::id]
    pub struct DataType {
        UNKNOWN,
        /// Pointer to memory, the data field in struct spa_data is set.
        #[constant = libspa_sys::SPA_DATA_MemPtr]
        MEM_PTR = 1,
        /// memfd, mmap to get to memory.
        #[constant = libspa_sys::SPA_DATA_MemFd]
        MEM_FD = 2,
        /// fd to dmabuf memory. This might not be readily mappable (unless the
        /// MAPPABLE flag is set) and should normally be handled with DMABUF
        /// apis.
        #[constant = libspa_sys::SPA_DATA_DmaBuf]
        DMA_BUF = 3,
        /// Memory is identified with an id. The actual memory can be obtained
        /// in some other way and can be identified with this id.
        #[constant = libspa_sys::SPA_DATA_MemId]
        MEM_ID = 4,
        /// A syncobj, usually requires a spa_meta_sync_timeline metadata with
        /// timeline points.
        #[constant = libspa_sys::SPA_DATA_SyncObj]
        SYNC_OBJ = 5,
    }

    /// Represents `enum spa_meta_type`.
    #[example = BITMAP]
    #[module = protocol::id]
    pub struct Meta {
        UNKNOWN,
        /// struct spa_meta_header.
        #[constant = libspa_sys::SPA_META_Header]
        HEADER = 1,
        /// struct spa_meta_region with cropping data.
        #[constant = libspa_sys::SPA_META_VideoCrop]
        VIDEO_CROP = 2,
        /// array of struct spa_meta_region with damage, where an invalid entry or end-of-array marks the end.
        #[constant = libspa_sys::SPA_META_VideoDamage]
        VIDEO_DAMAGE = 3,
        /// struct spa_meta_bitmap.
        #[constant = libspa_sys::SPA_META_Bitmap]
        BITMAP = 4,
        /// struct spa_meta_cursor.
        #[constant = libspa_sys::SPA_META_Cursor]
        CURSOR = 5,
        /// metadata contains a spa_meta_control associated with the data.
        #[constant = libspa_sys::SPA_META_Control]
        CONTROL = 6,
        /// don't write to buffer when count > 0.
        #[constant = libspa_sys::SPA_META_Busy]
        BUSY = 7,
        /// struct spa_meta_transform.
        #[constant = libspa_sys::SPA_META_VideoTransform]
        VIDEO_TRANSFORM = 8,
        /// struct spa_meta_sync_timeline.
        #[constant = libspa_sys::SPA_META_SyncTimeline]
        SYNC_TIMELINE = 9,
    }

    /// Equivalent to `enum spa_param_buffers`.
    #[example = DATA_TYPE]
    #[module = protocol::id]
    pub struct ParamBuffersKey {
        UNKNOWN,
        /// Number of buffers (Int).
        #[constant = libspa_sys::SPA_PARAM_BUFFERS_buffers]
        BUFFERS = 1,
        /// Number of data blocks per buffer (Int).
        #[constant = libspa_sys::SPA_PARAM_BUFFERS_blocks]
        BLOCKS = 2,
        /// Size of a data block memory (Int.
        #[constant = libspa_sys::SPA_PARAM_BUFFERS_size]
        SIZE = 3,
        /// Stride of data block memory (Int).
        #[constant = libspa_sys::SPA_PARAM_BUFFERS_stride]
        STRIDE = 4,
        /// Alignment of data block memory (Int).
        #[constant = libspa_sys::SPA_PARAM_BUFFERS_align]
        ALIGN = 5,
        /// Possible memory types (flags choice Int, mask of enum spa_data_type).
        #[constant = libspa_sys::SPA_PARAM_BUFFERS_dataType]
        DATA_TYPE = 6,
        /// Required meta data types (Int, mask of enum spa_meta_type).
        #[constant = libspa_sys::SPA_PARAM_BUFFERS_metaType]
        META_TYPE = 7,
    }

    /// properties for SPA_TYPE_OBJECT_ParamMeta.
    ///
    /// Equivalent to `enum spa_param_meta`.
    #[example = TYPE]
    #[module = protocol::id]
    pub struct ParamMetaKey {
        UNKNOWN,
        /// The type of the parameter, one of enum spa_param_meta (Id enum spa_param_meta).
        #[constant = libspa_sys::SPA_PARAM_META_type]
        TYPE = 1,
        /// The expected maximum size the meta (Int).
        #[constant = libspa_sys::SPA_PARAM_META_size]
        SIZE = 2,
    }

    /// properties for SPA_TYPE_OBJECT_ParamIO
    ///
    /// This corresponds to `enum spa_param_io`.
    #[example = SIZE]
    #[module = protocol::id]
    pub struct ParamIoKey {
        UNKNOWN,
        /// type ID, uniquely identifies the io area (Id enum spa_io_type).
        #[constant = libspa_sys::SPA_PARAM_IO_id]
        ID = 1,
        /// size of the io area (Int).
        #[constant = libspa_sys::SPA_PARAM_IO_size]
        SIZE = 2,
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
