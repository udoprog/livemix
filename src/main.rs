use core::ffi::{CStr, c_char, c_void};
use core::mem::MaybeUninit;
use core::pin::{Pin, pin};
use core::ptr;
use core::slice;

use pw_sys::{
    PW_VERSION_REGISTRY, PW_VERSION_REGISTRY_EVENTS, pw_context, pw_context_connect,
    pw_context_destroy, pw_context_new, pw_core, pw_core_disconnect, pw_core_methods, pw_init,
    pw_main_loop, pw_main_loop_destroy, pw_main_loop_get_loop, pw_main_loop_new, pw_main_loop_quit,
    pw_main_loop_run, pw_proxy_destroy, pw_registry, pw_registry_events, pw_registry_methods,
};
use spa_sys::{spa_dict, spa_dict_item, spa_dict_lookup, spa_hook};

#[repr(transparent)]
struct MainLoop {
    ptr: pw_main_loop,
}

impl MainLoop {
    #[inline]
    unsafe fn new(ptr: *mut pw_main_loop) -> &'static Self {
        debug_assert!(!ptr.is_null(), "MainLoop pointer cannot be null");
        unsafe { &mut *(ptr as *mut Self) }
    }

    #[inline]
    fn as_mut_ptr(&self) -> *mut pw_main_loop {
        &self.ptr as *const _ as *mut _
    }

    #[inline]
    fn new_context(&self) -> &'static Context {
        unsafe {
            let ptr = pw_context_new(pw_main_loop_get_loop(self.as_mut_ptr()), ptr::null_mut(), 0);
            Context::new(ptr)
        }
    }

    #[inline]
    fn run(&self) {
        unsafe {
            pw_main_loop_run(self.as_mut_ptr());
        }
    }

    #[inline]
    fn quit(&self) {
        unsafe {
            pw_main_loop_quit(self.as_mut_ptr());
        }
    }

    #[inline]
    fn destroy(&self) {
        unsafe {
            pw_main_loop_destroy(self.as_mut_ptr());
        }
    }
}

struct Context {
    ptr: pw_context,
}

impl Context {
    #[inline]
    unsafe fn new(ptr: *mut pw_context) -> &'static Self {
        debug_assert!(!ptr.is_null(), "Context pointer cannot be null");
        unsafe { &mut *(ptr as *mut Self) }
    }

    #[inline]
    fn as_mut_ptr(&self) -> *mut pw_context {
        &self.ptr as *const _ as *mut _
    }

    #[inline]
    fn connect(&self) -> &'static Core {
        unsafe {
            let ptr = pw_context_connect(self.as_mut_ptr(), ptr::null_mut(), 0);
            Core::new(ptr)
        }
    }

    #[inline]
    fn destroy(&self) {
        unsafe {
            pw_context_destroy(self.as_mut_ptr());
        }
    }
}

#[repr(transparent)]
struct Core {
    ptr: pw_core,
}

impl Core {
    #[inline]
    unsafe fn new(ptr: *mut pw_core) -> &'static Self {
        debug_assert!(!ptr.is_null(), "Core pointer cannot be null");
        unsafe { &mut *(ptr as *mut Self) }
    }

    #[inline]
    fn as_mut_ptr(&self) -> *mut pw_core {
        &self.ptr as *const _ as *mut _
    }

    #[inline]
    fn registry(&self) -> &'static Registry {
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
    fn disconnect(&self) {
        unsafe {
            pw_core_disconnect(self.as_mut_ptr());
        }
    }
}

#[repr(transparent)]
struct Registry {
    ptr: pw_registry,
}

impl Registry {
    #[inline]
    unsafe fn new(ptr: *mut pw_registry) -> &'static Self {
        debug_assert!(!ptr.is_null(), "Registry pointer cannot be null");
        unsafe { &mut *(ptr as *mut Self) }
    }

    #[inline]
    fn as_mut_ptr(&self) -> *mut pw_registry {
        &self.ptr as *const _ as *mut _
    }

    #[inline]
    unsafe fn add_listener<T>(
        &self,
        listener: Pin<&mut SpaHook>,
        events: *const pw_registry_events,
        data: &T,
    ) {
        let listener = unsafe { listener.get_unchecked_mut().data.as_mut_ptr() };

        let data = data as *const T as *mut c_void;

        libspa::spa_interface_call_method!(
            self.as_mut_ptr(),
            pw_registry_methods,
            add_listener,
            listener,
            events,
            data
        );
    }

    #[inline]
    unsafe fn destroy(&self) {
        unsafe {
            pw_proxy_destroy(self.as_mut_ptr().cast());
        }
    }
}

#[repr(transparent)]
struct SpaHook {
    data: MaybeUninit<spa_hook>,
}

impl SpaHook {
    fn empty() -> Self {
        Self {
            data: MaybeUninit::zeroed(),
        }
    }
}

#[repr(transparent)]
struct SpaDict {
    ptr: spa_dict,
}

impl SpaDict {
    #[inline]
    unsafe fn new(ptr: *const spa_dict) -> &'static Self {
        debug_assert!(!ptr.is_null(), "SpaDict pointer cannot be null");
        unsafe { &*(ptr as *const Self) }
    }

    #[inline]
    fn as_ptr(&self) -> *const spa_dict {
        &self.ptr as *const _
    }

    fn as_raw_items(&self) -> &[spa_dict_item] {
        unsafe { slice::from_raw_parts(self.ptr.items, self.ptr.n_items as usize) }
    }

    #[inline]
    fn len(&self) -> usize {
        self.ptr.n_items as usize
    }

    #[inline]
    fn iter(&self) -> impl Iterator<Item = (&CStr, &CStr)> {
        let items = self.as_raw_items();

        items.iter().map(|item| {
            let key = unsafe { CStr::from_ptr(item.key) };
            let value = unsafe { CStr::from_ptr(item.value) };
            (key, value)
        })
    }

    #[inline]
    fn lookup(&self, key: &CStr) -> Option<&CStr> {
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

struct CustomData {
    main_loop: &'static MainLoop,
}

unsafe extern "C" fn registry_event_global(
    data: *mut c_void,
    id: u32,
    _permissions: u32,
    ty: *const c_char,
    version: u32,
    props: *const spa_dict,
) {
    let data = unsafe { &*data.cast::<CustomData>() };
    let props = unsafe { SpaDict::new(props) };
    // println!("here");
    let ty = unsafe { CStr::from_ptr(ty) };
    let ty = ty.to_string_lossy();
    println!("object: id:{id} type:{ty}/{version}");

    for (key, value) in props.iter() {
        let key = key.to_string_lossy();
        let value = value.to_string_lossy();
        println!("  {key}: {value}");
    }

    data.main_loop.quit();
}

fn main() {
    unsafe {
        pw_init(ptr::null_mut(), ptr::null_mut());

        let main_loop = MainLoop::new(pw_main_loop_new(ptr::null()));
        let context = main_loop.new_context();

        let core = context.connect();
        let registry = core.registry();

        let mut registry_listener = pin!(SpaHook::empty());

        let events = pw_registry_events {
            version: PW_VERSION_REGISTRY_EVENTS,
            global: Some(registry_event_global),
            global_remove: None,
        };

        let custom_data = CustomData { main_loop };

        registry.add_listener(registry_listener.as_mut(), &events, &custom_data);

        main_loop.run();

        registry.destroy();
        core.disconnect();
        context.destroy();
        main_loop.destroy();
    }
}
