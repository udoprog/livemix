use std::collections::HashMap;
use std::fs::File;
use std::mem;
use std::os::fd::AsRawFd;
use std::time::{Duration, SystemTime};

use anyhow::{Context, Result, bail};
use client::events::StreamEvent;
use client::ptr::{atomic, volatile};
use client::{ClientNode, Port, PortId, PortParam};
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
const DEBUG_INTERVAL: usize = 100;

struct ExampleApplication {
    tick: usize,
    failed_signals: usize,
    buf: Vec<f32>,
    writer: hound::WavWriter<File>,
    accumulator: f32,
    formats: HashMap<(Direction, PortId), object::AudioFormat>,
    /// At the beginning of processing we check whether or not peers have a
    /// pending value greater than 0, and have a status of NOT_TRIGGERED.
    ///
    /// If that is not the case, we increment this counter add add the peer to the bitset.
    non_ready_peers: usize,
    non_ready_peers_bitset: IdSet,
}

impl ExampleApplication {
    #[tracing::instrument(skip(self, node))]
    fn process(&mut self, node: &mut ClientNode) -> Result<()> {
        self.tick = self.tick.wrapping_add(1);

        let mut start = None;

        if self.tick % DEBUG_INTERVAL == 0 {
            start = Some(SystemTime::now());
        }

        if let Some(this) = &node.activation {
            unsafe {
                let pending = atomic!(this, state[0].pending).load();
                let status = atomic!(this, status).load();

                if pending != 0 || status != ActivationStatus::TRIGGERED {
                    tracing::info!(?pending, ?status, "this");
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
                tracing::warn!(?previous_status, "Expected TRIGGERED");
            }

            if self.tick % DEBUG_INTERVAL == 0 {
                let xrun_count = unsafe { volatile!(activation, xrun_count).read() };
                let signal_time = unsafe { volatile!(activation, signal_time).read() };
                let now = client::utils::get_monotonic_nsec();
                tracing::warn!(xrun_count, signal_time, now);
            }
        }

        let Some(io_position) = &node.io_position else {
            tracing::error!("Missing IO position");
            return Ok(());
        };

        let clock = unsafe { volatile!(io_position, clock).read() };
        let duration = clock.duration;

        for port in node.ports.inputs_mut() {
            let Some(io_buffers) = &mut port.io_buffers else {
                continue;
            };

            let status = unsafe { volatile!(io_buffers, status).read() };

            tracing::warn!(?status);

            if !(status & Status::HAVE_DATA) {
                continue;
            }

            let Some(format) = self.formats.get(&(port.direction, port.id)) else {
                continue;
            };

            if format.rate != DEFAULT_RATE {
                tracing::warn!(?format, "Unsupported rate on input port");
                continue;
            }

            let id = unsafe { volatile!(io_buffers, buffer_id).read() };

            let Some(buffer) = port.buffers.get_mut(id as u32) else {
                bail!("Input no buffer with id {id} for port {}", port.id);
            };

            let _ = &buffer.metas[0];
            let data = &buffer.datas[0];

            let samples;

            unsafe {
                let chunk = data.chunk.as_ref();
                let offset = chunk.offset as usize % data.max_size;
                let size = (chunk.size as usize - offset).min(data.max_size);

                samples = size / mem::size_of::<f32>();

                self.buf.reserve(samples);

                self.buf
                    .as_mut_ptr()
                    .add(self.buf.len())
                    .copy_from_nonoverlapping(
                        data.region.as_ptr().wrapping_add(offset).cast::<f32>(),
                        samples,
                    );

                self.buf.set_len(self.buf.len() + samples);
            }

            let old_read =
                unsafe { volatile!(io_buffers, status).replace(flags::Status::NEED_DATA) };

            if self.tick % DEBUG_INTERVAL == 0 {
                tracing::warn!(?data.flags, ?id, ?old_read, samples);
            }
        }

        for port in node.ports.outputs_mut() {
            let Some(io_buffers) = &mut port.io_buffers else {
                continue;
            };

            let Some(format) = self.formats.get(&(port.direction, port.id)) else {
                continue;
            };

            if format.channels != 1 || format.format != AudioFormat::F32P || format.rate == 0 {
                tracing::warn!(?format, "Unsupported format on output port");
                continue;
            }

            let status = unsafe { volatile!(io_buffers, status).read() };
            let target_id = unsafe { volatile!(io_buffers, buffer_id).read() };

            if status & Status::NEED_DATA && target_id > 0 {
                port.buffers.free(target_id as u32);
            }

            if status & Status::HAVE_DATA {
                tracing::warn!(?port.direction, ?port.id, ?status);
                continue;
            }

            let Some(buffer) = port.buffers.next() else {
                tracing::error!("No available buffers");
                continue;
            };

            let _ = &buffer.metas[0];
            let data = &mut buffer.datas[0];

            // 128 seems to be the number of samples expected by the peer I'm
            // using so YMMV.
            let samples = (data.region.len() / mem::size_of::<f32>()).min(duration as usize);

            unsafe {
                let chunk = data.chunk.as_mut();

                let mut region = data.region.cast_array::<f32>()?;

                for d in region.as_slice_mut().iter_mut().take(samples) {
                    *d = self.accumulator.sin() * DEFAULT_VOLUME;
                    self.accumulator += M_PI_M2 * TONE / format.rate as f32;

                    if self.accumulator >= M_PI_M2 {
                        self.accumulator -= M_PI_M2;
                    }
                }

                chunk.size = (samples * mem::size_of::<f32>()) as u32;
                chunk.offset = 0;
                chunk.stride = 4;

                volatile!(io_buffers, buffer_id).replace(buffer.id as i32);
                volatile!(io_buffers, status).replace(flags::Status::HAVE_DATA);
            };

            if self.tick % DEBUG_INTERVAL == 0 {
                tracing::warn!(?data.flags, samples);
            }
        }

        for a in &node.peer_activations {
            unsafe {
                let signaled = a.signal()?;

                if signaled {
                    if self.failed_signals > 0 {
                        tracing::warn!(self.failed_signals, signaled);
                    }

                    self.failed_signals = 0;
                } else {
                    self.failed_signals += 1;
                }
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

        if self.tick % DEBUG_INTERVAL == 0 {
            if self.non_ready_peers > 0 {
                tracing::warn!(self.non_ready_peers, ?self.non_ready_peers_bitset, "Peer activation is not ready");
                self.non_ready_peers = 0;
                self.non_ready_peers_bitset.clear();
            }
        }

        if let Some(start) = start {
            let elapsed = start.elapsed().context("Elapsed time")?;
            tracing::warn!(?elapsed);
        }

        Ok(())
    }

    /// Process client.
    #[tracing::instrument(skip(self))]
    pub fn tick(&mut self) -> Result<()> {
        let mut samples = 0;
        let mut sum = 0.0;

        for sample in self.buf.drain(..) {
            self.writer.write_sample(sample)?;
            sum += sample;
            samples += 1;
        }

        self.writer.flush()?;
        tracing::trace!(samples, sum, len = self.writer.len(), "Flushed to file");
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

    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 48000,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };

    let writer = hound::WavWriter::new(File::create("capture.wav")?, spec)?;

    let mut app = ExampleApplication {
        tick: 0,
        failed_signals: 0,
        buf: Vec::with_capacity(1024 * 1024),
        writer,
        accumulator: 0.0,
        formats: HashMap::new(),
        non_ready_peers: 0,
        non_ready_peers_bitset: IdSet::new(),
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
                    app.tick()?;
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
