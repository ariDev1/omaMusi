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
fn build_typed<T: cpal::SizedSample + FromSample<f32> + 'static>(
    device: &cpal::Device,
    config: &SupportedStreamConfig,
    pcm: Consumer<f32>,
    decode: Arc<DecodeState>,
    shared: Arc<SharedOutput>,
) -> Result<Stream> {
    let channels = config.channels() as usize;
    let rate = config.sample_rate();
    let failure = shared.clone();
    let mut stream_config = config.config();
    let desired = (rate / 50).max(1);
    if let cpal::SupportedBufferSize::Range { min, max } = *config.buffer_size() {
        stream_config.buffer_size = cpal::BufferSize::Fixed(desired.clamp(min, max));
    }
    Ok(device.build_output_stream(
        stream_config,
        {
            let mut consume = consumer_callback(pcm, decode, shared, channels, rate);
            move |data: &mut [T], info| consume(data, info.timestamp())
        },
        move |_error| {
            failure.failed.store(true, Ordering::Release);
        },
        Some(std::time::Duration::from_secs(3)),
    )?)
}

fn consumer_callback<T: cpal::SizedSample + FromSample<f32>>(
    mut pcm: Consumer<f32>,
    decode: Arc<DecodeState>,
    shared: Arc<SharedOutput>,
    channels: usize,
    rate: u32,
) -> impl FnMut(&mut [T], cpal::OutputStreamTimestamp) {
    let mut last_playback_end = None;
    move |data: &mut [T], timestamp: cpal::OutputStreamTimestamp| {
        let zero = T::from_sample(0.0);
        data.fill(zero);
        if shared.paused.load(Ordering::Acquire)
            || shared.failed.load(Ordering::Acquire)
            || shared.ended.load(Ordering::Acquire)
        {
            return;
        }
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
        for (index, frame) in data.chunks_exact_mut(channels).enumerate() {
            if pcm.slots() >= channels {
                let mut mono = 0.0;
                for sample in frame {
                    let raw = pcm.pop().unwrap();
                    mono += raw;
                    *sample = T::from_sample((raw * volume).clamp(-1.0, 1.0));
                }
                shared.analysis[head].store((mono / channels as f32).to_bits(), Ordering::Relaxed);
                head = (head + 1) % 4096;
                frames += 1;
                last_playback_end =
                    timestamp
                        .playback
                        .checked_add(std::time::Duration::from_secs_f64(
                            (index + 1) as f64 / rate as f64,
                        ));
            }
        }
        // Silence in this buffer is scheduled in the future. Only the callback
        // clock can prove the final queued PCM frame has reached the device.
        if decode.done.load(Ordering::Acquire)
            && pcm.slots() < channels
            && last_playback_end.is_none_or(|end| timestamp.callback >= end)
        {
            shared.ended.store(true, Ordering::Release);
        }
        shared.analysis_head.store(head as u64, Ordering::Relaxed);
        shared.frames.fetch_add(frames, Ordering::Release);
        if frames > 0 {
            shared.revision.fetch_add(1, Ordering::Relaxed);
        }
        shared.sequence.fetch_add(1, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn eof_waits_for_scheduled_last_pcm_in_partial_and_ahead_buffers() {
        use cpal::{OutputStreamTimestamp, StreamInstant};
        use std::time::Duration;
        // A 10-ms final tone in a 20-ms callback, varied device latency.
        for latency_ms in [0, 5, 40, 100] {
            let (mut producer, pcm) = rtrb::RingBuffer::new(40);
            for _ in 0..20 {
                producer.push(0.5).unwrap();
            }
            let decode = Arc::new(DecodeState::new());
            decode.done.store(true, Ordering::Release);
            let shared = Arc::new(SharedOutput::new(1.0, false));
            let mut callback = consumer_callback(pcm, decode, shared.clone(), 2, 1000);
            let base = StreamInstant::from_millis(1000);
            let mut data = [0f32; 40];
            // Several callbacks can be requested ahead of actual device time.
            for (callback_ms, playback_ms) in
                [(0, latency_ms), (0, latency_ms + 20), (1, latency_ms + 40)]
            {
                callback(
                    &mut data,
                    OutputStreamTimestamp {
                        callback: base + Duration::from_millis(callback_ms),
                        playback: base + Duration::from_millis(playback_ms),
                    },
                );
                assert!(
                    !shared.ended.load(Ordering::Acquire),
                    "latency {latency_ms}: future silence must not finish queued audio"
                );
            }
            callback(
                &mut data,
                OutputStreamTimestamp {
                    callback: base + Duration::from_millis(latency_ms + 10),
                    playback: base + Duration::from_millis(latency_ms + 60),
                },
            );
            assert!(shared.ended.load(Ordering::Acquire));
            assert_eq!(shared.frames.load(Ordering::Acquire), 10);
        }
    }
}
