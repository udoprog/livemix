use std::fs::File;
use std::mem;
use std::os::fd::AsRawFd;
use std::time::{Duration, SystemTime};

use anyhow::{Context, Result, bail};
use client::ptr::{atomic, volatile};
use client::{ClientNode, Port, PortParam, StreamEvent};
use pod::buf::ArrayVec;
use pod::{ChoiceType, Type};
use protocol::buf::RecvBuf;
use protocol::consts::{ActivationStatus, Direction};
use protocol::flags::Status;
use protocol::id::{
    AudioFormat, Format, IoType, MediaSubType, MediaType, Meta, ObjectType, Param, ParamBuffers,
    ParamIo, ParamMeta,
};
use protocol::poll::{Interest, PollEvent};
use protocol::{Connection, Poll, TimerFd, ffi, flags};

const BUFFER_SAMPLES: u32 = 128;
const M_PI_M2: f32 = std::f32::consts::PI * 2.0;
const DEFAULT_RATE: f32 = 48000.0;
const DEFAULT_VOLUME: f32 = 0.5;

struct ExampleApplication {
    tick: usize,
    buf: Vec<f32>,
    writer: hound::WavWriter<File>,
    accumulator: f32,
}

impl ExampleApplication {
    #[tracing::instrument(skip(self, node))]
    fn process(&mut self, node: &mut ClientNode) -> Result<()> {
        self.tick = self.tick.wrapping_add(1);

        let mut start = None;

        if self.tick % 100 == 0 {
            start = Some(SystemTime::now());
        }

        if let Some(activation) = &node.activation {
            let previous_status =
                unsafe { atomic!(activation, status).swap(ActivationStatus::AWAKE) };

            if previous_status != ActivationStatus::TRIGGERED {
                tracing::warn!(?previous_status, "Expected TRIGGERED");
            }

            if self.tick % 100 == 0 {
                let xrun_count = unsafe { volatile!(activation, xrun_count).read() };
                let signal_time = unsafe { volatile!(activation, signal_time).read() };
                tracing::warn!(xrun_count, signal_time);
            }
        }

        for port in node.ports.inputs_mut() {
            let (Some(buffers), Some(io_buffers)) = (&port.buffers, &port.io_buffers) else {
                continue;
            };

            let status = unsafe { volatile!(io_buffers, status).read() };

            if status != Status::HAVE_DATA {
                continue;
            };

            let id = unsafe { volatile!(io_buffers, buffer_id).read() };

            let Some(buffer) = buffers.buffers.get(id as usize) else {
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

            if self.tick % 100 == 0 {
                tracing::warn!(?data.flags, ?id, ?old_read, samples);
            }
        }

        if true {
            for port in node.ports.outputs_mut() {
                let (Some(buffers), Some(io_buffers)) = (&mut port.buffers, &mut port.io_buffers)
                else {
                    continue;
                };

                let Some(buffer) = buffers.buffers.get_mut(self.tick % 2) else {
                    bail!("Output no buffer for port {}", port.id);
                };

                let _ = &buffer.metas[0];
                let data = &mut buffer.datas[0];

                let samples;

                unsafe {
                    let chunk = data.chunk.as_mut();
                    samples = data.max_size / mem::size_of::<f32>();

                    let mut ptr = data.region.as_mut_ptr().cast::<f32>();

                    for _ in 0..samples {
                        ptr.write(self.accumulator.sin() * DEFAULT_VOLUME);
                        self.accumulator += M_PI_M2 * 440.0 / DEFAULT_RATE;
                        ptr = ptr.wrapping_add(1);
                    }

                    chunk.size = (samples * mem::size_of::<f32>()) as u32;
                    chunk.offset = 0;
                    chunk.stride = 0;
                }

                unsafe {
                    volatile!(io_buffers, buffer_id).replace(buffer.id);
                    volatile!(io_buffers, status).replace(flags::Status::HAVE_DATA);
                };

                if self.tick % 100 == 0 {
                    tracing::warn!(?data.flags, samples);
                }
            }
        }

        for (_, a) in &node.peer_activations {
            // Signal peers that we are done.
            unsafe {
                a.signal()?;
            }
        }

        if let Some(activation) = &node.activation {
            let previous_status =
                unsafe { atomic!(activation, status).swap(ActivationStatus::NOT_TRIGGERED) };

            if previous_status != ActivationStatus::AWAKE {
                tracing::warn!(?previous_status, "Expected AWAKE");
            }
        }

        if let Some(start) = start {
            let elapsed = start.elapsed().context("Elapsed time")?;
            tracing::info!(?elapsed);
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
        tracing::warn!(samples, sum, len = self.writer.len(), "Flushed to file");
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
        buf: Vec::with_capacity(1024 * 1024),
        writer,
        accumulator: 0.0,
    };

    loop {
        while let Some(ev) = stream.run(&mut poll, &mut recv)? {
            match ev {
                StreamEvent::NodeCreated(node) => {
                    let node = stream.node_mut(node)?;

                    let mut pod = pod::array();

                    /*
                    let value = pod.clear_mut().embed_object(
                        ObjectType::FORMAT,
                        Param::ENUM_FORMAT,
                        |obj| {
                            obj.property(Format::MEDIA_TYPE).write(MediaType::AUDIO)?;
                            obj.property(Format::MEDIA_SUB_TYPE)
                                .write(MediaSubType::DSP)?;
                            obj.property(Format::AUDIO_FORMAT)
                                .write(AudioFormat::F32P)?;
                            obj.property(Format::AUDIO_CHANNELS).write(1)?;
                            obj.property(Format::AUDIO_RATE).write(48000)?;
                            Ok(())
                        },
                    )?;

                    node.set_param(Param::ENUM_FORMAT, [value])?;
                    */

                    let value =
                        pod.clear_mut()
                            .embed_object(ObjectType::FORMAT, Param::FORMAT, |obj| {
                                obj.property(Format::MEDIA_TYPE).write(MediaType::AUDIO)?;
                                obj.property(Format::MEDIA_SUB_TYPE)
                                    .write(MediaSubType::DSP)?;
                                obj.property(Format::AUDIO_FORMAT)
                                    .write(AudioFormat::F32P)?;
                                obj.property(Format::AUDIO_CHANNELS).write(1)?;
                                obj.property(Format::AUDIO_RATE).write(48000)?;
                                Ok(())
                            })?;

                    node.set_param(Param::FORMAT, [value])?;

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
                |choice| {
                    choice.write(AudioFormat::F32)?;
                    choice.write(AudioFormat::S16)?;
                    Ok(())
                },
            )?;
            obj.property(Format::AUDIO_CHANNELS).write(1)?;
            obj.property(Format::AUDIO_RATE).write(48000)?;
            Ok(())
        })?;

    port.set_param(Param::ENUM_FORMAT, [PortParam::new(value)])?;
    port.set_read(Param::ENUM_FORMAT);

    /*
    let value = pod
        .clear_mut()
        .embed_object(ObjectType::FORMAT, Param::FORMAT, |obj| {
            obj.property(Format::MEDIA_TYPE).write(MediaType::AUDIO)?;
            obj.property(Format::MEDIA_SUB_TYPE)
                .write(MediaSubType::DSP)?;
            obj.property(Format::AUDIO_FORMAT)
                .write(AudioFormat::F32P)?;
            obj.property(Format::AUDIO_CHANNELS).write(1)?;
            obj.property(Format::AUDIO_RATE).write(48000)?;
            Ok(())
        })?;

    port.set_param(Param::FORMAT, [PortParam::new(value)])?;
    */

    let value = pod
        .clear_mut()
        .embed_object(ObjectType::PARAM_META, Param::META, |obj| {
            obj.property(ParamMeta::TYPE).write(Meta::HEADER)?;
            obj.property(ParamMeta::SIZE)
                .write(mem::size_of::<ffi::MetaHeader>())?;
            Ok(())
        })?;

    port.set_param(Param::META, [PortParam::new(value)])?;
    port.set_read(Param::META);

    let value = pod
        .clear_mut()
        .embed_object(ObjectType::PARAM_IO, Param::IO, |obj| {
            obj.property(ParamIo::ID).write(IoType::BUFFERS)?;
            obj.property(ParamIo::SIZE)
                .write(mem::size_of::<ffi::IoBuffers>())?;
            Ok(())
        })?;

    port.add_param(Param::IO, PortParam::new(value))?;

    let value = pod
        .clear_mut()
        .embed_object(ObjectType::PARAM_IO, Param::IO, |obj| {
            obj.property(ParamIo::ID).write(IoType::CLOCK)?;
            obj.property(ParamIo::SIZE)
                .write(mem::size_of::<ffi::IoClock>())?;
            Ok(())
        })?;

    port.add_param(Param::IO, PortParam::new(value))?;

    let value = pod
        .clear_mut()
        .embed_object(ObjectType::PARAM_IO, Param::IO, |obj| {
            obj.property(ParamIo::ID).write(IoType::POSITION)?;
            obj.property(ParamIo::SIZE)
                .write(mem::size_of::<ffi::IoPosition>())?;
            Ok(())
        })?;

    port.add_param(Param::IO, PortParam::new(value))?;
    port.set_read(Param::IO);

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
    port.set_read(Param::BUFFERS);

    port.set_write(Param::FORMAT);
    Ok(())
}
