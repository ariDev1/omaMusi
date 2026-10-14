//! Preallocated, nonblocking audio consumer.
use super::decoder::DecodeState;
use anyhow::{Result, bail};
use cpal::{FromSample, SampleFormat, Stream, SupportedStreamConfig, traits::DeviceTrait};
use rtrb::Consumer;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
};

pub struct SharedOutput {
    pub paused: AtomicBool,
    pub volume: AtomicU32,
    pub frames: AtomicU64,
    pub latency_frames: AtomicU64,
    pub ended: AtomicBool,
    pub failed: AtomicBool,
    pub sequence: AtomicU64,
    pub revision: AtomicU64,
    pub analysis: [AtomicU32; 4096],
    pub analysis_head: AtomicU64,
}
impl SharedOutput {
    pub fn new(volume: f32, paused: bool) -> Self {
        Self {
            paused: AtomicBool::new(paused),
            volume: AtomicU32::new(volume.to_bits()),
            frames: AtomicU64::new(0),
            latency_frames: AtomicU64::new(0),
            ended: AtomicBool::new(false),
            failed: AtomicBool::new(false),
            sequence: AtomicU64::new(0),
            revision: AtomicU64::new(0),
            analysis: std::array::from_fn(|_| AtomicU32::new(0)),
            analysis_head: AtomicU64::new(0),
        }
    }
}

pub fn choose_config(device: &cpal::Device, source_rate: u32) -> Result<SupportedStreamConfig> {
    let configs: Vec<_> = device.supported_output_configs()?.collect();
    for format in [SampleFormat::F32, SampleFormat::I16] {
        if let Some(config) = configs.iter().find(|c| {
            c.channels() == 2
                && c.sample_format() == format
                && source_rate >= 8000
                && c.contains_rate(source_rate)
        }) {
            return Ok(config.with_sample_rate(source_rate));
        }
    }
    let default = device.default_output_config()?;
    for format in [SampleFormat::F32, SampleFormat::I16] {
        if let Some(config) = configs.iter().find(|c| {
            c.channels() == 2
                && c.sample_format() == format
                && c.contains_rate(default.sample_rate())
        }) {
            return Ok(config.with_sample_rate(default.sample_rate()));
        }
    }
    if matches!(
        default.sample_format(),
        SampleFormat::F32 | SampleFormat::I16 | SampleFormat::U16
    ) {
        return Ok(default);
    }
    bail!("No supported floating-point or 16-bit audio output format")
}

pub fn build(
    device: &cpal::Device,
    config: &SupportedStreamConfig,
    consumer: Consumer<f32>,
    decode: Arc<DecodeState>,
    shared: Arc<SharedOutput>,
) -> Result<Stream> {
    match config.sample_format() {
        SampleFormat::F32 => build_typed::<f32>(device, config, consumer, decode, shared),
        SampleFormat::I16 => build_typed::<i16>(device, config, consumer, decode, shared),
        SampleFormat::U16 => build_typed::<u16>(device, config, consumer, decode, shared),
        _ => bail!("Unsupported audio sample format"),
    }
}
fn build_typed<T: cpal::SizedSample + FromSample<f32>>(
    device: &cpal::Device,
    config: &SupportedStreamConfig,
    mut pcm: Consumer<f32>,
    decode: Arc<DecodeState>,
    shared: Arc<SharedOutput>,
) -> Result<Stream> {
    let channels = config.channels() as usize;
    let rate = config.sample_rate();
    let failure = shared.clone();
    let mut silence = 0u64;
    let mut stream_config = config.config();
    let desired = (rate / 50).max(1);
    if let cpal::SupportedBufferSize::Range { min, max } = *config.buffer_size() {
        stream_config.buffer_size = cpal::BufferSize::Fixed(desired.clamp(min, max));
    }
    Ok(device.build_output_stream(
        stream_config,
        move |data: &mut [T], info| {
            let zero = T::from_sample(0.0);
            data.fill(zero);
            if shared.paused.load(Ordering::Acquire)
                || shared.failed.load(Ordering::Acquire)
                || shared.ended.load(Ordering::Acquire)
            {
                return;
            }
            let timestamp = info.timestamp();
            let latency = (timestamp
                .playback
                .duration_since(timestamp.callback)
                .as_secs_f64()
                * rate as f64) as u64;
            shared.latency_frames.store(latency, Ordering::Release);
            let volume = f32::from_bits(shared.volume.load(Ordering::Relaxed));
            let mut frames = 0u64;
            let mut head = shared.analysis_head.load(Ordering::Relaxed) as usize;
            shared.sequence.fetch_add(1, Ordering::AcqRel);
            for frame in data.chunks_exact_mut(channels) {
                if pcm.slots() >= channels {
                    let mut mono = 0.0;
                    for sample in frame {
                        let raw = pcm.pop().unwrap();
                        mono += raw;
                        *sample = T::from_sample((raw * volume).clamp(-1.0, 1.0));
                    }
                    shared.analysis[head]
                        .store((mono / channels as f32).to_bits(), Ordering::Relaxed);
                    head = (head + 1) % 4096;
                    frames += 1;
                    silence = 0;
                } else if decode.done.load(Ordering::Acquire) {
                    silence += 1;
                    if silence >= latency.max(1) {
                        shared.ended.store(true, Ordering::Release);
                    }
                }
            }
            shared.analysis_head.store(head as u64, Ordering::Relaxed);
            shared.frames.fetch_add(frames, Ordering::Release);
            if frames > 0 {
                shared.revision.fetch_add(1, Ordering::Relaxed);
            }
            shared.sequence.fetch_add(1, Ordering::Release);
        },
        move |_error| {
            failure.failed.store(true, Ordering::Release);
        },
        Some(std::time::Duration::from_secs(3)),
    )?)
}
