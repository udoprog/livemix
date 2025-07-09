use core::cell::UnsafeCell;
use core::ffi::{CStr, c_char, c_int, c_void};
use core::fmt;
use core::pin::Pin;
use core::ptr;

use pw_sys::{
    PW_KEY_MEDIA_CATEGORY, PW_KEY_MEDIA_ROLE, PW_KEY_MEDIA_TYPE, PW_VERSION_REGISTRY, pw_buffer,
    pw_context, pw_context_connect, pw_context_destroy, pw_context_new, pw_core,
    pw_core_disconnect, pw_core_events, pw_core_info, pw_core_methods, pw_loop, pw_main_loop,
    pw_main_loop_destroy, pw_main_loop_get_loop, pw_main_loop_new, pw_main_loop_quit,
    pw_main_loop_run, pw_properties_new, pw_proxy_destroy, pw_registry, pw_registry_events,
    pw_registry_methods, pw_stream, pw_stream_add_listener, pw_stream_connect, pw_stream_control,
    pw_stream_dequeue_buffer, pw_stream_destroy, pw_stream_events, pw_stream_new,
    pw_stream_queue_buffer, pw_stream_state,
};
use spa_sys::{
    spa_command, spa_dict, spa_loop_utils_add_event, spa_loop_utils_signal_event, spa_pod,
};

use crate::spa;

crate::macros::decl_enum! {
    #[repr(i32)]
    pub enum StreamState {
        /// the stream is in error.
        Error = pw_sys::pw_stream_state_PW_STREAM_STATE_ERROR,
        /// unconnected.
        Unconnected = pw_sys::pw_stream_state_PW_STREAM_STATE_UNCONNECTED,
        /// connection is in progress.
        Connecting = pw_sys::pw_stream_state_PW_STREAM_STATE_CONNECTING,
        /// paused.
        Paused = pw_sys::pw_stream_state_PW_STREAM_STATE_PAUSED,
        /// streaming.
        Streaming = pw_sys::pw_stream_state_PW_STREAM_STATE_STREAMING,
    }
}

crate::macros::bitflags! {
    pub struct StreamFlags(u32) {
        /// No flags.
        pub const NONE = 0;
        /// Try to automatically connect this stream.
        pub const AUTOCONNECT = 1;
        /// Start the stream inactive, pw_stream_set_active() needs to be called.
        /// explicitly
        pub const INACTIVE = 2;
        /// mmap the buffers except DmaBuf that is not explicitly marked as
        /// mappable.
        pub const MAP_BUFFERS = 4;
        /// Be a driver.
        pub const DRIVER = 8;
        /// Call process from the realtime thread. You MUST use RT safe functions in
        /// the process callback.
        pub const RT_PROCESS = 16;
        /// Don't convert format.
        pub const NO_CONVERT = 32;
        /// Require exclusive access to the device.
        pub const EXCLUSIVE = 64;
        /// Don't try to reconnect this stream when the sink/source is removed
        pub const DONT_RECONNECT = 128;
        /// the application will allocate buffer memory. In the add_buffer event,
        /// the data of the buffer should be set
        pub const ALLOC_BUFFERS = 256;
        /// the output stream will not be scheduled automatically but
        /// _trigger_process() needs to be called. This can be used when the output
        /// of the stream depends on input from other streams.
        pub const TRIGGER = 512;
        /// Buffers will not be dequeued/queued from the realtime process()
        /// function. This is assumed when RT_PROCESS is unset but can also be the
        /// case when the process() function does a trigger_process() that will then
        /// dequeue/queue a buffer from another process() function. since 0.3.73
        pub const ASYNC = 1024;
        /// Call process as soon as there is a buffer to dequeue. This is only
        /// relevant for playback and when not using RT_PROCESS. It can be used to
        /// keep the maximum number of buffers queued.
        ///
        /// Since 0.3.81
        pub const EARLY_PROCESS = 2048;
        /// Call trigger_done from the realtime thread. You MUST use RT safe
        /// functions in the trigger_done callback.
        ///
        /// Since 1.1.0
        pub const RT_TRIGGER_DONE = 4096;
    }
}

macro_rules! events {
    (
        $(
            $vis:vis struct $name:ident($path:ident) {
                version: $version:path,
                $(
                    $(#[$($meta:meta)*])*
                    $field:ident: $signature:ty,
                )*
            }
        )*
    ) => {
        $(
            $vis struct $name {
                inner: $path,
            }

            impl $name {
                #[inline]
                $vis fn new() -> Self {
                    Self {
                        inner: $path {
                            version: $version,
                            $($field: None,)*
                        },
                    }
                }


                $(
                    $(#[$($meta)*])*
                    #[inline]
                    $vis fn $field(mut self, func: $signature) -> Self {
                        self.inner.$field = Some(func);
                        self
                    }
                )*
            }
        )*
    }
}

events! {
    pub struct CoreEvents(pw_core_events) {
        version: pw_sys::PW_VERSION_CORE_EVENTS,
        /// Notify new core info
        ///
        /// This event is emitted when first bound to the core or when the hello
        /// method is called.
        ///
        /// \\param info new core info.
        info: unsafe extern "C" fn(data: *mut c_void, info: *const pw_core_info),
        /// Emit a done event
        ///
        /// The done event is emitted as a result of a sync method with the same
        /// seq number.
        ///
        /// \\param seq the seq number passed to the sync method call.
        done:
            unsafe extern "C" fn(
                data: *mut c_void,
                id: u32,
                seq: c_int,
            ),
        /// Emit a ping event
        ///
        /// The client should reply with a pong reply with the same seq number.
        ping:
            unsafe extern "C" fn(
                data: *mut c_void,
                id: u32,
                seq: c_int,
            ),
        /// Fatal error event
        ///
        /// The error event is sent out when a fatal (non-recoverable) error has
        /// occurred. The id argument is the proxy object where the error
        /// occurred, most often in response to a request to that object. The
        /// message is a brief description of the error, for (debugging)
        /// convenience.
        ///
        /// This event is usually also emitted on the proxy object with \\a id.
        ///
        /// \\param id object where the error occurred \\param seq the sequence
        /// number that generated the error \\param res error code \\param
        /// message error description
        error:
            unsafe extern "C" fn(
                data: *mut c_void,
                id: u32,
                seq: c_int,
                res: c_int,
                message: *const c_char,
            ),
        /// Remove an object ID
        ///
        /// This event is used internally by the object ID management logic.
        /// When a client deletes an object, the server will send this event to
        /// acknowledge that it has seen the delete request. When the client
        /// receives this event, it will know that it can safely reuse the
        /// object ID.
        ///
        /// \\param id deleted object ID
        remove_id: unsafe extern "C" fn(data: *mut c_void, id: u32),
        /// Notify an object binding
        ///
        /// This event is emitted when a local object ID is bound to a global
        /// ID. It is emitted before the global becomes visible in the registry.
        ///
        /// The bound_props event is an enhanced version of this event that also
        /// contains the extra global properties.
        ///
        /// \\param id bound object ID \\param global_id the global id bound to
        bound_id:
            unsafe extern "C" fn(data: *mut c_void, id: u32, global_id: u32),
        /// Add memory for a client
        ///
        /// Memory is given to a client as \\a fd of a certain memory \\a type.
        ///
        /// Further references to this fd will be made with the per memory
        /// unique identifier \\a id.
        ///
        /// \\param id the unique id of the memory \\param type the memory type,
        /// one of enum spa_data_type \\param fd the file descriptor \\param
        /// flags extra flags
        add_mem:
            unsafe extern "C" fn(
                data: *mut c_void,
                id: u32,
                type_: u32,
                fd: c_int,
                flags: u32,
            ),
        /// Remove memory for a client
        ///
        /// \\param id the memory id to remove
        remove_mem: unsafe extern "C" fn(data: *mut c_void, id: u32),
        /// Notify an object binding
        ///
        /// This event is emitted when a local object ID is bound to a global
        /// ID. It is emitted before the global becomes visible in the registry.
        ///
        /// This is an enhanced version of the bound_id event.
        ///
        /// \\param id bound object ID \\param global_id the global id bound to
        /// \\param props The properties of the new global object.
        ///
        /// Since version 4:1
        bound_props: unsafe extern "C" fn(
            data: *mut c_void,
            id: u32,
            global_id: u32,
            props: *const spa_dict,
        ),
    }

    pub struct RegistryEvents(pw_registry_events) {
        version: pw_sys::PW_VERSION_REGISTRY_EVENTS,
        /// Notify of a new global object
        ///
        /// The registry emits this event when a new global object is available.
        ///
        /// \\param id the global object id
        /// \\param permissions the permissions of the object
        /// \\param type the type of the interface
        /// \\param version the version of the interface
        /// \\param props extra properties of the global
        global: unsafe extern "C" fn(
            data: *mut c_void,
            id: u32,
            permissions: u32,
            type_: *const c_char,
            version: u32,
            props: *const spa_dict,
        ),
        /// Notify of a global object removal
        ///
        /// Emitted when a global object was removed from the registry. If the
        /// client has any bindings to the global, it should destroy those.
        ///
        /// \\param id the id of the global that was removed
        global_remove: unsafe extern "C" fn(data: *mut c_void, id: u32),
    }

    pub struct StreamEvents(pw_stream_events) {
        version: pw_sys::PW_VERSION_STREAM_EVENTS,
        destroy: unsafe extern "C" fn(data: *mut c_void),
        /// when the stream state changes. Since 1.4 this also sets errno when
        /// the new state is PW_STREAM_STATE_ERROR.
        state_changed: unsafe extern "C" fn(
                data: *mut c_void,
                old: pw_stream_state,
                state: pw_stream_state,
                error: *const c_char,
            ),
        /// Notify information about a control.
        control_info:
            unsafe extern "C" fn(
                data: *mut c_void,
                id: u32,
                control: *const pw_stream_control,
            ),
        /// When io changed on the stream.
        io_changed:
            unsafe extern "C" fn(
                data: *mut c_void,
                id: u32,
                area: *mut c_void,
                size: u32,
            ),
        /// When a parameter changed.
        param_changed:
            unsafe extern "C" fn(data: *mut c_void, id: u32, param: *const spa_pod),
        /// When a new buffer was created for this stream
        add_buffer:
            unsafe extern "C" fn(data: *mut c_void, buffer: *mut pw_buffer),
        /// When a buffer was destroyed for this stream
        remove_buffer:
            unsafe extern "C" fn(data: *mut c_void, buffer: *mut pw_buffer),
        /// When a buffer can be queued (for playback streams) or dequeued (for
        /// capture streams). This is normally called from the mainloop but can
        /// also be called directly from the realtime data thread if the user is
        /// prepared to deal with this.
        process: unsafe extern "C" fn(data: *mut c_void),
        /// The stream is drained
        drained: unsafe extern "C" fn(data: *mut c_void),
        /// A command notify, Since 0.3.39:1
        command: unsafe extern "C" fn(data: *mut c_void, command: *const spa_command),
        /// a trigger_process completed. Since version 0.3.40:2. This is
        /// normally called from the mainloop but since 1.1.0 it can also be
        /// called directly from the realtime data thread if the user is
        /// prepared to deal with this.
        trigger_done: unsafe extern "C" fn(data: *mut c_void),
    }
}

pub enum StreamKind {
    AudioPlayback,
    AudioCapture,
}

pub const ID_ANY: u32 = 0xffffffff;

/// Initialize the system PipeWire library.
pub fn init() {
    unsafe {
        pw_sys::pw_init(ptr::null_mut(), ptr::null_mut());
    }
}

#[repr(transparent)]
pub struct MainLoop {
    ptr: UnsafeCell<pw_main_loop>,
}

impl MainLoop {
    #[inline]
    pub fn new() -> &'static Self {
        let ptr = unsafe { pw_main_loop_new(ptr::null()) };
        debug_assert!(!ptr.is_null(), "pw_main_loop pointer cannot be null");
        unsafe { &*(ptr as *mut Self) }
    }

    #[inline]
    fn as_mut_ptr(&self) -> *mut pw_main_loop {
        self.ptr.get()
    }

    pub fn get_loop(&self) -> &'static Loop {
        unsafe { Loop::new(pw_main_loop_get_loop(self.as_mut_ptr())) }
    }

    #[inline]
    pub fn run(&self) -> c_int {
        unsafe { pw_main_loop_run(self.as_mut_ptr()) }
    }

    #[inline]
    pub fn quit(&self) {
        unsafe {
            pw_main_loop_quit(self.as_mut_ptr());
        }
    }

    #[inline]
    pub fn destroy(&self) {
        unsafe {
            pw_main_loop_destroy(self.as_mut_ptr());
        }
    }
}

pub struct Loop {
    ptr: UnsafeCell<pw_loop>,
}

impl Loop {
    #[inline]
    pub unsafe fn new(ptr: *mut pw_loop) -> &'static Self {
        debug_assert!(!ptr.is_null(), "MainLoop pointer cannot be null");
        unsafe { &*(ptr as *mut Self) }
    }

    #[inline]
    fn as_mut_ptr(&self) -> *mut pw_loop {
        self.ptr.get()
    }

    #[inline]
    pub fn new_context(&self) -> &'static Context {
        unsafe {
            let ptr = pw_context_new(self.as_mut_ptr(), ptr::null_mut(), 0);
            Context::new(ptr)
        }
    }

    #[inline]
    pub fn add_event<T>(
        &self,
        func: unsafe extern "C" fn(*mut c_void, u64),
        data: &T,
    ) -> &spa::Source {
        unsafe {
            let lp = self.as_mut_ptr();
            let data = (data as *const T).cast_mut().cast::<c_void>();
            spa::Source::new(spa_loop_utils_add_event((*lp).utils, Some(func), data))
        }
    }

    #[inline]
    pub fn signal_event(&self, source: &spa::Source) -> c_int {
        unsafe {
            let lp = self.as_mut_ptr();
            spa_loop_utils_signal_event((*lp).utils, source.as_mut_ptr())
        }
    }
}

pub struct Context {
    ptr: UnsafeCell<pw_context>,
}

impl Context {
    #[inline]
    unsafe fn new(ptr: *mut pw_context) -> &'static Self {
        debug_assert!(!ptr.is_null(), "Context pointer cannot be null");
        unsafe { &*(ptr as *mut Self) }
    }

    #[inline]
    fn as_mut_ptr(&self) -> *mut pw_context {
        self.ptr.get()
    }

    #[inline]
    pub fn connect(&self) -> &'static Core {
        unsafe {
            let ptr = pw_context_connect(self.as_mut_ptr(), ptr::null_mut(), 0);
            Core::new(ptr)
        }
    }

    #[inline]
    pub fn destroy(&self) {
        unsafe {
            pw_context_destroy(self.as_mut_ptr());
        }
    }
}

#[repr(transparent)]
pub struct Core {
    ptr: UnsafeCell<pw_core>,
}

impl Core {
    #[inline]
    unsafe fn new(ptr: *mut pw_core) -> &'static Self {
        debug_assert!(!ptr.is_null(), "Core pointer cannot be null");
        unsafe { &*(ptr as *mut Self) }
    }

    #[inline]
    fn as_mut_ptr(&self) -> *mut pw_core {
        self.ptr.get()
    }

    #[inline]
    pub fn registry(&self) -> &'static Registry {
        unsafe {
            let registry = libspa::spa_interface_call_method!(
                self.as_mut_ptr(),
                pw_core_methods,
                get_registry,
                PW_VERSION_REGISTRY,
                0
            );

            Registry::new(registry)
        }
    }

    #[inline]
    pub unsafe fn add_listener<T>(
        &self,
        mut listener: Pin<&mut spa::Hook>,
        events: Pin<&CoreEvents>,
        data: &T,
    ) {
        let data = (data as *const T).cast_mut().cast::<c_void>();

        libspa::spa_interface_call_method!(
            self.as_mut_ptr(),
            pw_core_methods,
            add_listener,
            unsafe { listener.as_mut().as_mut_ptr() },
            &events.inner,
            data
        );

        unsafe {
            listener.assume_init();
        }
    }

    pub fn sync(&self, id: u32, seq: c_int) -> c_int {
        unsafe {
            libspa::spa_interface_call_method!(self.as_mut_ptr(), pw_core_methods, sync, id, seq)
        }
    }

    #[inline]
    pub fn new_stream(&self, name: &CStr, kind: StreamKind) -> &'static Stream {
        unsafe {
            let props = match kind {
                StreamKind::AudioPlayback => pw_properties_new(
                    PW_KEY_MEDIA_TYPE.as_ptr().cast::<c_char>(),
                    c"Audio".as_ptr(),
                    PW_KEY_MEDIA_CATEGORY.as_ptr().cast::<c_char>(),
                    c"Playback".as_ptr(),
                    PW_KEY_MEDIA_ROLE.as_ptr().cast::<c_char>(),
                    c"Music".as_ptr(),
                    ptr::null::<c_char>(),
                ),
                StreamKind::AudioCapture => pw_properties_new(
                    PW_KEY_MEDIA_TYPE.as_ptr().cast::<c_char>(),
                    c"Audio".as_ptr(),
                    PW_KEY_MEDIA_CATEGORY.as_ptr().cast::<c_char>(),
                    c"Capture".as_ptr(),
                    PW_KEY_MEDIA_ROLE.as_ptr().cast::<c_char>(),
                    c"Music".as_ptr(),
                    ptr::null::<c_char>(),
                ),
            };

            let ptr = pw_stream_new(self.as_mut_ptr(), name.as_ptr(), props);
            Stream::new(ptr)
        }
    }

    #[inline]
    pub fn disconnect(&self) {
        unsafe {
            pw_core_disconnect(self.as_mut_ptr());
        }
    }
}

pub struct Stream {
    ptr: UnsafeCell<pw_stream>,
}

impl Stream {
    #[inline]
    unsafe fn new(ptr: *mut pw_stream) -> &'static Self {
        debug_assert!(!ptr.is_null(), "Stream pointer cannot be null");
        unsafe { &*(ptr as *mut Self) }
    }

    #[inline]
    fn as_mut_ptr(&self) -> *mut pw_stream {
        self.ptr.get()
    }

    #[inline]
    pub unsafe fn add_listener<T>(
        &self,
        mut listener: Pin<&mut spa::Hook>,
        events: Pin<&StreamEvents>,
        data: &T,
    ) {
        let data = data as *const T as *mut c_void;

        unsafe {
            pw_stream_add_listener(
                self.as_mut_ptr(),
                listener.as_mut().as_mut_ptr(),
                &events.inner,
                data,
            );
            listener.assume_init();
        }
    }

    #[inline]
    pub unsafe fn connect(
        &self,
        direction: u32,
        target_id: u32,
        flags: StreamFlags,
        params: &mut [*const spa_pod],
    ) -> c_int {
        unsafe {
            let n_params = params.len() as u32;
            pw_stream_connect(
                self.as_mut_ptr(),
                direction,
                target_id,
                flags.into_raw(),
                params.as_mut_ptr(),
                n_params,
            )
        }
    }

    #[inline]
    pub fn dequeue_buffer(&self) -> Option<&'static mut Buffer> {
        let buffer = unsafe { pw_stream_dequeue_buffer(self.as_mut_ptr()) };
        if buffer.is_null() {
            None
        } else {
            Some(unsafe { Buffer::new(buffer) })
        }
    }

    #[inline]
    pub fn queue_buffer(&self, b: &'static mut Buffer) -> c_int {
        unsafe { pw_stream_queue_buffer(self.as_mut_ptr(), b.as_mut_ptr()) }
    }

    #[inline]
    pub unsafe fn destroy(&self) {
        unsafe {
            pw_stream_destroy(self.as_mut_ptr());
        }
    }
}

pub struct Buffer {
    ptr: UnsafeCell<pw_buffer>,
}

impl Buffer {
    #[inline]
    unsafe fn new(ptr: *mut pw_buffer) -> &'static mut Self {
        debug_assert!(!ptr.is_null(), "Buffer pointer cannot be null");
        unsafe { &mut *(ptr as *mut Self) }
    }

    #[inline]
    fn as_mut_ptr(&self) -> *mut pw_buffer {
        self.ptr.get()
    }

    #[inline]
    pub fn buffer(&self) -> &mut spa::Buffer {
        unsafe { spa::Buffer::new(ptr::addr_of_mut!((*self.ptr.get()).buffer).read()) }
    }

    /// The requested number of frames.
    #[inline]
    pub fn requested(&self) -> u64 {
        unsafe { ptr::addr_of_mut!((*self.ptr.get()).requested).read() }
    }
}

#[repr(transparent)]
pub struct Registry {
    ptr: UnsafeCell<pw_registry>,
}

impl Registry {
    #[inline]
    unsafe fn new(ptr: *mut pw_registry) -> &'static Self {
        debug_assert!(!ptr.is_null(), "Registry pointer cannot be null");
        unsafe { &*(ptr as *mut Self) }
    }

    #[inline]
    fn as_mut_ptr(&self) -> *mut pw_registry {
        self.ptr.get()
    }

    #[inline]
    pub unsafe fn add_listener<T>(
        &self,
        mut listener: Pin<&mut spa::Hook>,
        events: Pin<&RegistryEvents>,
        data: &T,
    ) {
        let data = data as *const T as *mut c_void;

        libspa::spa_interface_call_method!(
            self.as_mut_ptr(),
            pw_registry_methods,
            add_listener,
            unsafe { listener.as_mut().as_mut_ptr() },
            &events.inner,
            data
        );

        unsafe {
            listener.assume_init();
        }
    }

    #[inline]
    pub unsafe fn destroy(&self) {
        unsafe {
            pw_proxy_destroy(self.as_mut_ptr().cast());
        }
    }
}
