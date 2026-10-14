//! Native output, bounded decoding and immutable visual snapshots.
pub mod decoder;
mod output;
use crate::{player::TrackInfo, process};
use anyhow::{Context, Result, bail};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use decoder::Decoder;
use output::SharedOutput;
use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    sync::{Arc, atomic::Ordering},
};

pub fn probe(path: &Path) -> Result<TrackInfo> {
    if !path.is_file() {
        bail!("Audio file is unavailable: {}", path.display());
    }
    let data = process::ffprobe(path).unwrap_or(serde_json::Value::Null);
    let format = &data["format"];
    let tags = format["tags"]
        .as_object()
        .map(|tags| {
            tags.iter()
                .map(|(k, v)| (k.to_lowercase(), v.as_str().unwrap_or("").to_owned()))
                .collect::<std::collections::HashMap<_, _>>()
        })
        .unwrap_or_default();
    let stream = data["streams"]
        .as_array()
        .and_then(|v| v.iter().find(|s| s["codec_type"] == "audio"));
    let duration = format["duration"]
        .as_str()
        .and_then(|v| v.parse::<f64>().ok())
        .filter(|n| n.is_finite() && *n >= 0.0)
        .unwrap_or(0.0);
    let sample_rate = stream
        .and_then(|s| s["sample_rate"].as_str())
        .and_then(|s| s.parse::<u32>().ok())
        .filter(|n| *n > 0)
        .unwrap_or(48000);
    Ok(TrackInfo {
        duration,
        sample_rate,
        title: tags.get("title").cloned().unwrap_or_else(|| {
            path.file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned()
        }),
        artist: tags.get("artist").cloned().unwrap_or_default(),
        album: tags.get("album").cloned().unwrap_or_default(),
    })
}
#[derive(Clone, Debug)]
pub struct PlaybackSnapshot {
    pub frames_written: u64,
    pub latency_seconds: f64,
    pub position: f64,
    pub duration: f64,
    pub paused: bool,
    pub sample_rate: u32,
    pub volume: f32,
    pub generation: u64,
    pub has_stream: bool,
    pub error: Option<String>,
}
#[derive(Clone)]
pub struct PcmSnapshot {
    pub samples: [f32; 4096],
    pub sample_rate: u32,
    pub revision: u64,
    pub generation: u64,
}
#[derive(Clone, Debug, PartialEq)]
pub enum AudioEvent {
    Finished(u64),
    Failed(u64, String),
}
struct Session {
    stream: cpal::Stream,
    decoder: Decoder,
    shared: Arc<SharedOutput>,
    rate: u32,
    offset: f64,
    duration: f64,
    generation: u64,
    notified: bool,
}
pub struct AudioPlayer {
    host: cpal::Host,
    device: Option<cpal::Device>,
    session: Option<Session>,
    path: Option<PathBuf>,
    source_rate: u32,
    generation: u64,
    volume: f32,
    cached_samples: RefCell<PcmSnapshot>,
}

impl AudioPlayer {
    pub fn new() -> Result<Self> {
        let host = cpal::default_host();
        Ok(Self {
            host,
            device: None,
            session: None,
            path: None,
            source_rate: 48000,
            generation: 0,
            volume: 0.7,
            cached_samples: RefCell::new(PcmSnapshot {
                samples: [0.0; 4096],
                sample_rate: 48000,
                revision: 0,
                generation: 0,
            }),
        })
    }
    /// Bind playback to a selected output; default construction follows the desktop default.
    pub fn with_device(device: cpal::Device) -> Result<Self> {
        let mut player = Self::new()?;
        player.host = cpal::host_from_id(device.id()?.host())?;
        player.device = Some(device);
        Ok(player)
    }
    pub fn play(
        &mut self,
        path: &Path,
        offset: f64,
        source_rate: u32,
        generation: u64,
        paused: bool,
    ) -> Result<()> {
        self.stop();
        self.generation = generation;
        let device = if let Some(selected) = &self.device {
            self.host
                .output_devices()?
                .find(|device| device == selected)
                .context("Selected audio output device is no longer available")?
        } else {
            self.host
                .default_output_device()
                .context("No audio output device. Check your audio service and device")?
        };
        let config = output::choose_config(&device, source_rate)?;
        let mut decoder = Decoder::spawn(path, offset, config.sample_rate(), config.channels())?;
        let shared = Arc::new(SharedOutput::new(self.volume, paused));
        let stream = output::build(
            &device,
            &config,
            decoder.take_consumer().unwrap(),
            decoder.state.clone(),
            shared.clone(),
        )?;
        if !paused {
            stream.play()?;
        }
        self.path = Some(path.to_owned());
        self.source_rate = source_rate;
        self.cached_samples.replace(PcmSnapshot {
            samples: [0.0; 4096],
            sample_rate: config.sample_rate(),
            revision: 0,
            generation,
        });
        self.session = Some(Session {
            stream,
            decoder,
            shared,
            rate: config.sample_rate(),
            offset,
            duration: 0.0,
            generation,
            notified: false,
        });
        Ok(())
    }
    pub fn set_duration(&mut self, duration: f64) {
        if let Some(session) = &mut self.session {
            session.duration = duration.max(0.0);
        }
    }
    pub fn pause(&self, paused: bool) {
        if let Some(session) = &self.session {
            session.shared.paused.store(paused, Ordering::Release);
            let result = if paused {
                session.stream.pause()
            } else {
                session.stream.play()
            };
            if result.is_err() {
                session.shared.failed.store(true, Ordering::Release);
            }
        }
    }
    pub fn toggle_pause(&self) {
        if let Some(session) = &self.session {
            self.pause(!session.shared.paused.load(Ordering::Acquire));
        }
    }
    pub fn seek(&mut self, seconds: f64) -> Result<()> {
        if !seconds.is_finite() {
            bail!("Invalid seek position");
        }
        if let Some(path) = self.path.clone() {
            let snapshot = self.snapshot();
            self.play(
                &path,
                seconds.max(0.0),
                self.source_rate,
                snapshot.generation,
                snapshot.paused,
            )?;
            self.set_duration(snapshot.duration);
        }
        Ok(())
    }
    pub fn set_volume(&mut self, value: f32) {
        if !value.is_finite() {
            return;
        }
        self.volume = value.clamp(0.0, 1.0);
        if let Some(session) = &self.session {
            session
                .shared
                .volume
                .store(self.volume.to_bits(), Ordering::Release);
        }
    }
    pub fn stop(&mut self) {
        if let Some(mut session) = self.session.take() {
            session.shared.paused.store(true, Ordering::Release);
            drop(session.stream);
            session.decoder.close();
        }
        self.path = None;
        self.cached_samples.replace(PcmSnapshot {
            samples: [0.0; 4096],
            sample_rate: 48000,
            revision: 0,
            generation: self.generation,
        });
    }
    pub fn snapshot(&self) -> PlaybackSnapshot {
        if let Some(s) = &self.session {
            let consumed = s.shared.frames.load(Ordering::Acquire);
            let latency = s.shared.latency_frames.load(Ordering::Acquire);
            PlaybackSnapshot {
                frames_written: consumed,
                latency_seconds: latency as f64 / s.rate as f64,
                position: s.offset
                    + if s.shared.ended.load(Ordering::Acquire) {
                        consumed
                    } else {
                        consumed.saturating_sub(latency)
                    } as f64
                        / s.rate as f64,
                duration: s.duration,
                paused: s.shared.paused.load(Ordering::Acquire),
                sample_rate: s.rate,
                volume: self.volume,
                generation: s.generation,
                has_stream: true,
                error: if s.shared.failed.load(Ordering::Acquire) {
                    Some("Audio output failed. Check your output device".into())
                } else {
                    s.decoder.error()
                },
            }
        } else {
            PlaybackSnapshot {
                frames_written: 0,
                latency_seconds: 0.0,
                position: 0.0,
                duration: 0.0,
                paused: false,
                sample_rate: 48000,
                volume: self.volume,
                generation: self.generation,
                has_stream: false,
                error: None,
            }
        }
    }
    pub fn samples(&self) -> PcmSnapshot {
        if let Some(s) = &self.session {
            for _ in 0..8 {
                let sequence = s.shared.sequence.load(Ordering::Acquire);
                if sequence % 2 != 0 {
                    continue;
                }
                let head = s.shared.analysis_head.load(Ordering::Relaxed) as usize;
                let samples = std::array::from_fn(|i| {
                    f32::from_bits(s.shared.analysis[(head + i) % 4096].load(Ordering::Relaxed))
                });
                let revision = s.shared.revision.load(Ordering::Relaxed);
                if s.shared.sequence.load(Ordering::Acquire) == sequence {
                    self.cached_samples.replace(PcmSnapshot {
                        samples,
                        sample_rate: s.rate,
                        revision,
                        generation: s.generation,
                    });
                    break;
                }
            }
        }
        self.cached_samples.borrow().clone()
    }
    pub fn poll_events(&mut self) -> Vec<AudioEvent> {
        let Some(s) = &mut self.session else {
            return Vec::new();
        };
        if s.notified {
            return Vec::new();
        }
        if s.shared.failed.load(Ordering::Acquire) {
            s.notified = true;
            return vec![AudioEvent::Failed(
                s.generation,
                "Audio output failed. Check your output device".into(),
            )];
        }
        if s.shared.ended.load(Ordering::Acquire) {
            s.notified = true;
            return vec![if let Some(error) = s.decoder.error() {
                AudioEvent::Failed(s.generation, error)
            } else {
                AudioEvent::Finished(s.generation)
            }];
        }
        Vec::new()
    }
}
impl Drop for AudioPlayer {
    fn drop(&mut self) {
        self.stop();
    }
}
