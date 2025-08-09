use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::fs::File;
use std::io::BufWriter;
use std::mem::{self, MaybeUninit};
use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use client::events::StreamEvent;
use client::{ClientNode, MixId, Port, PortId, PortParam, Stats, Stream};
use pod::buf::ArrayVec;
use pod::{ChoiceType, Readable, Type, Writable};
use protocol::buf::RecvBuf;
use protocol::consts::Direction;
use protocol::flags::ChunkFlags;
use protocol::id::{
    self, AudioFormat, FormatKey, IoType, MediaSubType, MediaType, Meta, ObjectType, Param,
    ParamBuffersKey, ParamIoKey, ParamMetaKey,
};
use protocol::poll::{Interest, PollEvent};
use protocol::{Connection, Poll, TimerFd, ffi, object};

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
    stats: Stats,
}

impl ExampleApplication {
    #[tracing::instrument(skip(self, node))]
    fn process(&mut self, node: &mut ClientNode) -> Result<()> {
        self.tick = self.tick.wrapping_add(1);
        node.start_process()?;

        let Some(duration) = node.duration() else {
            bail!("Clock duration is not configured on node")
        };

        for port in node.ports.inputs_mut() {
            let Some(format) = self.formats.get(&(port.direction, port.id)) else {
                continue;
            };

            if format.channels != 1 || format.format != AudioFormat::F32P || format.rate == 0 {
                tracing::warn!(?format, "Unsupported format on output port");
                continue;
            }

            for mix in port.mixes.iter_mut() {
                let Some(mut ib) = port.port_buffers.next_input(mix) else {
                    self.stats.no_input_buffer += 1;
                    continue;
                };

                let b = match self.inputs.entry((port.id, ib.mix_id())) {
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

                let buffer = ib.buffer_mut();
                let _ = &buffer.metas[0];
                let data = &buffer.datas[0];

                unsafe {
                    let Some(region) = data.valid_region() else {
                        bail!("No valid memory region");
                    };

                    let region = region.cast_array::<f32>()?;

                    b.buf.reserve(region.len());

                    b.buf
                        .as_mut_ptr()
                        .add(b.buf.len())
                        .copy_from_nonoverlapping(region.as_ptr(), region.len());

                    b.buf.set_len(b.buf.len() + region.len());
                }

                ib.need_data()?;
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

            let Some(mut ob) = port.port_buffers.next_output(&mut port.mixes) else {
                self.stats.no_output_buffer += 1;
                continue;
            };

            let accumulator = self.accumulators.entry(port.id).or_default();

            let b = ob.buffer_mut();

            let _ = &b.metas[0];
            let data = &mut b.datas[0];

            let mut region = data.uninit_region().cast_array::<MaybeUninit<f32>>()?;
            let samples = region.len().min(duration as usize);

            for d in region.as_slice_mut().iter_mut().take(samples) {
                d.write(accumulator.sin() * DEFAULT_VOLUME);
                *accumulator += M_PI_M2 * TONE / format.rate as f32;

                if *accumulator >= M_PI_M2 {
                    *accumulator -= M_PI_M2;
                }
            }

            data.write_chunk(ffi::Chunk {
                size: u32::try_from(samples.saturating_mul(mem::size_of::<f32>()))
                    .unwrap_or(u32::MAX),
                offset: 0,
                stride: 4,
                flags: ChunkFlags::NONE,
            });

            ob.have_data()?;
        }

        node.end_process()?;
        Ok(())
    }

    /// Process client.
    #[tracing::instrument(skip_all)]
    pub fn tick(&mut self, stream: &mut Stream) -> Result<()> {
        for this in stream.nodes_mut() {
            self.stats.merge(this.stats_mut());
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

        self.stats.report();
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
    #[derive(Readable, Writable)]
    #[pod(object(type = ObjectType::PARAM_IO, id = Param::IO))]
    struct ParamIo {
        #[pod(property(key = ParamIoKey::ID))]
        ty: IoType,
        #[pod(property(key = ParamIoKey::SIZE))]
        size: usize,
    }

    #[derive(Readable, Writable)]
    #[pod(object(type = ObjectType::PARAM_META, id = Param::META))]
    struct ParamMeta {
        #[pod(property(key = ParamMetaKey::TYPE))]
        ty: Meta,
        #[pod(property(key = ParamMetaKey::SIZE))]
        size: usize,
    }

    let mut pod = pod::array();

    let value = pod
        .clear_mut()
        .embed_object(ObjectType::FORMAT, Param::ENUM_FORMAT, |obj| {
            obj.property(FormatKey::MEDIA_TYPE)
                .write(MediaType::AUDIO)?;
            obj.property(FormatKey::MEDIA_SUB_TYPE)
                .write(MediaSubType::DSP)?;
            obj.property(FormatKey::AUDIO_FORMAT).write_choice(
                ChoiceType::ENUM,
                Type::ID,
                |choice| choice.write((AudioFormat::S16, AudioFormat::F32, AudioFormat::F32P)),
            )?;
            obj.property(FormatKey::AUDIO_CHANNELS).write(1)?;
            obj.property(FormatKey::AUDIO_RATE).write_choice(
                ChoiceType::RANGE,
                Type::INT,
                |c| c.write((DEFAULT_RATE as u32, 44100, 48000)),
            )?;
            Ok(())
        })?;

    port.set_param(Param::ENUM_FORMAT, [PortParam::new(value)])?;

    port.push_param(
        Param::META,
        PortParam::new(pod.clear_mut().embed(ParamMeta {
            ty: Meta::HEADER,
            size: mem::size_of::<ffi::MetaHeader>(),
        })?),
    )?;

    port.push_param(
        Param::IO,
        PortParam::new(pod.clear_mut().embed(ParamIo {
            ty: IoType::BUFFERS,
            size: mem::size_of::<ffi::IoBuffers>(),
        })?),
    )?;

    port.push_param(
        Param::IO,
        PortParam::new(pod.clear_mut().embed(ParamIo {
            ty: IoType::CLOCK,
            size: mem::size_of::<ffi::IoClock>(),
        })?),
    )?;

    port.push_param(
        Param::IO,
        PortParam::new(pod.clear_mut().embed(ParamIo {
            ty: IoType::POSITION,
            size: mem::size_of::<ffi::IoPosition>(),
        })?),
    )?;

    let value = pod
        .clear_mut()
        .embed_object(ObjectType::PARAM_BUFFERS, Param::BUFFERS, |obj| {
            obj.property(ParamBuffersKey::BUFFERS).write_choice(
                ChoiceType::RANGE,
                Type::INT,
                |choice| choice.write((1, 1, 32)),
            )?;

            obj.property(ParamBuffersKey::BLOCKS).write(1i32)?;

            obj.property(ParamBuffersKey::SIZE).write_choice(
                ChoiceType::RANGE,
                Type::INT,
                |choice| {
                    choice.write((BUFFER_SAMPLES * mem::size_of::<f32>() as u32, 32, i32::MAX))
                },
            )?;

            obj.property(ParamBuffersKey::STRIDE)
                .write(mem::size_of::<f32>())?;
            Ok(())
        })?;

    port.set_param(Param::BUFFERS, [PortParam::new(value)])?;
    port.set_write(Param::FORMAT);
    Ok(())
}
