use core::cell::UnsafeCell;
use core::f32::consts::PI;
use core::ffi::{CStr, c_char, c_int, c_void};
use core::mem;
use core::mem::MaybeUninit;
use core::pin::pin;
use core::ptr;
use core::ptr::NonNull;
use std::pin::{self, Pin};

use pw_sys::pw_stream_state;
use spa_sys::spa_pod;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::pw;
use crate::spa;

const M_PI_M2: f32 = PI * PI;
const DEFAULT_RATE: u32 = 48000;
const DEFAULT_CHANNELS: u32 = 2;
const DEFAULT_VOLUME: f32 = 0.1;

struct EventGlobalData {}

struct RoundTripData {
    pending: c_int,
}

struct PlaybackData {
    stream: &'static pw::Stream,
    count: u32,
    accumulator: f32,
}

struct CaptureData {
    lp: &'static pw::Loop,
    stream: &'static pw::Stream,
    first: bool,
}

unsafe extern "C" fn registry_event_global(
    data: *mut c_void,
    id: u32,
    _permissions: u32,
    ty: *const c_char,
    version: u32,
    props: *const spa_sys::spa_dict,
) {
    let _data = unsafe { &*data.cast::<EventGlobalData>() };
    let props = unsafe { spa::Dict::new(props) };
    // tracing::info!("here");
    let ty = unsafe { CStr::from_ptr(ty) };
    let ty = ty.to_string_lossy();
    tracing::info!("object: id:{id} type:{ty}/{version}");

    for (key, value) in props.iter() {
        let key = key.to_string_lossy();
        let value = value.to_string_lossy();
        tracing::info!("  {key}: {value}");
    }
}

unsafe extern "C" fn registry_event_global_remove(_: *mut c_void, id: u32) {
    tracing::info!("object removed: id:{id}");
}

unsafe extern "C" fn on_done(data: *mut c_void, id: u32, seq: c_int) {
    let data = unsafe { &*data.cast::<RoundTripData>() };

    if id == pw_sys::PW_ID_CORE && seq == data.pending {
        tracing::info!("init done");
    }
}

unsafe extern "C" fn on_process_playback(data: *mut c_void) {
    let data = unsafe { &mut *data.cast::<PlaybackData>() };

    if data.count == 0 {
        tracing::info!(
            "{:?}: playback: called: {}",
            std::thread::current().id(),
            data.count
        );
    }

    data.count += 1;

    let Some(b) = data.stream.dequeue_buffer() else {
        tracing::info!("out of buffers");
        return;
    };

    let buf = b.buffer();

    let datas = buf.datas();

    let [buf_data] = datas else {
        return;
    };

    let mut dst = buf_data.data_ptr().cast::<i16>();

    let stride = (mem::size_of::<u16>() as u32) * DEFAULT_CHANNELS;

    let mut n_frames = buf_data.max_size() / stride;

    if b.requested() > 0 {
        n_frames = (b.requested() as u32).min(n_frames);
    }

    for _ in 0..n_frames {
        data.accumulator += M_PI_M2 * 440.0 / (DEFAULT_RATE as f32);

        if data.accumulator >= M_PI_M2 {
            data.accumulator -= M_PI_M2;
        }

        /* sin() gives a value between -1.0 and 1.0, we first apply
         * the volume and then scale with 32767.0 to get a 16 bits value
         * between [-32767 32767].
         * Another common method to convert a double to
         * 16 bits is to multiple by 32768.0 and then clamp to
         * [-32768 32767] to get the full 16 bits range. */
        let val = (data.accumulator.sin() * DEFAULT_VOLUME * 32767.0) as i16;

        for _ in 0..DEFAULT_CHANNELS {
            unsafe {
                dst.write(val);
            }
            dst = dst.wrapping_add(1);
        }
    }

    let chunk = buf_data.chunk_mut();

    *chunk.offset_mut() = 0;
    *chunk.stride_mut() = stride as i32;
    *chunk.size_mut() = n_frames * stride;

    data.stream.queue_buffer(b);
}

unsafe extern "C" fn capture_param_changed(
    data: *mut ::std::os::raw::c_void,
    id: u32,
    param: *const spa_pod,
) {
    let id = spa::Param::from_raw(id);

    let data = unsafe { &mut *data.cast::<CaptureData>() };
    tracing::info!(
        "{:?}: capture: param {id:?} changed",
        std::thread::current().id()
    );
    data.first = true;

    match id {
        spa::Param::Tag => {}
        spa::Param::Format => unsafe {
            let (media_type, media_sub_type) = spa::format_parse(param);
            tracing::info!("{media_type:?} / {media_sub_type:?}");

            if media_type != spa::MediaType::Audio {
                tracing::info!("not an audio format");
                return;
            }

            match media_sub_type {
                spa::MediaSubType::Raw => {
                    let mut info = MaybeUninit::<spa_sys::spa_audio_info_raw>::zeroed();
                    spa_sys::spa_format_audio_raw_parse(param, info.as_mut_ptr());
                    let info = info.assume_init();
                    let format = spa::AudioFormat::from_raw(info.format);
                    dbg!(format, info.channels);
                    tracing::info!("raw audio format");
                }
                _ => {
                    return;
                }
            }
        },
        _ => {}
    }
}

unsafe extern "C" fn capture_state_changed(
    data: *mut c_void,
    old: pw_stream_state,
    state: pw_stream_state,
    error: *const c_char,
) {
    let old = pw::StreamState::from_raw(old);
    let state = pw::StreamState::from_raw(state);
    tracing::info!("capture: {old:?} -> {state:?}");
}

unsafe extern "C" fn capture_on_process(data: *mut c_void) {
    let data = unsafe { &mut *data.cast::<CaptureData>() };

    if data.first {
        tracing::info!("{:?}: capture: first process", std::thread::current().id());
        data.first = false;
    }

    let Some(b) = data.stream.dequeue_buffer() else {
        tracing::info!("out of buffers");
        return;
    };

    data.stream.queue_buffer(b);
}

#[derive(Clone)]
pub struct Handle {
    lp: &'static pw::Loop,
    tx: UnboundedSender<Task>,
    rx_event: &'static spa::Source,
    shutdown: &'static spa::Source,
}

impl Handle {
    /// Send a task to the mixer.
    pub fn send(&self, task: Task) {
        if self.tx.send(task).is_ok() {
            self.lp.signal_event(self.rx_event);
        }
    }

    /// Shut down main loop.
    pub fn shutdown(&self) {
        self.lp.signal_event(self.shutdown);
    }
}

unsafe impl Send for Handle {}
unsafe impl Sync for Handle {}

pub struct Mixer {
    main_loop: &'static pw::MainLoop,
    rx_event: &'static spa::Source,
    rx: UnboundedReceiver<Task>,
    rx_context: *mut RxIndirect,
}

unsafe impl Send for Mixer {}

/// A task sent to the mixer.
#[derive(Debug)]
pub enum Task {
    AddPlaybackStream,
    AddCaptureStream,
}

struct RxData {
    rx: UnboundedReceiver<Task>,
    core: &'static pw::Core,
    state: NonNull<MixerState>,
}

fn on_rx(data: *mut c_void, count: u64) {
    let data = unsafe { &mut *(data.cast::<RxData>()) };
    let state = unsafe { data.state.as_mut() };

    while let Ok(task) = data.rx.try_recv() {
        tracing::info!("{task:?}");

        match task {
            Task::AddPlaybackStream => {
                let stream = data
                    .core
                    .new_stream(c"audio-playback", pw::StreamKind::AudioPlayback);

                let stream_data = Box::new(PlaybackData {
                    stream,
                    count: 0,
                    accumulator: 0.0,
                });

                let mut stream_listener = Box::pin(spa::Hook::empty());
                let events = Box::pin(pw::StreamEvents::new().process(on_process_playback));

                unsafe {
                    stream.add_listener(stream_listener.as_mut(), events.as_ref(), &*stream_data);
                }

                // buffer and stream setup.
                let mut buffer = Box::new([0u8; 1024]);

                let mut builder = spa_sys::spa_pod_builder {
                    data: buffer.as_mut_ptr().cast(),
                    size: buffer.len() as _,
                    _padding: 0,
                    state: spa_sys::spa_pod_builder_state {
                        offset: 0,
                        flags: 0,
                        frame: ptr::null_mut(),
                    },
                    callbacks: spa_sys::spa_callbacks {
                        funcs: ptr::null(),
                        data: ptr::null_mut(),
                    },
                };

                let audio_info = spa_sys::spa_audio_info_raw {
                    format: spa::AudioFormat::S16.into_raw(),
                    flags: 0,
                    rate: DEFAULT_RATE,
                    channels: DEFAULT_CHANNELS,
                    position: [0; 64],
                };

                unsafe {
                    let info = spa_sys::spa_format_audio_raw_build(
                        &mut builder,
                        spa_sys::SPA_PARAM_EnumFormat,
                        &audio_info,
                    );

                    let mut params: [*const spa_pod; 1] = [info];

                    stream.connect(
                        spa_sys::SPA_DIRECTION_OUTPUT,
                        pw::ID_ANY,
                        pw::StreamFlags::MAP_BUFFERS,
                        &mut params,
                    );
                }

                state.playback.push(PlaybackState {
                    stream_data,
                    stream_listener,
                    events,
                    buffer,
                    stream,
                });
            }
            Task::AddCaptureStream => {}
        }
    }
}

/// Receiver indirection which is only set up once the mixer is running.
struct RxIndirect {
    f: fn(data: *mut c_void, count: u64),
    data: *mut c_void,
}

/// Set up a handle and a mixer.
pub fn setup() -> (Handle, Mixer) {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

    let main_loop = pw::MainLoop::new();
    let lp = main_loop.get_loop();

    unsafe extern "C" fn indirect_rx(data: *mut c_void, count: u64) {
        unsafe {
            let RxIndirect { f, data } = data.cast::<RxIndirect>().read();
            f(data, count);
        }
    }

    unsafe extern "C" fn on_shutdown(data: *mut c_void, _: u64) {
        tracing::info!("Shutting down main loop");
        let main_loop = unsafe { &*(data.cast::<pw::MainLoop>()) };
        main_loop.quit();
    }

    let rx_context = Box::leak(Box::new(RxIndirect {
        f: |_, _| {},
        data: ptr::null_mut(),
    }));

    let rx_event = lp.add_event(indirect_rx, rx_context);
    let shutdown = lp.add_event(on_shutdown, main_loop);

    let handle = Handle {
        lp,
        tx,
        rx_event,
        shutdown,
    };

    let mixer = Mixer {
        main_loop,
        rx_event,
        rx,
        rx_context,
    };

    (handle, mixer)
}

struct PlaybackState {
    stream_data: Box<PlaybackData>,
    stream_listener: Pin<Box<spa::Hook>>,
    events: Pin<Box<pw::StreamEvents>>,
    buffer: Box<[u8; 1024]>,
    stream: &'static pw::Stream,
}

#[derive(Default)]
struct MixerState {
    playback: Vec<PlaybackState>,
}

impl Mixer {
    /// Run the mixer.
    pub fn run(self) {
        tracing::info!("{:?}", std::thread::current().id());

        let mut state = MixerState::default();

        unsafe {
            let lp = self.main_loop.get_loop();
            let context = lp.new_context();

            let core = context.connect();
            let registry = core.registry();

            let mut rx_data = RxData {
                rx: self.rx,
                core,
                state: NonNull::from(&mut state),
            };

            self.rx_context.write(RxIndirect {
                f: on_rx,
                data: (&mut rx_data as *mut RxData).cast::<c_void>(),
            });

            let mut registry_listener = pin!(spa::Hook::empty());

            let custom_data = EventGlobalData {};

            let events = pin!(
                pw::RegistryEvents::new()
                    .global(registry_event_global)
                    .global_remove(registry_event_global_remove)
            );
            registry.add_listener(registry_listener.as_mut(), events.as_ref(), &custom_data);

            let stream = core.new_stream(c"audio-capture", pw::StreamKind::AudioCapture);

            let capture_data = CaptureData {
                stream,
                first: true,
                lp,
            };

            let mut capture_stream_listener = pin!(spa::Hook::empty());

            let events = pin!(
                pw::StreamEvents::new()
                    .param_changed(capture_param_changed)
                    .state_changed(capture_state_changed)
                    .process(capture_on_process)
            );

            stream.add_listener(
                capture_stream_listener.as_mut(),
                events.as_ref(),
                &capture_data,
            );

            // buffer and stream setup.
            let mut buffer = [0u8; 1024];

            let mut b = spa_sys::spa_pod_builder {
                data: buffer.as_mut_ptr().cast(),
                size: buffer.len() as _,
                _padding: 0,
                state: spa_sys::spa_pod_builder_state {
                    offset: 0,
                    flags: 0,
                    frame: ptr::null_mut(),
                },
                callbacks: spa_sys::spa_callbacks {
                    funcs: ptr::null(),
                    data: ptr::null_mut(),
                },
            };

            let audio_info = spa_sys::spa_audio_info_raw {
                format: spa::AudioFormat::S16.into_raw(),
                flags: 0,
                rate: DEFAULT_RATE,
                channels: DEFAULT_CHANNELS,
                position: [0; 64],
            };

            let mut params: [*const spa_pod; 1] = [spa_sys::spa_format_audio_raw_build(
                &mut b,
                spa_sys::SPA_PARAM_EnumFormat,
                &audio_info,
            )];

            let flags = pw::StreamFlags::MAP_BUFFERS;

            stream.connect(spa_sys::SPA_DIRECTION_INPUT, pw::ID_ANY, flags, &mut params);

            let mut roundtrip_listener = pin!(spa::Hook::empty());

            let core_events = pin!(pw::CoreEvents::new().done(on_done));

            let pending = core.sync(pw_sys::PW_ID_CORE, 0);
            let roundtrip_data = RoundTripData { pending };
            core.add_listener(
                roundtrip_listener.as_mut(),
                core_events.as_ref(),
                &roundtrip_data,
            );

            let err = self.main_loop.run();

            tracing::info!("Main loop exited with: {err:?}");

            capture_stream_listener.remove();
            roundtrip_listener.remove();
            registry_listener.remove();

            for playback in &mut state.playback {
                playback.stream_listener.as_mut().remove();
                playback.stream.destroy();
            }

            stream.destroy();
            registry.destroy();
            core.disconnect();
            context.destroy();
            self.main_loop.destroy();

            _ = Box::from_raw(self.rx_context);
        }
    }
}
