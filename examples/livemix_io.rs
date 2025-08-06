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
use protocol::consts::{ActivationStatus, Direction};
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

struct ExampleApplication {
    tick: usize,
    formats: HashMap<(Direction, PortId), object::AudioFormat>,
    accumulators: HashMap<PortId, f32>,
    inputs: HashMap<(PortId, MixId), InputBuffer>,
    /// At the beginning of processing we check whether or not peers have a
    /// pending value greater than 0, and have a status of NOT_TRIGGERED.
    ///
    /// If that is not the case, we increment this counter add add the peer to the bitset.
    non_ready_peers: usize,
    non_ready_peers_bitset: IdSet,
    not_triggered: usize,
    not_input_have_data: usize,
    not_self_triggered: usize,
    no_output_buffer: usize,
    failed_signals: usize,
    timing_sum: u64,
    timing_count: usize,
    debug_activations: bool,
}

impl ExampleApplication {
    #[tracing::instrument(skip(self, node))]
    fn process(&mut self, node: &mut ClientNode) -> Result<()> {
        self.tick = self.tick.wrapping_add(1);

        if self.debug_activations && self.tick % 100 == 0 {
            if let Some(a) = &node.activation {
                unsafe {
                    let pending = atomic!(a, state[0].pending).load();
                    let required = atomic!(a, state[0].required).load();
                    tracing::warn!(?pending, ?required, "this");
                }
            }

            for a in &node.peer_activations {
                unsafe {
                    let pending = atomic!(a.region, state[0].pending).load();
                    let required = atomic!(a.region, state[0].required).load();
                    tracing::warn!(?a.peer_id, ?pending, ?required);
                }
            }
        }

        let then = utils::get_monotonic_nsec();

        if let Some(this) = &node.activation {
            unsafe {
                let pending = atomic!(this, state[0].pending).load();
                let status = atomic!(this, status).load();

                if pending != 0 || status != ActivationStatus::TRIGGERED {
                    self.not_self_triggered += 1;
                }
            }
        }

        for a in &node.peer_activations {
            unsafe {
                let pending = atomic!(a.region, state[0].pending).load();
                let status = atomic!(a.region, status).load();

                if pending == 0 || status != ActivationStatus::NOT_TRIGGERED {
                    self.non_ready_peers += 1;
                    self.non_ready_peers_bitset.set(a.peer_id);
                }
            }
        }

        if let Some(activation) = &node.activation {
            let previous_status =
                unsafe { atomic!(activation, status).swap(ActivationStatus::AWAKE) };

            if previous_status != ActivationStatus::TRIGGERED {
                self.not_triggered += 1;
            }
        }

        let Some(io_position) = &node.io_position else {
            tracing::error!("Missing IO position");
            return Ok(());
        };

        let clock = unsafe { volatile!(io_position, clock).read() };
        let duration = clock.duration;

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
                    self.not_input_have_data += 1;
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
            let Some(format) = self.formats.get(&(port.direction, port.id)) else {
                continue;
            };

            if format.channels != 1 || format.format != AudioFormat::F32P || format.rate == 0 {
                tracing::warn!(?format, "Unsupported format on output port");
                continue;
            }

            // Recycle buffers.
            for buf in &mut port.io_buffers {
                let status = unsafe { volatile!(buf.region, status).read() };
                let target_id = unsafe { volatile!(buf.region, buffer_id).read() };

                if status & Status::NEED_DATA && target_id > 0 {
                    port.port_buffers.free(target_id as u32, buf.mix_id);
                }

                let Some(buffer) = port.port_buffers.next(buf.mix_id) else {
                    self.no_output_buffer += 1;
                    continue;
                };

                let mut accumulator = *self.accumulators.entry(port.id).or_default();

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
                        accumulator += M_PI_M2 * TONE / format.rate as f32;

                        if accumulator >= M_PI_M2 {
                            accumulator -= M_PI_M2;
                        }
                    }

                    chunk.size = (samples * mem::size_of::<f32>()) as u32;
                    chunk.offset = 0;
                    chunk.stride = 4;
                }

                unsafe {
                    volatile!(buf.region, buffer_id).replace(buffer.id as i32);
                    volatile!(buf.region, status).replace(flags::Status::HAVE_DATA);
                };
            }

            let accumulator = self.accumulators.entry(port.id).or_default();

            *accumulator += (M_PI_M2 * TONE / format.rate as f32) * (duration as f32);
            *accumulator %= M_PI_M2;
        }

        for a in &node.peer_activations {
            unsafe {
                let signaled = a.signal()?;
                self.failed_signals += usize::from(!signaled);
            }
        }

        // Set activation to NOT_TRIGGERED indicating we are ready to be
        // scheduled again.
        if let Some(activation) = &node.activation {
            let previous_status =
                unsafe { atomic!(activation, status).swap(ActivationStatus::NOT_TRIGGERED) };

            if previous_status != ActivationStatus::AWAKE {
                tracing::warn!(?previous_status, "Expected AWAKE");
            }
        }

        let now = utils::get_monotonic_nsec();
        self.timing_sum += now.saturating_sub(then);
        self.timing_count += 1;
        Ok(())
    }

    /// Process client.
    #[tracing::instrument(skip_all)]
    pub fn tick(&mut self, _stream: &mut Stream) -> Result<()> {
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

                tracing::info!(?file, samples, sum, len = writer.len(), "Wrote");
                writer.finalize()?;
            }
        }

        if self.failed_signals > 0 || self.non_ready_peers > 0 {
            tracing::warn!(self.failed_signals, self.non_ready_peers, ?self.non_ready_peers_bitset);
            self.failed_signals = 0;
            self.non_ready_peers = 0;
            self.non_ready_peers_bitset.clear();
        }

        if self.not_triggered > 0 {
            tracing::warn!(self.not_triggered);
            self.not_triggered = 0;
        }

        if self.not_input_have_data > 0 {
            tracing::warn!(self.not_input_have_data);
            self.not_input_have_data = 0;
        }

        if self.not_self_triggered > 0 {
            tracing::warn!(self.not_self_triggered);
            self.not_self_triggered = 0;
        }

        if self.no_output_buffer > 0 {
            tracing::warn!(self.no_output_buffer);
            self.no_output_buffer = 0;
        }

        if self.timing_count > 0 {
            let average_timing =
                Duration::from_nanos((self.timing_sum as f64 / self.timing_count as f64) as u64);
            tracing::warn!(self.timing_count, self.timing_sum, ?average_timing);
            self.timing_count = 0;
            self.timing_sum = 0;
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

    let mut app = ExampleApplication {
        tick: 0,
        formats: HashMap::new(),
        accumulators: HashMap::new(),
        inputs: HashMap::new(),
        non_ready_peers: 0,
        non_ready_peers_bitset: IdSet::new(),
        not_triggered: 0,
        not_input_have_data: 0,
        not_self_triggered: 0,
        no_output_buffer: 0,
        failed_signals: 0,
        timing_sum: 0,
        timing_count: 0,
        debug_activations: false,
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
