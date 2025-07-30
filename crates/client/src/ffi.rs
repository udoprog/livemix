use core::ffi::c_char;
use core::fmt;

use protocol::consts;
use protocol::flags;

/** the maximum number of segments visible in the future */
const IO_POSITION_MAX_SEGMENTS: usize = 8;

#[derive(Copy, Clone, PartialEq, Eq)]
#[repr(transparent)]
struct Pad<T>(T);

impl<T> fmt::Debug for Pad<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Pad")
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct NodeActivationState {
    /// Current status, the result of spa_node_process().
    pub status: flags::Status,
    /// Required number of signals.
    pub required: u32,
    /// Number of pending signals.
    pub pending: u32,
}

/// The position information adds extra meaning to the raw clock times.
///
/// It
/// is set on all nodes and the clock id will contain the clock of the
/// driving
/// node in the graph.
///
/// The position information contains 1 or more segments
/// that convert the
/// raw clock times to a stream time. They are sorted based
/// on their
/// start times, and thus the order in which they will activate in
///
/// the future. This makes it possible to look ahead in the scheduled
/// segments
/// and anticipate the changes in the timeline.
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct IoPosition {
    /// clock position of driver, always valid and read only
    pub clock: IoClock,
    /// size of the video in the current cycle
    pub video: IoVideoSize,
    /// an offset to subtract from the clock position to get a running time.
    /// This is the time that the state has been in the RUNNING state and the
    /// time that should be used to compare the segment start values against.
    pub offset: i64,
    /// one of enum spa_io_position_state
    pub state: u32,
    /// number of segments
    pub n_segments: u32,
    /// segments
    pub segments: [IoSegment; IO_POSITION_MAX_SEGMENTS],
}

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct IoVideoSize {
    /// optional flags
    pub flags: u32,
    /// video stride in bytes
    pub stride: u32,
    /// the video size
    pub size: Rectangle,
    /// the minimum framerate, the cycle duration is always smaller to ensure
    /// there is only one video frame per cycle.
    pub framerate: Fraction,
    _pad: Pad<[u32; 4]>,
}

/// A segment converts a running time to a segment (stream) position.
///
/// The segment position is valid when the current running time is between start
/// and start + duration. The position is then calculated as: (running time
/// - start) * rate + position;
///
/// Support for looping is done by specifying the LOOPING flags with a non-zero
/// duration. When the running time reaches start + duration, duration is added
/// to start and the loop repeats.
///
/// Care has to be taken when the running time + clock.duration extends past the
/// start + duration from the segment; the user should correctly wrap around and
/// partially repeat the loop in the current cycle.
///
/// Extra information can be placed in the segment by setting the valid flags
/// and filling up the corresponding structures.
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct IoSegment {
    pub version: u32,
    /// extra flags
    pub flags: u32,
    /// value of running time when this info is active. Can be in the future for
    /// pending changes. It does not have to be in exact multiples of the clock
    /// duration.
    pub start: u64,
    /// duration when this info becomes invalid expressed in running time. If
    /// the duration is 0, this segment extends to the next segment. If the
    /// segment becomes invalid and the looping flag is set, the segment
    /// repeats.
    pub duration: u64,
    /// overall rate of the segment, can be negative for backwards time
    /// reporting.
    pub rate: f64,
    /// The position when the running time == start. can be invalid when the
    /// owner of the extra segment information has not yet made the mapping.
    pub position: u64,
    pub bar: IoSegmentBar,
    pub video: IoSegmentVideo,
}

/// video frame segment
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct IoSegmentVideo {
    /// flags
    pub flags: u32,
    /// offset in segment
    pub offset: u32,
    pub framerate: Fraction,
    pub hours: u32,
    pub minutes: u32,
    pub seconds: u32,
    pub frames: u32,
    /// 0 for progressive, 1 and 2 for interlaced
    pub field_count: u32,
    _pad: Pad<[u32; 11]>,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Fraction {
    pub num: u32,
    pub denom: u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Rectangle {
    pub width: u32,
    pub height: u32,
}

/// bar and beat segment
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct IoSegmentBar {
    /// extra flags
    pub flags: u32,
    /// offset in segment of this beat
    pub offset: u32,
    /// time signature numerator
    pub signature_num: f32,
    /// time signature denominator
    pub signature_denom: f32,
    /// beats per minute
    pub bpm: f64,
    /// current beat in segment
    pub beat: f64,
    _pad: Pad<[u32; 8]>,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct NodeActivation {
    pub status: consts::ActivationStatus,
    /// unsigned int version:1;
    /// A sync is pending.
    /// unsigned int pending_sync:1;
    /// A new position is pending.
    /// unsigned int pending_new_pos:1;
    pub header_bits: u32,
    /// one current state and one next state as version flag.
    pub state: [NodeActivationState; 2],
    /// time at which the node was triggered (i.e.  as ready to start processing
    /// in the current  iteration).
    pub signal_time: u64,
    /// time at which processing actually started.
    pub awake_time: u64,
    /// time at which processing was completed.
    pub finish_time: u64,
    /// previous time at which the node was triggered.
    pub prev_signal_time: u64,
    /// reposition info, used when driver  has this node id.
    pub reposition: IoSegment,
    /// update for the extra segment info fields used when driver segment_owner
    /// has this node id.
    pub segment: IoSegment,
    /// id of owners for each segment info struct nodes that want to update
    /// segment info need  CAS their node id in this array.
    pub segment_owner: [u32; 16],
    pub prev_awake_time: u64,
    pub prev_finish_time: u64,
    /// must be 0.
    _pad: Pad<[u32; 7]>,
    /// Version of client, see above.
    pub client_version: u32,
    /// Version of server, see above.
    pub server_version: u32,
    /// driver active on client.
    pub active_driver_id: u32,
    /// the current node driver id.
    pub driver_id: u32,
    /// extra flags.
    pub flags: u32,
    /// contains current position and segment info extra info is updated by
    /// nodes that have  themselves as owner in the segment structs.
    pub position: IoPosition,
    /// sync timeout in  position goes to RUNNING without waiting longer for
    /// sync clients.
    pub sync_timeout: u64,
    /// number of cycles before timeout.
    pub sync_left: u64,
    /// averaged over short, medium, long time.
    pub cpu_load: [f32; 3],
    /// number of xruns.
    pub xrun_count: u32,
    /// time of last xrun in microseconds.
    pub xrun_time: u64,
    /// delay of last xrun in microseconds.
    pub xrun_delay: u64,
    /// max of all xruns in microseconds.
    pub max_delay: u64,
    /// next command.
    pub command: u32,
    /// owner id with new reposition info, last one to update wins.
    pub reposition_owner: u32,
}

#[test]
fn test_layout() {
    use core::mem;

    assert_eq!(mem::size_of::<NodeActivation>(), 2312);
    assert_eq!(mem::offset_of!(NodeActivation, client_version), 540);
}

/// Absolute time reporting.
///
/// Nodes that can report clocking information will receive this io block. The
/// application sets the id. This is usually set as part of the position
/// information but can also be set separately.
///
/// The clock counts the elapsed time according to the clock provider since the
/// provider was last started.
///
/// Driver nodes are supposed to update the contents of \ref SPA_IO_Clock before
/// signaling the start of a graph cycle.  These updated clock values become
/// visible to other nodes in \ref SPA_IO_Position. Non-driver nodes do not need
/// to update the contents of their \ref SPA_IO_Clock.
///
/// The host generally gives each node a separate \ref spa_io_clock in \ref
/// SPA_IO_Clock, so that updates made by the driver are not visible in the
/// contents of \ref SPA_IO_Clock of other nodes. Instead, \ref SPA_IO_Position
/// is used to look up the current graph time.
///
/// A node is a driver when \ref spa_io_clock.id in \ref SPA_IO_Clock and \ref
/// spa_io_position.clock.id in \ref SPA_IO_Position are the same.
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct IoClock {
    /// Clock flags.
    pub flags: flags::IoClockFlag,
    /// Unique clock id, set by host application.
    pub id: u32,
    /// Clock name prefixed with API, set by node when it receives \ref
    /// SPA_IO_Clock. The clock name is unique per clock and can be used to
    /// check if nodes share the same clock.
    pub name: [c_char; 64],
    /// Time in nanoseconds against monotonic clock (CLOCK_MONOTONIC). This
    /// fields reflects a real time instant in the past. The value may have
    /// jitter.
    pub nsec: u64,
    /// Rate for position/duration/delay/xrun.
    pub rate: Fraction,
    /// Current position, in samples `rate`.
    pub position: u64,
    /// Duration of current cycle, in samples `rate`.
    pub duration: u64,
    /// Delay between position and hardware, in samples `rate`.
    pub delay: i64,
    /// Rate difference between clock and monotonic time, as a ratio of clock
    /// speeds.
    pub rate_diff: f64,
    /// Estimated next wakeup time in nanoseconds. This time is a logical start
    /// time of the next cycle, and is not necessarily in the future.
    pub next_nsec: u64,
    /// Target rate of next cycle.
    pub target_rate: Fraction,
    /// Target duration of next cycle.
    pub target_duration: u64,
    /// Seq counter. must be equal at start and end of read and lower bit must
    /// be 0.
    pub target_seq: u32,
    /// incremented each time the graph is started.
    pub cycle: u32,
    /// Estimated accumulated xrun duration.
    pub xrun: u64,
}

/// IO area to exchange buffers.
///
/// A set of buffers should first be configured on the node/port. Further
/// references to those buffers will be made by using the id of the buffer.
///
/// If status is SPA_STATUS_OK, the host should ignore the io area.
///
/// If status is SPA_STATUS_NEED_DATA, the host should:
/// 1) recycle the buffer in buffer_id, if possible
/// 2) prepare a new buffer and place the id in buffer_id.
///
/// If status is SPA_STATUS_HAVE_DATA, the host should consume the buffer in
/// buffer_id and set the state to SPA_STATUS_NEED_DATA when new data is
/// requested.
///
/// If status is SPA_STATUS_STOPPED, some error occurred on the port.
///
/// If status is SPA_STATUS_DRAINED, data from the io area was used to drain.
///
/// Status can also be a negative errno value to indicate errors. such as:
/// -EINVAL: buffer_id is invalid -EPIPE: no more buffers available
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct IoBuffers {
    /// the status code.
    status: flags::Status,
    /// a buffer id.
    buffer_id: u32,
}

/// Describes essential buffer header metadata such as flags and timestamps.
#[repr(C)]
pub struct MetaHeader {
    /// flags.
    flags: flags::MetaHeaderFlags,
    /// offset in current cycle.
    offset: u32,
    /// presentation timestamp in nanoseconds.
    pts: i64,
    /// decoding timestamp as a difference with pts.
    dts_offset: i64,
    /// sequence number, increments with a media specific frequency.
    seq: u64,
}

#[cfg(feature = "test-pipewire-sys")]
#[test]
fn test_sizes() {
    use core::mem;

    assert_eq!(
        mem::size_of::<IoPosition>(),
        mem::size_of::<libspa_sys::spa_io_position>()
    );
    assert_eq!(
        mem::align_of::<IoPosition>(),
        mem::align_of::<libspa_sys::spa_io_position>()
    );

    assert_eq!(
        mem::size_of::<IoClock>(),
        mem::size_of::<libspa_sys::spa_io_clock>()
    );
    assert_eq!(
        mem::align_of::<IoClock>(),
        mem::align_of::<libspa_sys::spa_io_clock>()
    );
}
