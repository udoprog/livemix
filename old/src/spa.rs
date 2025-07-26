use core::cell::UnsafeCell;
use core::ffi::{CStr, c_void};
use core::mem::MaybeUninit;
use core::pin::Pin;
use core::ptr;
use core::slice;

use spa_sys::{
    spa_buffer, spa_chunk, spa_data, spa_dict, spa_dict_item, spa_dict_lookup, spa_hook,
    spa_hook_remove, spa_source,
};

crate::macros::decl_enum! {
    #[repr(u32)]
    pub enum Param {
        Invalid = spa_sys::SPA_PARAM_Invalid,
        PropInfo = spa_sys::SPA_PARAM_PropInfo,
        Props = spa_sys::SPA_PARAM_Props,
        EnumFormat = spa_sys::SPA_PARAM_EnumFormat,
        Format = spa_sys::SPA_PARAM_Format,
        Buffers = spa_sys::SPA_PARAM_Buffers,
        Meta = spa_sys::SPA_PARAM_Meta,
        IO = spa_sys::SPA_PARAM_IO,
        EnumProfile = spa_sys::SPA_PARAM_EnumProfile,
        Profile = spa_sys::SPA_PARAM_Profile,
        EnumPortConfig = spa_sys::SPA_PARAM_EnumPortConfig,
        PortConfig = spa_sys::SPA_PARAM_PortConfig,
        EnumRoute = spa_sys::SPA_PARAM_EnumRoute,
        Route = spa_sys::SPA_PARAM_Route,
        Control = spa_sys::SPA_PARAM_Control,
        Latency = spa_sys::SPA_PARAM_Latency,
        ProcessLatency = spa_sys::SPA_PARAM_ProcessLatency,
        Tag = spa_sys::SPA_PARAM_Tag,
    }

    #[repr(u32)]
    pub enum MediaType {
        Unknown = spa_sys::SPA_MEDIA_TYPE_unknown,
        Audio = spa_sys::SPA_MEDIA_TYPE_audio,
        Video = spa_sys::SPA_MEDIA_TYPE_video,
        Image = spa_sys::SPA_MEDIA_TYPE_image,
        Binary = spa_sys::SPA_MEDIA_TYPE_binary,
        Stream = spa_sys::SPA_MEDIA_TYPE_stream,
        Application = spa_sys::SPA_MEDIA_TYPE_application,
    }

    #[repr(u32)]
    pub enum MediaSubType {
        Unknown = spa_sys::SPA_MEDIA_SUBTYPE_unknown,
        Raw = spa_sys::SPA_MEDIA_SUBTYPE_raw,
        Dsp = spa_sys::SPA_MEDIA_SUBTYPE_dsp,
        Iec958 = spa_sys::SPA_MEDIA_SUBTYPE_iec958,
        Dsd = spa_sys::SPA_MEDIA_SUBTYPE_dsd,
        StartAudio = spa_sys::SPA_MEDIA_SUBTYPE_START_Audio,
        Mp3 = spa_sys::SPA_MEDIA_SUBTYPE_mp3,
        Aac = spa_sys::SPA_MEDIA_SUBTYPE_aac,
        Vorbis = spa_sys::SPA_MEDIA_SUBTYPE_vorbis,
        Wma = spa_sys::SPA_MEDIA_SUBTYPE_wma,
        Ra = spa_sys::SPA_MEDIA_SUBTYPE_ra,
        Sbc = spa_sys::SPA_MEDIA_SUBTYPE_sbc,
        Adpcm = spa_sys::SPA_MEDIA_SUBTYPE_adpcm,
        G723 = spa_sys::SPA_MEDIA_SUBTYPE_g723,
        G726 = spa_sys::SPA_MEDIA_SUBTYPE_g726,
        G729 = spa_sys::SPA_MEDIA_SUBTYPE_g729,
        Amr = spa_sys::SPA_MEDIA_SUBTYPE_amr,
        Gsm = spa_sys::SPA_MEDIA_SUBTYPE_gsm,
        Alac = spa_sys::SPA_MEDIA_SUBTYPE_alac,
        Flac = spa_sys::SPA_MEDIA_SUBTYPE_flac,
        Ape = spa_sys::SPA_MEDIA_SUBTYPE_ape,
        Opus = spa_sys::SPA_MEDIA_SUBTYPE_opus,
        StartVideo = spa_sys::SPA_MEDIA_SUBTYPE_START_Video,
        H264 = spa_sys::SPA_MEDIA_SUBTYPE_h264,
        Mjpg = spa_sys::SPA_MEDIA_SUBTYPE_mjpg,
        Dv = spa_sys::SPA_MEDIA_SUBTYPE_dv,
        Mpegts = spa_sys::SPA_MEDIA_SUBTYPE_mpegts,
        H263 = spa_sys::SPA_MEDIA_SUBTYPE_h263,
        Mpeg1 = spa_sys::SPA_MEDIA_SUBTYPE_mpeg1,
        Mpeg2 = spa_sys::SPA_MEDIA_SUBTYPE_mpeg2,
        Mpeg4 = spa_sys::SPA_MEDIA_SUBTYPE_mpeg4,
        Xvid = spa_sys::SPA_MEDIA_SUBTYPE_xvid,
        Vc1 = spa_sys::SPA_MEDIA_SUBTYPE_vc1,
        Vp8 = spa_sys::SPA_MEDIA_SUBTYPE_vp8,
        Vp9 = spa_sys::SPA_MEDIA_SUBTYPE_vp9,
        Bayer = spa_sys::SPA_MEDIA_SUBTYPE_bayer,
        StartImage = spa_sys::SPA_MEDIA_SUBTYPE_START_Image,
        Jpeg = spa_sys::SPA_MEDIA_SUBTYPE_jpeg,
        StartBinary = spa_sys::SPA_MEDIA_SUBTYPE_START_Binary,
        StartStream = spa_sys::SPA_MEDIA_SUBTYPE_START_Stream,
        Midi = spa_sys::SPA_MEDIA_SUBTYPE_midi,
        StartApplication = spa_sys::SPA_MEDIA_SUBTYPE_START_Application,
        Control = spa_sys::SPA_MEDIA_SUBTYPE_control,
    }

    #[repr(u32)]
    pub enum AudioFormat {
        Unknown = spa_sys::SPA_AUDIO_FORMAT_UNKNOWN,
        Encoded = spa_sys::SPA_AUDIO_FORMAT_ENCODED,
        StartInterleaved = spa_sys::SPA_AUDIO_FORMAT_START_Interleaved,
        S8 = spa_sys::SPA_AUDIO_FORMAT_S8,
        U8 = spa_sys::SPA_AUDIO_FORMAT_U8,
        #[allow(non_camel_case_types)]
        S16_LE = spa_sys::SPA_AUDIO_FORMAT_S16_LE,
        #[allow(non_camel_case_types)]
        S16_BE = spa_sys::SPA_AUDIO_FORMAT_S16_BE,
        #[allow(non_camel_case_types)]
        U16_LE = spa_sys::SPA_AUDIO_FORMAT_U16_LE,
        #[allow(non_camel_case_types)]
        U16_BE = spa_sys::SPA_AUDIO_FORMAT_U16_BE,
        #[allow(non_camel_case_types)]
        S24_32_LE = spa_sys::SPA_AUDIO_FORMAT_S24_32_LE,
        #[allow(non_camel_case_types)]
        S24_32_BE = spa_sys::SPA_AUDIO_FORMAT_S24_32_BE,
        #[allow(non_camel_case_types)]
        U24_32_LE = spa_sys::SPA_AUDIO_FORMAT_U24_32_LE,
        #[allow(non_camel_case_types)]
        U24_32_BE = spa_sys::SPA_AUDIO_FORMAT_U24_32_BE,
        #[allow(non_camel_case_types)]
        S32_LE = spa_sys::SPA_AUDIO_FORMAT_S32_LE,
        #[allow(non_camel_case_types)]
        S32_BE = spa_sys::SPA_AUDIO_FORMAT_S32_BE,
        #[allow(non_camel_case_types)]
        U32_LE = spa_sys::SPA_AUDIO_FORMAT_U32_LE,
        #[allow(non_camel_case_types)]
        U32_BE = spa_sys::SPA_AUDIO_FORMAT_U32_BE,
        #[allow(non_camel_case_types)]
        S24_LE = spa_sys::SPA_AUDIO_FORMAT_S24_LE,
        #[allow(non_camel_case_types)]
        S24_BE = spa_sys::SPA_AUDIO_FORMAT_S24_BE,
        #[allow(non_camel_case_types)]
        U24_LE = spa_sys::SPA_AUDIO_FORMAT_U24_LE,
        #[allow(non_camel_case_types)]
        U24_BE = spa_sys::SPA_AUDIO_FORMAT_U24_BE,
        #[allow(non_camel_case_types)]
        S20_LE = spa_sys::SPA_AUDIO_FORMAT_S20_LE,
        #[allow(non_camel_case_types)]
        S20_BE = spa_sys::SPA_AUDIO_FORMAT_S20_BE,
        #[allow(non_camel_case_types)]
        U20_LE = spa_sys::SPA_AUDIO_FORMAT_U20_LE,
        #[allow(non_camel_case_types)]
        U20_BE = spa_sys::SPA_AUDIO_FORMAT_U20_BE,
        #[allow(non_camel_case_types)]
        S18_LE = spa_sys::SPA_AUDIO_FORMAT_S18_LE,
        #[allow(non_camel_case_types)]
        S18_BE = spa_sys::SPA_AUDIO_FORMAT_S18_BE,
        #[allow(non_camel_case_types)]
        U18_LE = spa_sys::SPA_AUDIO_FORMAT_U18_LE,
        #[allow(non_camel_case_types)]
        U18_BE = spa_sys::SPA_AUDIO_FORMAT_U18_BE,
        #[allow(non_camel_case_types)]
        F32_LE = spa_sys::SPA_AUDIO_FORMAT_F32_LE,
        #[allow(non_camel_case_types)]
        F32_BE = spa_sys::SPA_AUDIO_FORMAT_F32_BE,
        #[allow(non_camel_case_types)]
        F64_LE = spa_sys::SPA_AUDIO_FORMAT_F64_LE,
        #[allow(non_camel_case_types)]
        F64_BE = spa_sys::SPA_AUDIO_FORMAT_F64_BE,
        ULAW = spa_sys::SPA_AUDIO_FORMAT_ULAW,
        ALAW = spa_sys::SPA_AUDIO_FORMAT_ALAW,
        StartPlanar = spa_sys::SPA_AUDIO_FORMAT_START_Planar,
        U8P = spa_sys::SPA_AUDIO_FORMAT_U8P,
        S16P = spa_sys::SPA_AUDIO_FORMAT_S16P,
        S24_32P = spa_sys::SPA_AUDIO_FORMAT_S24_32P,
        S32P = spa_sys::SPA_AUDIO_FORMAT_S32P,
        S24P = spa_sys::SPA_AUDIO_FORMAT_S24P,
        F32P = spa_sys::SPA_AUDIO_FORMAT_F32P,
        F64P = spa_sys::SPA_AUDIO_FORMAT_F64P,
        S8P = spa_sys::SPA_AUDIO_FORMAT_S8P,
        StartOther = spa_sys::SPA_AUDIO_FORMAT_START_Other,
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

/// Parse format.
pub fn format_parse(param: *const spa_sys::spa_pod) -> (MediaType, MediaSubType) {
    let mut media_type = 0;
    let mut media_subtype = 0;

    unsafe {
        spa_sys::spa_format_parse(param, &mut media_type, &mut media_subtype);
    }

    (
        MediaType::from_raw(media_type),
        MediaSubType::from_raw(media_subtype),
    )
}

pub struct Hook {
    data: MaybeUninit<spa_hook>,
    init: bool,
}

impl Hook {
    /// Set up an empty hook.
    pub fn empty() -> Self {
        Self {
            data: MaybeUninit::zeroed(),
            init: false,
        }
    }

    /// Access the underlying hook pointer.
    pub(crate) unsafe fn as_mut_ptr(self: Pin<&mut Self>) -> *mut spa_hook {
        unsafe { self.get_unchecked_mut().data.as_mut_ptr() }
    }

    pub(crate) unsafe fn assume_init(self: Pin<&mut Self>) {
        // Will only touch init.
        let this = unsafe { self.get_unchecked_mut() };
        this.init = true;
    }

    /// Remove the current hook.
    pub fn remove(self: Pin<&mut Self>) {
        // Will only touch init.
        let this = unsafe { self.get_unchecked_mut() };

        if this.init {
            unsafe {
                spa_hook_remove(this.data.as_ptr().cast_mut());
            }

            this.init = false;
        }
    }
}

#[repr(transparent)]
pub struct Dict {
    ptr: UnsafeCell<spa_dict>,
}

impl Dict {
    #[inline]
    pub unsafe fn new(ptr: *const spa_dict) -> &'static Self {
        debug_assert!(!ptr.is_null(), "spa_dict pointer cannot be null");
        unsafe { &*(ptr as *const Self) }
    }

    #[inline]
    fn as_ptr(&self) -> *const spa_dict {
        self.ptr.get().cast_const()
    }

    fn as_raw_items(&self) -> &[spa_dict_item] {
        unsafe {
            let ptr = &*self.ptr.get();
            slice::from_raw_parts(ptr.items, ptr.n_items as usize)
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        unsafe { ptr::addr_of!((*self.ptr.get()).n_items).read() as usize }
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = (&CStr, &CStr)> {
        let items = self.as_raw_items();

        items.iter().map(|item| {
            let key = unsafe { CStr::from_ptr(item.key) };
            let value = unsafe { CStr::from_ptr(item.value) };
            (key, value)
        })
    }

    #[inline]
    pub fn lookup(&self, key: &CStr) -> Option<&CStr> {
        unsafe {
            let value = spa_dict_lookup(self.as_ptr(), key.as_ptr());

            if value.is_null() {
                None
            } else {
                Some(CStr::from_ptr(value))
            }
        }
    }
}

#[repr(transparent)]
pub struct Buffer {
    ptr: UnsafeCell<spa_buffer>,
}

impl Buffer {
    #[inline]
    pub(crate) unsafe fn new(ptr: *mut spa_buffer) -> &'static mut Self {
        debug_assert!(!ptr.is_null(), "spa_buffer: pointer cannot be null");
        unsafe { &mut *(ptr as *mut Self) }
    }

    #[inline]
    pub fn datas(&mut self) -> &mut [Data] {
        unsafe {
            let ptr = self.ptr.get();
            slice::from_raw_parts_mut((*ptr).datas.cast(), (*ptr).n_datas as usize)
        }
    }
}

#[repr(transparent)]
pub struct Data {
    ptr: UnsafeCell<spa_data>,
}

impl Data {
    /// Get the max size of the data buffer.
    pub fn max_size(&self) -> u32 {
        unsafe { ptr::addr_of!((*self.ptr.get()).maxsize).read() }
    }

    /// Access the underlying raw data pointer.
    #[inline]
    pub fn data_ptr(&self) -> *mut c_void {
        unsafe { ptr::addr_of!((*self.ptr.get()).data).read() }
    }

    /// Access the chunk of the data buffer mutably.
    #[inline]
    pub fn chunk_mut(&mut self) -> &mut Chunk {
        unsafe {
            let ptr = ptr::addr_of!((*self.ptr.get()).chunk).read();
            Chunk::new(ptr)
        }
    }
}

#[repr(transparent)]
pub struct Chunk {
    ptr: UnsafeCell<spa_chunk>,
}

impl Chunk {
    #[inline]
    unsafe fn new<'a>(ptr: *mut spa_chunk) -> &'a mut Self {
        debug_assert!(!ptr.is_null(), "spa_chunk_ pointer cannot be null");
        unsafe { &mut *(ptr as *mut Self) }
    }

    #[inline]
    pub fn offset_mut(&mut self) -> &mut u32 {
        unsafe { &mut *ptr::addr_of_mut!((*self.ptr.get()).offset) }
    }

    #[inline]
    pub fn stride_mut(&mut self) -> &mut i32 {
        unsafe { &mut *ptr::addr_of_mut!((*self.ptr.get()).stride) }
    }

    #[inline]
    pub fn size_mut(&mut self) -> &mut u32 {
        unsafe { &mut *ptr::addr_of_mut!((*self.ptr.get()).size) }
    }
}

#[repr(transparent)]
pub struct Source {
    ptr: UnsafeCell<spa_source>,
}

impl Source {
    #[inline]
    pub(crate) unsafe fn new(ptr: *mut spa_source) -> &'static Self {
        debug_assert!(!ptr.is_null(), "spa_source: pointer cannot be null");
        unsafe { &*(ptr as *mut Self) }
    }

    #[inline]
    pub(crate) fn as_mut_ptr(&self) -> *mut spa_source {
        self.ptr.get()
    }
}
