pod::macros::flags! {
    #[examples = [AUTOCONNECT, INACTIVE]]
    #[not_set = [EXCLUSIVE]]
    pub struct StreamFlags(u32) {
        pub const NONE;
        /// Try to automatically connect this stream.
        pub const AUTOCONNECT = 1 << 0;
        /// Start the stream inactive, pw_stream_set_active() needs to be
        /// called. explicitly
        pub const INACTIVE = 1 << 1;
        /// mmap the buffers except DmaBuf that is not explicitly marked as
        /// mappable.
        pub const MAP_BUFFERS = 1 << 2;
        /// Be a driver.
        pub const DRIVER = 1 << 3;
        /// Call process from the realtime thread. You MUST use RT safe
        /// functions in the process callback.
        pub const RT_PROCESS = 1 << 4;
        /// Don't convert format.
        pub const NO_CONVERT = 1 << 5;
        /// Require exclusive access to the device.
        pub const EXCLUSIVE = 1 << 6;
        /// Don't try to reconnect this stream when the sink/source is removed
        pub const DONT_RECONNECT = 1 << 7;
        /// the application will allocate buffer memory. In the add_buffer
        /// event, the data of the buffer should be set
        pub const ALLOC_BUFFERS = 1 << 8;
        /// the output stream will not be scheduled automatically but
        /// _trigger_process() needs to be called. This can be used when the
        /// output of the stream depends on input from other streams.
        pub const TRIGGER = 1 << 9;
        /// Buffers will not be dequeued/queued from the realtime process()
        /// function. This is assumed when RT_PROCESS is unset but can also be
        /// the case when the process() function does a trigger_process() that
        /// will then dequeue/queue a buffer from another process() function.
        /// since 0.3.73
        pub const ASYNC = 1 << 10;
        /// Call process as soon as there is a buffer to dequeue. This is only
        /// relevant for playback and when not using RT_PROCESS. It can be used
        /// to keep the maximum number of buffers queued.
        ///
        /// Since 0.3.81
        pub const EARLY_PROCESS = 1 << 11;
        /// Call trigger_done from the realtime thread. You MUST use RT safe
        /// functions in the trigger_done callback.
        ///
        /// Since 1.1.0
        pub const RT_TRIGGER_DONE = 1 << 12;
    }

    #[examples = [PARAMS]]
    #[not_set = [INFO]]
    pub struct ClientNodeUpdate(u32) {
        pub const NONE;
        pub const PARAMS = 1 << 0;
        pub const INFO = 1 << 1;
    }

    #[examples = [FLAGS, PROPS]]
    #[not_set = [PARAMS]]
    pub struct NodeChangeMask(u32) {
        pub const NONE;
        pub const FLAGS = 1 << 0;
        pub const PROPS = 1 << 1;
        pub const PARAMS = 1 << 2;
    }

    #[examples = [RT, NEED_CONFIGURE]]
    #[not_set = [ASYNC]]
    pub struct Node(u64) {
        pub const NONE;
        /// Node can do real-time processing.
        pub const RT = 1 << 0;
        /// Input ports can be added/removed.
        pub const IN_DYNAMIC_PORTS = 1 << 1;
        /// Output ports can be added/removed.
        pub const OUT_DYNAMIC_PORTS = 1 << 2;
        /// Input ports can be reconfigured with PortConfig parameter.
        pub const IN_PORT_CONFIG = 1 << 3;
        /// Output ports can be reconfigured with PortConfig parameter.
        pub const OUT_PORT_CONFIG = 1 << 4;
        /// Node needs configuration before it can be started.
        pub const NEED_CONFIGURE = 1 << 5;
        /// The process function might not immediately produce or consume data but might offload the work to a worker thread.
        pub const ASYNC = 1 << 6;
    }

    #[examples = [SERIAL, READ]]
    #[not_set = [WRITE]]
    pub struct Param(u32) {
        pub const NONE;
        /// Flag to signal update even when the read/write flags don't change.
        pub const SERIAL = 1 << 0;
        pub const READ = 1 << 1;
        pub const WRITE = 1 << 2;
    }
}

impl Param {
    /// Read and write flags combined.
    pub const READWRITE: Self = Self(Param::WRITE.0 | Param::READ.0);
}
