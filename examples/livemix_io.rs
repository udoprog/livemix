use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::fs::File;
use std::io::BufWriter;
use std::mem;
use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use client::events::StreamEvent;
use client::ptr::{atomic, volatile};
use client::{ClientNode, MixId, Port, PortId, PortParam, Stream, utils};
use pod::buf::ArrayVec;
use pod::{ChoiceType, Type};
use protocol::buf::RecvBuf;
use protocol::consts::{Activation, Direction};
use protocol::flags::Status;
use protocol::id::{
    self, AudioFormat, Format, IoType, MediaSubType, MediaType, Meta, ObjectType, Param,
    ParamBuffers, ParamIo, ParamMeta,
};
use protocol::ids::IdSet;
use protocol::poll::{Interest, PollEvent};
use protocol::{Connection, Poll, TimerFd, ffi, flags, object};

const BUFFER_SAMPLES: u32 = 128;
const M_PI_M2: f32 = std::f32::consts::PI * 2.0;
const DEFAULT_RATE: u32 = 48000;
const DEFAULT_VOLUME: f32 = 0.1;
const TONE: f32 = 440.0;

struct InputBuffer {
    format: object::AudioFormat,
    buf: Vec<f32>,
}

#[derive(Default)]
struct Stats {
    /// At the beginning of processing we check whether or not peers have a
    /// pending value greater than 0, and have a status of NOT_TRIGGERED.
    ///
    /// If that is not the case, we increment this counter add add the peer to the bitset.
    non_ready: usize,
    non_ready_set: IdSet,
    not_input_have_data: usize,
    not_self_triggered: usize,
    no_output_buffer: usize,
    failed: usize,
    failed_set: IdSet,
    succeeded: usize,
    succeeded_set: IdSet,
    timing_sum: u64,
    timing_count: usize,
}

struct ExampleApplication {
    tick: usize,
    formats: HashMap<(Direction, PortId), object::AudioFormat>,
    accumulators: HashMap<PortId, f32>,
    inputs: HashMap<(PortId, MixId), InputBuffer>,
    stats: Stats,
}

impl ExampleApplication {
    #[tracing::instrument(skip(self, node))]
    fn process(&mut self, node: &mut ClientNode) -> Result<()> {
        self.tick = self.tick.wrapping_add(1);
        let then = utils::get_monotonic_nsec();

        let Some(na) = &mut node.activation else {
            return Ok(());
        };

        let status;

        unsafe {
            status = atomic!(na, status);

            if !status.compare_exchange(Activation::TRIGGERED, Activation::AWAKE) {
                self.stats.not_self_triggered += 1;
                return Ok(());
            }

            let awake_time = volatile!(na, awake_time).replace(then);
            volatile!(na, prev_awake_time).write(awake_time);
        }

        for a in &node.peer_activations {
            unsafe {
                let pending = atomic!(a.region, state[0].pending).load();
                let status = atomic!(a.region, status).load();

                if pending == 0 || status != Activation::NOT_TRIGGERED {
                    self.stats.non_ready += 1;
                    self.stats.non_ready_set.set(a.peer_id);
                }
            }
        }

        let duration;

        if let Some(io_position) = &mut node.io_position {
            duration = unsafe { volatile!(io_position, clock.duration).read() };
        } else {
            tracing::error!("Missing IO position");
            return Ok(());
        }

        for port in node.ports.inputs_mut() {
            let Some(format) = self.formats.get(&(port.direction, port.id)) else {
                continue;
            };

            if format.channels != 1 || format.format != AudioFormat::F32P || format.rate == 0 {
                tracing::warn!(?format, "Unsupported format on output port");
                continue;
            }

            for buf in &mut port.io_buffers {
                let status = unsafe { volatile!(buf.region, status).read() };

                if !(status & Status::HAVE_DATA) {
                    self.stats.not_input_have_data += 1;
                    continue;
                }

                let id = unsafe { volatile!(buf.region, buffer_id).read() };

                let Some(buffer) = port.port_buffers.get_mut(buf.mix_id, id as u32) else {
                    bail!("Input no buffer with id {id} for port {}", port.id);
                };

                let _ = &buffer.metas[0];
                let data = &buffer.datas[0];

                let b = match self.inputs.entry((port.id, buf.mix_id)) {
                    Entry::Occupied(mut e) => {
                        if e.get().format != *format {
                            e.get_mut().buf.clear();
                            e.get_mut().format = format.clone();
                        }

                        e.into_mut()
                    }
                    Entry::Vacant(e) => e.insert(InputBuffer {
                        format: format.clone(),
                        buf: Vec::with_capacity(duration as usize),
                    }),
                };

                let samples;

                unsafe {
                    let chunk = data.chunk.as_ref();
                    let offset = chunk.offset as usize % data.max_size;
                    let size = (chunk.size as usize - offset).min(data.max_size);

                    samples = size / mem::size_of::<f32>();

                    b.buf.reserve(samples);

                    b.buf
                        .as_mut_ptr()
                        .add(b.buf.len())
                        .copy_from_nonoverlapping(
                            data.region.as_ptr().wrapping_add(offset).cast::<f32>(),
                            samples,
                        );

                    b.buf.set_len(b.buf.len() + samples);
                }

                unsafe { volatile!(buf.region, status).replace(flags::Status::NEED_DATA) };
            }
        }

        for port in node.ports.outputs_mut() {
            // Recycle buffers.
            for buf in &mut port.io_buffers {
                let status = unsafe { volatile!(buf.region, status).read() };
                let target_id = unsafe { volatile!(buf.region, buffer_id).read() };

                if status & Status::NEED_DATA && target_id >= 0 {
                    port.port_buffers.free(buf.mix_id, target_id as u32);
                }
            }

            let Some(format) = self.formats.get(&(port.direction, port.id)) else {
                continue;
            };

            if format.channels != 1 || format.format != AudioFormat::F32P || format.rate == 0 {
                tracing::warn!(?format, "Unsupported format on output port");
                continue;
            }

            let buf_id = {
                let mixes = port.io_buffers.iter().map(|b| b.mix_id);

                let Some(buffer) = port.port_buffers.next(mixes) else {
                    self.stats.no_output_buffer += 1;
                    continue;
                };

                let accumulator = self.accumulators.entry(port.id).or_default();

                let _ = &buffer.metas[0];
                let data = &mut buffer.datas[0];

                // 128 seems to be the number of samples expected by the peer I'm
                // using so YMMV.
                let samples = (data.region.len() / mem::size_of::<f32>()).min(duration as usize);

                unsafe {
                    let chunk = data.chunk.as_mut();

                    let mut region = data.region.cast_array::<f32>()?;

                    for d in region.as_slice_mut().iter_mut().take(samples) {
                        *d = accumulator.sin() * DEFAULT_VOLUME;
                        *accumulator += M_PI_M2 * TONE / format.rate as f32;

                        if *accumulator >= M_PI_M2 {
                            *accumulator -= M_PI_M2;
                        }
                    }

                    chunk.size = (samples * mem::size_of::<f32>()) as u32;
                    chunk.offset = 0;
                    chunk.stride = 4;
                }

                buffer.id
            };

            // Recycle buffers.
            for buf in &mut port.io_buffers {
                let status = unsafe { volatile!(buf.region, status).read() };

                if !(status & Status::NEED_DATA) && !(status & Status::OK) {
                    port.port_buffers.free(buf.mix_id, buf_id);
                    continue;
                }

                unsafe {
                    volatile!(buf.region, buffer_id).replace(buf_id as i32);
                    volatile!(buf.region, status).replace(flags::Status::HAVE_DATA);
                };
            }
        }

        let was_awake = unsafe { status.compare_exchange(Activation::AWAKE, Activation::FINISHED) };

        if was_awake {
            for a in &node.peer_activations {
                unsafe {
                    let signaled = a.trigger()?;

                    if signaled {
                        self.stats.succeeded += 1;
                        self.stats.succeeded_set.set(a.peer_id);
                    } else {
                        self.stats.failed += 1;
                        self.stats.failed_set.set(a.peer_id);
                    }
                }
            }
        }

        let now = utils::get_monotonic_nsec();
        self.stats.timing_sum += now.saturating_sub(then);
        self.stats.timing_count += 1;

        unsafe {
            let prev_finish_time = volatile!(na, finish_time).replace(then);
            volatile!(na, prev_finish_time).write(prev_finish_time);
        }

        Ok(())
    }

    /// Process client.
    #[tracing::instrument(skip_all)]
    pub fn tick(&mut self, stream: &mut Stream) -> Result<()> {
        for this in stream.nodes() {
            if let Some(na) = this.activation.as_ref() {
                unsafe {
                    let state = volatile!(na, state[0]).read();
                    let driver_id = volatile!(na, driver_id).read();
                    let active_driver_id = volatile!(na, active_driver_id).read();
                    let flags = volatile!(na, flags).read();
                    tracing::warn!(?this.read_fd, ?state, ?driver_id, ?active_driver_id, ?flags);
                }
            }

            for peer in &this.peer_activations {
                let activation = unsafe { peer.region.read() };
                tracing::warn!(?peer.peer_id, ?peer.signal_fd, activation.status = ?activation.status, activation.state = ?activation.state[0]);
            }
        }

        for (&(port_id, mix_id), b) in &mut self.inputs {
            if b.format.format != AudioFormat::F32P {
                b.buf.clear();
                continue;
            }

            let spec = hound::WavSpec {
                channels: b.format.channels as u16,
                sample_rate: b.format.rate,
                bits_per_sample: 32,
                sample_format: hound::SampleFormat::Float,
            };

            if b.buf.len() > 0 {
                let file = PathBuf::from(format!("capture_{port_id}_{mix_id}.wav"));

                let mut writer = 'writer: {
                    if !file.is_file() {
                        break 'writer hound::WavWriter::new(
                            BufWriter::new(File::create(&file)?),
                            spec,
                        )?;
                    }

                    let writer = hound::WavWriter::append(&file)?;

                    if writer.spec() == spec {
                        break 'writer writer;
                    }

                    tracing::warn!(?file, "File format mismatch, overwriting");
                    hound::WavWriter::new(BufWriter::new(File::create(&file)?), spec)?
                };

                let mut samples = 0;
                let mut sum = 0.0;

                for sample in b.buf.drain(..) {
                    writer.write_sample(sample)?;
                    sum += sample;
                    samples += 1;
                }

                tracing::warn!(?file, samples, sum, len = writer.len(), "Wrote");
                writer.finalize()?;
            }
        }

        let stats = &mut self.stats;

        if stats.non_ready > 0 {
            tracing::warn!(stats.non_ready, ?stats.non_ready_set);
            stats.non_ready = 0;
            stats.non_ready_set.clear();
        }

        if stats.failed > 0 || stats.succeeded > 0 {
            tracing::warn!(stats.failed, stats.succeeded, ?stats.failed_set, ?stats.succeeded_set);
            stats.failed = 0;
            stats.failed_set.clear();
            stats.succeeded = 0;
            stats.succeeded_set.clear();
        }

        if stats.not_input_have_data > 0 {
            tracing::warn!(stats.not_input_have_data);
            stats.not_input_have_data = 0;
        }

        if stats.not_self_triggered > 0 {
            tracing::warn!(stats.not_self_triggered);
            stats.not_self_triggered = 0;
        }

        if stats.no_output_buffer > 0 {
            tracing::warn!(stats.no_output_buffer);
            stats.no_output_buffer = 0;
        }

        if stats.timing_count > 0 {
            let average_timing =
                Duration::from_nanos((stats.timing_sum as f64 / stats.timing_count as f64) as u64);
            tracing::warn!(stats.timing_count, stats.timing_sum, ?average_timing);
            stats.timing_count = 0;
            stats.timing_sum = 0;
        }

        Ok(())
    }
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::try_init().map_err(anyhow::Error::msg)?;

    let mut poll = Poll::new()?;

    let mut c = Connection::open()?;
    c.set_nonblocking(true)?;

    let timer = TimerFd::new()?;
    timer.set_nonblocking(true)?;
    timer.set_interval(Duration::from_secs(10))?;

    let mut stream = client::Stream::new(c)?;

    let timer_token = stream.token()?;
    poll.add(timer.as_raw_fd(), timer_token, Interest::READ)?;

    let mut events = ArrayVec::<PollEvent, 4>::new();
    let mut recv = RecvBuf::new();

    let stats = Stats::default();

    let mut app = ExampleApplication {
        tick: 0,
        formats: HashMap::new(),
        accumulators: HashMap::new(),
        inputs: HashMap::new(),
        stats,
    };

    loop {
        while let Some(ev) = stream.run(&mut poll, &mut recv)? {
            match ev {
                StreamEvent::NodeCreated(node) => {
                    let node = stream.node_mut(node)?;

                    let port = node.ports.insert(Direction::INPUT)?;
                    port.name = String::from("input");
                    add_port_params(port)?;

                    let port = node.ports.insert(Direction::OUTPUT)?;
                    port.name = String::from("output");
                    add_port_params(port)?;
                }
                StreamEvent::Process(node) => {
                    let node = stream.node_mut(node)?;
                    app.process(node).context("Processing node")?;
                }
                StreamEvent::SetPortParam(ev) => {
                    // Decode a received parameter.
                    match ev.param {
                        id::Param::FORMAT => {
                            let node = stream.node(ev.node_id)?;
                            let port = node.ports.get(ev.direction, ev.port_id)?;

                            if let [param] = port.get_param(ev.param) {
                                let format = param.value.as_ref().read::<object::Format>()?;

                                match format.media_type {
                                    MediaType::AUDIO => {
                                        let audio_format =
                                            param.value.as_ref().read::<object::AudioFormat>()?;
                                        app.formats
                                            .insert((ev.direction, ev.port_id), audio_format);
                                    }
                                    other => {
                                        tracing::error!(?other, "Unsupported media type on port");
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
                StreamEvent::RemovePortParam(ev) => match ev.param {
                    id::Param::FORMAT => {
                        tracing::info!(
                            "Removed format parameter from port {}/{}",
                            ev.direction,
                            ev.port_id
                        );
                        app.formats.remove(&(ev.direction, ev.port_id));
                    }
                    _ => {}
                },
                _ => {
                    // Other events, ignore.
                }
            }
        }

        poll.poll(&mut events)?;

        while let Some(e) = events.pop() {
            if e.token == timer_token {
                if e.interest.is_read() {
                    timer.read().context("reading the timer")?;
                    app.tick(&mut stream)?;
                }

                continue;
            }

            stream.drive(&mut recv, e)?;
        }
    }
}

fn add_port_params(port: &mut Port) -> Result<()> {
    let mut pod = pod::array();

    let value = pod
        .clear_mut()
        .embed_object(ObjectType::FORMAT, Param::ENUM_FORMAT, |obj| {
            obj.property(Format::MEDIA_TYPE).write(MediaType::AUDIO)?;
            obj.property(Format::MEDIA_SUB_TYPE)
                .write(MediaSubType::DSP)?;
            obj.property(Format::AUDIO_FORMAT).write_choice(
                ChoiceType::ENUM,
                Type::ID,
                |choice| choice.write((AudioFormat::S16, AudioFormat::F32, AudioFormat::F32P)),
            )?;
            obj.property(Format::AUDIO_CHANNELS).write(1)?;
            obj.property(Format::AUDIO_RATE)
                .write_choice(ChoiceType::RANGE, Type::INT, |c| {
                    c.write((DEFAULT_RATE as u32, 44100, 48000))
                })?;
            Ok(())
        })?;

    port.set_param(Param::ENUM_FORMAT, [PortParam::new(value)])?;

    let value = pod
        .clear_mut()
        .embed_object(ObjectType::PARAM_META, Param::META, |obj| {
            obj.property(ParamMeta::TYPE).write(Meta::HEADER)?;
            obj.property(ParamMeta::SIZE)
                .write(mem::size_of::<ffi::MetaHeader>())?;
            Ok(())
        })?;

    port.set_param(Param::META, [PortParam::new(value)])?;

    let value = pod
        .clear_mut()
        .embed_object(ObjectType::PARAM_IO, Param::IO, |obj| {
            obj.property(ParamIo::ID).write(IoType::BUFFERS)?;
            obj.property(ParamIo::SIZE)
                .write(mem::size_of::<ffi::IoBuffers>())?;
            Ok(())
        })?;

    port.push_param(Param::IO, PortParam::new(value))?;

    let value = pod
        .clear_mut()
        .embed_object(ObjectType::PARAM_IO, Param::IO, |obj| {
            obj.property(ParamIo::ID).write(IoType::CLOCK)?;
            obj.property(ParamIo::SIZE)
                .write(mem::size_of::<ffi::IoClock>())?;
            Ok(())
        })?;

    port.push_param(Param::IO, PortParam::new(value))?;

    let value = pod
        .clear_mut()
        .embed_object(ObjectType::PARAM_IO, Param::IO, |obj| {
            obj.property(ParamIo::ID).write(IoType::POSITION)?;
            obj.property(ParamIo::SIZE)
                .write(mem::size_of::<ffi::IoPosition>())?;
            Ok(())
        })?;

    port.push_param(Param::IO, PortParam::new(value))?;

    let value = pod
        .clear_mut()
        .embed_object(ObjectType::PARAM_BUFFERS, Param::BUFFERS, |obj| {
            obj.property(ParamBuffers::BUFFERS).write_choice(
                ChoiceType::RANGE,
                Type::INT,
                |choice| choice.write((1, 1, 32)),
            )?;

            obj.property(ParamBuffers::BLOCKS).write(1i32)?;

            obj.property(ParamBuffers::SIZE).write_choice(
                ChoiceType::RANGE,
                Type::INT,
                |choice| {
                    choice.write((BUFFER_SAMPLES * mem::size_of::<f32>() as u32, 32, i32::MAX))
                },
            )?;

            obj.property(ParamBuffers::STRIDE)
                .write(mem::size_of::<f32>())?;
            Ok(())
        })?;

    port.set_param(Param::BUFFERS, [PortParam::new(value)])?;
    port.set_write(Param::FORMAT);
    Ok(())
}
