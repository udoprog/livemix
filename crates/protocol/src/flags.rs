pod::macros::flags! {
    #[examples = [AUTOCONNECT, INACTIVE]]
    #[not_set = [EXCLUSIVE]]
    #[module = protocol::flags]
    pub struct StreamFlags(u32) {
        NONE;
        /// Try to automatically connect this stream.
        #[constant = pipewire_sys::pw_stream_flags_PW_STREAM_FLAG_AUTOCONNECT]
        AUTOCONNECT = 1 << 0;
        /// Start the stream inactive, pw_stream_set_active() needs to be called
        /// explicitly.
        #[constant = pipewire_sys::pw_stream_flags_PW_STREAM_FLAG_INACTIVE]
        INACTIVE = 1 << 1;
        /// mmap the buffers except DmaBuf that is not explicitly marked as
        /// mappable.
        #[constant = pipewire_sys::pw_stream_flags_PW_STREAM_FLAG_MAP_BUFFERS]
        MAP_BUFFERS = 1 << 2;
        /// Be a driver.
        #[constant = pipewire_sys::pw_stream_flags_PW_STREAM_FLAG_DRIVER]
        DRIVER = 1 << 3;
        /// Call process from the realtime thread. You MUST use RT safe
        /// functions in the process callback.
        #[constant = pipewire_sys::pw_stream_flags_PW_STREAM_FLAG_RT_PROCESS]
        RT_PROCESS = 1 << 4;
        /// Don't convert format.
        #[constant = pipewire_sys::pw_stream_flags_PW_STREAM_FLAG_NO_CONVERT]
        NO_CONVERT = 1 << 5;
        /// Require exclusive access to the device.
        #[constant = pipewire_sys::pw_stream_flags_PW_STREAM_FLAG_EXCLUSIVE]
        EXCLUSIVE = 1 << 6;
        /// Don't try to reconnect this stream when the sink/source is removed.
        #[constant = pipewire_sys::pw_stream_flags_PW_STREAM_FLAG_DONT_RECONNECT]
        DONT_RECONNECT = 1 << 7;
        /// the application will allocate buffer memory. In the add_buffer
        /// event, the data of the buffer should be set.
        #[constant = pipewire_sys::pw_stream_flags_PW_STREAM_FLAG_ALLOC_BUFFERS]
        ALLOC_BUFFERS = 1 << 8;
        /// the output stream will not be scheduled automatically but
        /// _trigger_process() needs to be called. This can be used when the
        /// output of the stream depends on input from other streams.
        #[constant = pipewire_sys::pw_stream_flags_PW_STREAM_FLAG_TRIGGER]
        TRIGGER = 1 << 9;
        /// Buffers will not be dequeued/queued from the realtime process()
        /// function. This is assumed when RT_PROCESS is unset but can also be
        /// the case when the process() function does a trigger_process() that
        /// will then dequeue/queue a buffer from another process() function.
        ///
        /// Since `0.3.73`.
        #[constant = pipewire_sys::pw_stream_flags_PW_STREAM_FLAG_ASYNC]
        ASYNC = 1 << 10;
        /// Call process as soon as there is a buffer to dequeue. This is only
        /// relevant for playback and when not using RT_PROCESS. It can be used
        /// to keep the maximum number of buffers queued.
        ///
        /// Since `0.3.81`.
        #[constant = pipewire_sys::pw_stream_flags_PW_STREAM_FLAG_EARLY_PROCESS]
        EARLY_PROCESS = 1 << 11;
        /// Call trigger_done from the realtime thread. You MUST use RT safe
        /// functions in the trigger_done callback.
        ///
        /// Since `1.1.0`.
        #[constant = pipewire_sys::pw_stream_flags_PW_STREAM_FLAG_RT_TRIGGER_DONE]
        RT_TRIGGER_DONE = 1 << 12;
    }

    #[examples = [PARAMS]]
    #[not_set = [INFO]]
    #[module = protocol::flags]
    pub struct ClientNodeUpdate(u32) {
        NONE;
        #[constant = pipewire_sys::PW_CLIENT_NODE_UPDATE_PARAMS]
        PARAMS = 1 << 0;
        #[constant = pipewire_sys::PW_CLIENT_NODE_UPDATE_INFO]
        INFO = 1 << 1;
    }

    #[examples = [PARAMS]]
    #[not_set = [INFO]]
    #[module = protocol::flags]
    pub struct ClientNodePortUpdate(u32) {
        NONE;
        #[constant = pipewire_sys::PW_CLIENT_NODE_PORT_UPDATE_PARAMS]
        PARAMS = 1 << 0;
        #[constant = pipewire_sys::PW_CLIENT_NODE_PORT_UPDATE_INFO]
        INFO = 1 << 1;
    }

    #[examples = [FLAGS, PROPS]]
    #[not_set = [PARAMS]]
    #[module = protocol::flags]
    pub struct NodeChangeMask(u64) {
        NONE;
        #[constant = libspa_sys::SPA_NODE_CHANGE_MASK_FLAGS]
        FLAGS = 1 << 0;
        #[constant = libspa_sys::SPA_NODE_CHANGE_MASK_PROPS]
        PROPS = 1 << 1;
        #[constant = libspa_sys::SPA_NODE_CHANGE_MASK_PARAMS]
        PARAMS = 1 << 2;
    }

    #[examples = [FLAGS, PROPS]]
    #[not_set = [PARAMS]]
    #[module = protocol::flags]
    pub struct PortChangeMask(u64) {
        NONE;
        /// Same as `SPA_PORT_CHANGE_MASK_FLAGS`.
        #[constant = libspa_sys::SPA_PORT_CHANGE_MASK_FLAGS]
        FLAGS = 1 << 0;
        /// Same as `SPA_PORT_CHANGE_MASK_RATE`.
        #[constant = libspa_sys::SPA_PORT_CHANGE_MASK_RATE]
        RATE = 1 << 1;
        /// Same as `SPA_PORT_CHANGE_MASK_PROPS`.
        #[constant = libspa_sys::SPA_PORT_CHANGE_MASK_PROPS]
        PROPS = 1 << 2;
        /// Same as `SPA_PORT_CHANGE_MASK_PARAMS`.
        #[constant = libspa_sys::SPA_PORT_CHANGE_MASK_PARAMS]
        PARAMS = 1 << 3;
    }

    #[examples = [RT, NEED_CONFIGURE]]
    #[not_set = [ASYNC]]
    #[module = protocol::flags]
    pub struct Node(u64) {
        NONE;
        /// Node can do real-time processing.
        #[constant = libspa_sys::SPA_NODE_FLAG_RT]
        RT = 1 << 0;
        /// Input ports can be added/removed.
        #[constant = libspa_sys::SPA_NODE_FLAG_IN_DYNAMIC_PORTS]
        IN_DYNAMIC_PORTS = 1 << 1;
        /// Output ports can be added/removed.
        #[constant = libspa_sys::SPA_NODE_FLAG_OUT_DYNAMIC_PORTS]
        OUT_DYNAMIC_PORTS = 1 << 2;
        /// Input ports can be reconfigured with PortConfig parameter.
        #[constant = libspa_sys::SPA_NODE_FLAG_IN_PORT_CONFIG]
        IN_PORT_CONFIG = 1 << 3;
        /// Output ports can be reconfigured with PortConfig parameter.
        #[constant = libspa_sys::SPA_NODE_FLAG_OUT_PORT_CONFIG]
        OUT_PORT_CONFIG = 1 << 4;
        /// Node needs configuration before it can be started.
        #[constant = libspa_sys::SPA_NODE_FLAG_NEED_CONFIGURE]
        NEED_CONFIGURE = 1 << 5;
        /// The process function might not immediately produce or consume data but might offload the work to a worker thread.
        #[constant = libspa_sys::SPA_NODE_FLAG_ASYNC]
        ASYNC = 1 << 6;
    }

    #[examples = [REMOVABLE, OPTIONAL]]
    #[not_set = [TERMINAL]]
    #[module = protocol::flags]
    pub struct Port(u64) {
        NONE;
        /// Port can be removed.
        #[constant = libspa_sys::SPA_PORT_FLAG_REMOVABLE]
        REMOVABLE = 1 << 0;
        /// Processing on port is optional.
        #[constant = libspa_sys::SPA_PORT_FLAG_OPTIONAL]
        OPTIONAL = 1 << 1;
        /// The port can allocate buffer data.
        #[constant = libspa_sys::SPA_PORT_FLAG_CAN_ALLOC_BUFFERS]
        CAN_ALLOC_BUFFERS = 1 << 2;
        /// The port can process data in-place and will need a writable input
        /// buffer.
        #[constant = libspa_sys::SPA_PORT_FLAG_IN_PLACE]
        IN_PLACE = 1 << 3;
        /// The port does not keep a ref on the buffer. This means the node will
        /// always completely consume the input buffer and it will be recycled
        /// after process.
        #[constant = libspa_sys::SPA_PORT_FLAG_NO_REF]
        NO_REF = 1 << 4;
        /// Output buffers from this port are timestamped against a live clock.
        #[constant = libspa_sys::SPA_PORT_FLAG_LIVE]
        LIVE = 1 << 5;
        /// Connects to some device.
        #[constant = libspa_sys::SPA_PORT_FLAG_PHYSICAL]
        PHYSICAL = 1 << 6;
        /// Data was not created from this port or will not be made available on
        /// another port.
        #[constant = libspa_sys::SPA_PORT_FLAG_TERMINAL]
        TERMINAL = 1 << 7;
        /// Data pointer on buffers can be changed. Only the buffer data marked
        /// as DYNAMIC can be changed.
        #[constant = libspa_sys::SPA_PORT_FLAG_DYNAMIC_DATA]
        DYNAMIC_DATA = 1 << 8;
    }

    #[examples = [SERIAL, READ]]
    #[not_set = [WRITE]]
    #[module = protocol::flags]
    pub struct Param(u32) {
        NONE;
        /// Flag to signal update even when the read/write flags don't change.
        SERIAL = 1 << 0;
        READ = 1 << 1;
        WRITE = 1 << 2;
    }

    #[examples = [OUT]]
    #[not_set = [MAPPED]]
    #[module = protocol::flags]
    pub struct DataFlag(u32) {
        NONE;
        OUT = 1 << 0;
        MAPPED = 1 << 1;
    }

    /// Describes `enum pw_memblock_flags`.
    #[examples = [MAP]]
    #[not_set = [DONT_NOTIFY]]
    #[module = protocol::flags]
    pub struct MemBlock(u32) {
        NONE;
        /// memory is readable.
        READABLE = 1 << 0;
        /// memory is writable.
        WRITABLE = 1 << 1;
        /// seal the fd.
        SEAL = 1 << 2;
        /// mmap the fd.
        MAP = 1 << 3;
        /// don't close fd.
        DONT_CLOSE = 1 << 4;
        /// don't notify events.
        DONT_NOTIFY = 1 << 5;
        /// the fd can not be mmapped.
        UNMAPPABLE = 1 << 6;
    }

    /// Describes `enum pw_memblock_flags`.
    #[examples = [WRITE]]
    #[not_set = [PRIVATE]]
    #[module = protocol::flags]
    pub struct MemMap(u32) {
        NONE;
        /// map in read mode.
        READ = 1 << 0;
        /// map in write mode.
        WRITE = 1 << 1;
        /// map the same area twice after each other, creating a circular ringbuffer.
        TWICE = 1 << 2;
        /// writes will be private.
        PRIVATE = 1 << 3;
        /// lock the memory into RAM.
        LOCKED = 1 << 4;
    }

    /// Describes `SPA_IO_CLOCK_FLAG_*`.
    #[examples = [FREEWHEEL]]
    #[not_set = [LAZY]]
    #[module = protocol::flags]
    pub struct IoClockFlag(u32) {
        NONE;
        /// Graph is freewheeling.
        FREEWHEEL = 1 << 0;
        /// Recovering from xrun.
        XRUN_RECOVER = 1 << 1;
        /// Lazy scheduling.
        LAZY = 1 << 2;
        /// The rate of the clock is only approximate.
        NO_RATE = 1 << 3;
    }

    /// Describes `SPA_STATUS_*`.
    #[examples = [NEED_DATA]]
    #[not_set = [HAVE_DATA]]
    #[module = protocol::flags]
    pub struct Status(u32) {
        /// Equivalent of `SPA_STATUS_NEED_OK`.
        NONE;
        /// Equivalent of `SPA_STATUS_NEED_DATA`.
        NEED_DATA = 1 << 0;
        /// Equivalent of `SPA_STATUS_HAVE_DATA`.
        HAVE_DATA = 1 << 1;
        /// Equivalent of `SPA_STATUS_STOPPED`.
        STOPPED = 1 << 2;
        /// Equivalent of `SPA_STATUS_DRAINED`.
        DRAINED = 1 << 3;
    }

    /// Describes `SPA_HEADER_FLAG_*`.
    #[examples = [CORRUPTED]]
    #[not_set = [HEADER]]
    #[module = protocol::flags]
    pub struct MetaHeaderFlags(u32) {
        NONE;
        /// data is not continuous with previous buffer.
        DISCONT = 1 << 0;
        /// data might be corrupted.
        CORRUPTED = 1 << 1;
        /// media specific marker.
        MARKER = 1 << 2;
        /// data contains a codec specific header.
        HEADER = 1 << 3;
        /// data contains media neutral data.
        GAP = 1 << 4;
        /// cannot be decoded independently.
        DELTA_UNIT = 1 << 5;
    }
}

impl Param {
    /// Read and write flags combined.
    pub const READWRITE: Self = Self(Self::WRITE.0 | Self::READ.0);
}

impl MemBlock {
    pub const READWRITE: Self = Self(Self::READABLE.0 | Self::WRITABLE.0);
}

impl MemMap {
    pub const READWRITE: Self = Self(Self::READ.0 | Self::WRITE.0);
}
