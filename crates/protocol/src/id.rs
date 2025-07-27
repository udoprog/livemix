pod::macros::id! {
    #[example = FORMAT]
    pub enum Param {
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

    #[example = OPUS]
    pub enum MediaSubType {
        UNKNOWN,
        RAW = 1,
        DSP = 2,
        IEC958 = 3,
        DSD = 4,
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

    #[example = FORMAT]
    pub enum ObjectType {
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
    pub enum Prop {
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
}
