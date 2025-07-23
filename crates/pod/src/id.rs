macro_rules! declare_enum {
    ($ty_vis:vis enum $ty:ident {
        $default:ident = $default_value:expr
        $(, $field:ident = $field_value:expr)* $(,)?
    }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        #[repr(u32)]
        $ty_vis enum $ty {
            $default = $default_value,
            $($field = $field_value,)*
        }

        impl self::sealed::Sealed for $ty {}

        impl IntoId for $ty {
            #[inline]
            fn into_id(self) -> u32 {
                self as u32
            }

            #[inline]
            fn from_id(value: u32) -> Self {
                match value {
                    $($field_value => Self::$field,)*
                    _ => Self::$default,
                }
            }
        }
    };
}

declare_enum! {
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
}

mod sealed {
    pub trait Sealed {}
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Id<T>(pub T);
