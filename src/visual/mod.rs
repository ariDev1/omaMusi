//! Shared audio-reactive state for all visuals.
pub mod analysis;
pub mod director;
pub mod horizon;
pub mod waveform;
use crate::audio::PcmSnapshot;
use analysis::MusicAnalyzer;
use director::VisualDirector;
use horizon::{HorizonHotspots, wave_payload};
use rustfft::{Fft, FftPlanner, num_complex::Complex};
use std::sync::Arc;
use waveform::WaveformPersistence;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum VisualMode {
    Warp,
    Spectrum,
    Waveform,
    Spectrogram,
    PhiCathedral,
    EventHorizon,
    ParticleDance,
    CoverArt,
    CoverGallery,
    CosmicWeb,
    TimeTunnel,
    GpuLab,
}
impl VisualMode {
    pub const ALL: [Self; 12] = [
        Self::Warp,
        Self::Spectrum,
        Self::Waveform,
        Self::Spectrogram,
        Self::PhiCathedral,
        Self::EventHorizon,
        Self::ParticleDance,
        Self::CoverArt,
        Self::CoverGallery,
        Self::CosmicWeb,
        Self::TimeTunnel,
        Self::GpuLab,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Warp => "Warp",
            Self::Spectrum => "Spectrum",
            Self::Waveform => "Waveform",
            Self::Spectrogram => "Spectrogram",
            Self::PhiCathedral => "Phi Cathedral",
            Self::EventHorizon => "Event Horizon",
            Self::ParticleDance => "Particle Dance",
            Self::CoverArt => "Cover Art",
            Self::CoverGallery => "Cover Gallery",
            Self::CosmicWeb => "Cosmic Web",
            Self::TimeTunnel => "Time Tunnel",
            Self::GpuLab => "GPU Lab",
        }
    }
    pub fn parse(value: &str) -> Option<Self> {
        let lower = value.to_lowercase();
        let alias = match lower.as_str() {
            "sprites" => "warp",
            "phi" | "cathedral" | "phi-cathedral" => "phi cathedral",
            other => other,
        };
        Self::ALL
            .iter()
            .copied()
            .find(|mode| mode.name().to_lowercase() == alias)
    }
}

#[derive(Clone)]
pub struct VisualFrame {
    pub green: [f32; 3],
    pub magenta: [f32; 3],
    pub mode: VisualMode,
    pub active: bool,
    pub sample_rate: u32,
    pub energy: f64,
    pub bass: f64,
    pub treble: f64,
    pub time: f64,
    pub phi_pulse: f64,
    pub phi_bloom: f64,
    pub phi_tension: f64,
    pub phi_event: f64,
    pub phi_velocity: f64,
    pub phi_impulse: f64,
    pub aether_onset: f64,
    pub aether_beat_phase: f64,
    pub aether_beat_confidence: f64,
    pub aether_density: f64,
    pub aether_scene_morph: f64,
    pub aether_world_turn: f64,
    pub aether_beat_pulse: f64,
    pub aether_phrase_progress: f64,
    pub bands: [f64; 96],
    pub peaks: [f64; 96],
    pub history: Vec<u8>,
    pub history_revision: u64,
    pub waveform: [f32; 1024],
    pub wave_alpha: f64,
    pub waves: [[f32; 2]; 5],
    pub spots: [[f32; 3]; 4],
    pub bursts: Vec<[f64; 2]>,
    pub particle_vertices: Vec<[f32; 3]>,
    pub tints: Vec<u8>,
    /// Background, accent, cyan and bright foreground RGB values.
    pub colors: [[f32; 3]; 4],
}
impl VisualFrame {
    fn new() -> Self {
        let vertices = include_bytes!("../../assets/star-seeds.bin")
            .as_chunks::<12>()
            .0
            .iter()
            .map(|v| {
                std::array::from_fn(|i| f32::from_le_bytes(v[i * 4..i * 4 + 4].try_into().unwrap()))
            })
            .collect();
        Self {
            green: [158.0 / 255.0, 206.0 / 255.0, 106.0 / 255.0],
            magenta: [187.0 / 255.0, 154.0 / 255.0, 247.0 / 255.0],
            mode: VisualMode::Warp,
            active: false,
            sample_rate: 48000,
            energy: 0.0,
            bass: 0.0,
            treble: 0.0,
            time: 0.0,
            phi_pulse: 0.0,
            phi_bloom: 0.0,
            phi_tension: 0.0,
            phi_event: 0.0,
            phi_velocity: 0.0,
            phi_impulse: 0.0,
            aether_onset: 0.0,
            aether_beat_phase: 0.0,
            aether_beat_confidence: 0.0,
            aether_density: 0.0,
            aether_scene_morph: 0.0,
            aether_world_turn: 0.0,
            aether_beat_pulse: 0.0,
            aether_phrase_progress: 0.0,
            bands: [0.0; 96],
            peaks: [0.0; 96],
            history: vec![0; 96 * 360 * 3],
            history_revision: 0,
            waveform: [0.0; 1024],
            wave_alpha: 0.0,
            waves: [[-1.0, 0.0]; 5],
            spots: [[-1.0, 0.0, 0.0]; 4],
            bursts: Vec::new(),
            particle_vertices: vertices,
            tints: include_bytes!("../../assets/star-tints.bin").to_vec(),
            colors: [
                [26.0 / 255.0, 27.0 / 255.0, 38.0 / 255.0],
                [122.0 / 255.0, 162.0 / 255.0, 247.0 / 255.0],
                [125.0 / 255.0, 207.0 / 255.0, 1.0],
                [1.0, 1.0, 1.0],
            ],
        }
    }
}
pub struct VisualState {
    pub mode: VisualMode,
    pub waveform: WaveformPersistence,
    pub particle_vertices: Vec<[f32; 3]>,
    frame: VisualFrame,
    buffer: [f32; 4096],
    fft: Arc<dyn Fft<f64>>,
    fft_buffer: Vec<Complex<f64>>,
    fft_scratch: Vec<Complex<f64>>,
    window: [f64; 4096],
    bins: Vec<Vec<usize>>,
    centers: [f64; 96],
    analysis_rate: u32,
    audio_generation: u64,
    audio_revision: u64,
    seen_revision: u64,
    exposure: f64,
    previous_bass: f64,
    beat_cooldown: f64,
    phi_cooldown: f64,
    history_elapsed: f64,
    analyzer: MusicAnalyzer,
    director: VisualDirector,
    hotspots: HorizonHotspots,
}
impl Default for VisualState {
    fn default() -> Self {
        Self::new()
    }
}
impl VisualState {
    pub fn new() -> Self {
        let fft = FftPlanner::new().plan_fft_forward(4096);
        let scratch_len = fft.get_inplace_scratch_len();
        let frame = VisualFrame::new();
        Self {
            mode: VisualMode::Warp,
            waveform: WaveformPersistence::new(),
            particle_vertices: frame.particle_vertices.clone(),
            frame,
            buffer: [0.0; 4096],
            fft,
            fft_buffer: vec![Complex::new(0.0, 0.0); 4096],
            fft_scratch: vec![Complex::new(0.0, 0.0); scratch_len],
            window: std::array::from_fn(|i| {
                0.5 - 0.5 * (std::f64::consts::TAU * i as f64 / 4095.0).cos()
            }),
            bins: Vec::new(),
            centers: [0.0; 96],
            analysis_rate: 0,
            audio_generation: 0,
            audio_revision: 0,
            seen_revision: 0,
            exposure: 0.0,
            previous_bass: 0.0,
            beat_cooldown: 0.0,
            phi_cooldown: 0.0,
            history_elapsed: 0.0,
            analyzer: MusicAnalyzer::new(),
            director: VisualDirector::default(),
            hotspots: HorizonHotspots::default(),
        }
    }
    pub fn feed(&mut self, pcm: &PcmSnapshot) {
        if pcm.generation != self.audio_generation {
            self.reset();
            self.audio_generation = pcm.generation;
            self.audio_revision = 0;
            self.seen_revision = 0;
        }
        self.frame.sample_rate = pcm.sample_rate.max(60);
        if pcm.revision != self.audio_revision {
            self.buffer = pcm.samples.map(|v| {
                if v.is_finite() {
                    v.clamp(-1.0, 1.0)
                } else {
                    0.0
                }
            });
            self.audio_revision = pcm.revision;
        }
    }
    pub fn reset(&mut self) {
        let phase = self.frame.time;
        let history_revision = self.frame.history_revision + 1;
        let colors = self.frame.colors;
        let green = self.frame.green;
        let magenta = self.frame.magenta;
        self.frame = VisualFrame::new();
        self.frame.time = phase;
        self.frame.history_revision = history_revision;
        self.frame.colors = colors;
        self.frame.green = green;
        self.frame.magenta = magenta;
        self.buffer.fill(0.0);
        self.waveform.reset();
        self.seen_revision = self.audio_revision;
        self.exposure = 0.0;
        self.previous_bass = 0.0;
        self.beat_cooldown = 0.0;
        self.phi_cooldown = 0.0;
        self.history_elapsed = 0.0;
        self.analyzer.reset();
        self.director.reset();
        self.hotspots.reset();
    }
    pub fn cycle(&mut self, step: i32) -> VisualMode {
        self.mode = VisualMode::ALL
            [(self.mode as i32 + step).rem_euclid(VisualMode::ALL.len() as i32) as usize];
        self.frame.mode = self.mode;
        self.mode
    }
    pub fn set_colors(&mut self, colors: [[f32; 3]; 4]) {
        self.frame.colors = colors;
    }
    pub fn set_palette(&mut self, colors: [[f32; 3]; 6]) {
        self.frame.colors = colors[..4].try_into().unwrap();
        self.frame.green = colors[4];
        self.frame.magenta = colors[5];
    }
    pub fn frame(&self) -> &VisualFrame {
        &self.frame
    }
    fn trace(&self) -> [f32; 1024] {
        let data = &self.buffer[2048..];
        let start = data
            .windows(2)
            .position(|p| p[0] <= 0.0 && p[1] > 0.0)
            .unwrap_or(0);
        let data = &data[start..(start + 1024).min(2048)];
        let rms = (data.iter().map(|v| v * v).sum::<f32>() / data.len() as f32).sqrt() as f64;
        let gain = (0.45 / (rms + 0.04)).min(4.0) as f32;
        let gained: Vec<_> = data.iter().map(|v| *v * gain).collect();
        std::array::from_fn(|i| {
            waveform::interpolate(&gained, i as f64 * (data.len() - 1) as f64 / 1023.0) as f32
        })
    }
    pub fn tick(&mut self, elapsed: f64, active: bool) {
        let elapsed = if elapsed.is_finite() {
            elapsed.max(0.001)
        } else {
            0.001
        };
        let dt = elapsed.min(0.05);
        let fresh = active
            && self.mode == VisualMode::Waveform
            && self.seen_revision != self.audio_revision;
        self.exposure = (self.exposure + elapsed).min(0.1);
        let trace = if fresh { Some(self.trace()) } else { None };
        self.waveform.step(
            elapsed,
            trace.as_ref().map(|v| v.as_slice()),
            Some(self.exposure),
        );
        if fresh || !active || self.mode != VisualMode::Waveform {
            self.exposure = 0.0;
        }
        self.seen_revision = self.audio_revision;
        if self.analysis_rate != self.frame.sample_rate {
            self.analysis_rate = self.frame.sample_rate;
            let upper = (self.analysis_rate as f64 / 2.0).min(18000.0);
            let edges: [f64; 97] = std::array::from_fn(|i| {
                (30.0f64.ln() + (upper.ln() - 30.0f64.ln()) * i as f64 / 96.0).exp()
            });
            self.bins = (0..96)
                .map(|i| {
                    (0..2049)
                        .filter(|b| {
                            let hz = *b as f64 * self.analysis_rate as f64 / 4096.0;
                            hz >= edges[i] && hz < edges[i + 1]
                        })
                        .collect()
                })
                .collect();
            self.centers = std::array::from_fn(|i| (edges[i] * edges[i + 1]).sqrt());
        }
        let mut levels = [0.0f64; 96];
        let (target_energy, target_bass, target_treble) = if active {
            let rms = (self.buffer.iter().map(|v| v * v).sum::<f32>() / 4096.0).sqrt() as f64;
            for (i, sample) in self.fft_buffer.iter_mut().enumerate() {
                *sample = Complex::new(self.buffer[i] as f64 * self.window[i], 0.0);
            }
            self.fft
                .process_with_scratch(&mut self.fft_buffer, &mut self.fft_scratch);
            let spectrum: [f64; 2049] = std::array::from_fn(|i| self.fft_buffer[i].norm() / 1024.0);
            let frequency = |i: usize| i as f64 * self.analysis_rate as f64 / 4096.0;
            let bass_bins: Vec<_> = (0..2049)
                .filter(|i| frequency(*i) >= 30.0 && frequency(*i) < 220.0)
                .collect();
            let treble_bins: Vec<_> = (0..2049)
                .filter(|i| frequency(*i) > 2500.0 && frequency(*i) < 14000.0)
                .collect();
            let bass = if bass_bins.is_empty() {
                0.0
            } else {
                (bass_bins.iter().map(|i| spectrum[*i].powi(2)).sum::<f64>()
                    / bass_bins.len() as f64)
                    .sqrt()
            };
            let treble = if treble_bins.is_empty() {
                0.0
            } else {
                treble_bins.iter().map(|i| spectrum[*i]).sum::<f64>() / treble_bins.len() as f64
            };
            for (i, level) in levels.iter_mut().enumerate() {
                let magnitude = if self.bins[i].is_empty() {
                    let position =
                        (self.centers[i] / (self.analysis_rate as f64 / 4096.0)).clamp(0.0, 2048.0);
                    let lo = position.floor() as usize;
                    let hi = (lo + 1).min(2048);
                    spectrum[lo] + (spectrum[hi] - spectrum[lo]) * (position - lo as f64)
                } else {
                    self.bins[i]
                        .iter()
                        .map(|b| spectrum[*b])
                        .fold(0.0, f64::max)
                };
                *level = ((20.0 * (magnitude + 1e-7).log10() + 66.0) / 66.0).clamp(0.0, 1.0);
            }
            let targets = (
                (rms * 5.0).min(1.0),
                (bass * 8.0).min(1.0),
                (treble * 70.0).min(1.0),
            );
            self.beat_cooldown -= dt;
            if targets.1 > self.previous_bass + 0.12 && self.beat_cooldown <= 0.0 {
                self.frame.bursts.push([0.0, targets.1]);
                if self.frame.bursts.len() > 5 {
                    self.frame.bursts.remove(0);
                }
                self.beat_cooldown = 0.24;
            }
            self.previous_bass += (targets.1 - self.previous_bass) * (dt * 5.0).min(1.0);
            targets
        } else {
            (0.0, 0.0, 0.0)
        };
        let ambient_trace =
            matches!(self.mode, VisualMode::Warp | VisualMode::PhiCathedral).then(|| self.trace());
        let f = &mut self.frame;
        f.active = active;
        f.mode = self.mode;
        let smooth = 1.0 - (-dt * if target_energy > f.energy { 22.0 } else { 8.0 }).exp();
        f.energy += (target_energy - f.energy) * smooth;
        f.bass += (target_bass - f.bass) * smooth;
        f.treble += (target_treble - f.treble) * smooth;
        let pulse = (target_bass * 1.25 + target_energy * 0.35).min(1.0);
        let bloom = (target_energy * 0.75 + target_bass * 0.45).min(1.0);
        let tension = (target_treble * 0.55
            + target_energy * 0.35
            + levels[24..72].iter().sum::<f64>() / 48.0 * 0.65)
            .min(1.0);
        f.phi_pulse += (pulse - f.phi_pulse)
            * (1.0 - (-dt * if pulse > f.phi_pulse { 28.0 } else { 7.0 }).exp());
        f.phi_bloom += (bloom - f.phi_bloom) * (1.0 - (-dt * 2.4).exp());
        f.phi_tension += (tension - f.phi_tension) * (1.0 - (-dt * 4.0).exp());
        self.phi_cooldown = (self.phi_cooldown - dt).max(0.0);
        let burst = f.bursts.last().is_some_and(|b| b[0] < 0.06);
        if active && burst && self.phi_cooldown <= 0.0 && target_bass > 0.42 && target_energy > 0.12
        {
            f.phi_event = 1.0;
            self.phi_cooldown = 3.2 + (1.0 - target_energy) * 2.8;
        }
        f.phi_event *= (-dt * 1.45).exp();
        f.phi_impulse =
            (f.phi_impulse + (target_bass - self.previous_bass).max(0.0) * 2.8) * (-dt * 7.5).exp();
        f.phi_impulse = f.phi_impulse.clamp(0.0, 1.0);
        let drive = 0.18 + target_energy * 1.25 + target_bass * 0.65 + f.phi_impulse;
        f.phi_velocity += (drive - f.phi_velocity) * (1.0 - (-dt * 4.2).exp());
        f.phi_velocity = f.phi_velocity.clamp(0.08, 2.6);
        f.time += dt
            * (0.16
                + if active { 0.50 } else { 0.0 }
                + f.energy * 2.6
                + f.bass * 0.90
                + f.treble * 0.40);
        for b in &mut f.bursts {
            b[0] += dt;
        }
        f.bursts.retain(|b| b[0] < 1.8);
        for (i, level) in levels.iter().enumerate() {
            f.bands[i] = level.max(f.bands[i] * (-dt * 11.0).exp());
            f.peaks[i] = f.bands[i].max(f.peaks[i] - dt * 0.4);
        }
        let metrics = self
            .analyzer
            .update(dt, &f.bands, f.energy, f.bass, f.treble);
        let directed = self.director.update(metrics, dt);
        self.hotspots.update(dt, metrics, active);
        f.aether_onset = metrics.onset;
        f.aether_beat_phase = metrics.beat_phase;
        f.aether_beat_confidence = metrics.beat_confidence;
        f.aether_density = metrics.density;
        f.aether_scene_morph = directed.scene_morph;
        f.aether_world_turn = directed.world_turn;
        f.aether_beat_pulse = directed.beat_pulse;
        f.aether_phrase_progress = directed.phrase_progress;
        f.waves = wave_payload(&f.bursts);
        f.spots = self.hotspots.payload();
        if active {
            self.history_elapsed += dt;
            if self.history_elapsed >= 1.0 / 30.0 {
                self.history_elapsed %= 1.0 / 30.0;
                for row in 0..96 {
                    let offset = row * 360 * 3;
                    f.history.copy_within(offset + 3..offset + 360 * 3, offset);
                    let value = f.bands[95 - row];
                    for channel in 0..3 {
                        let low = f.colors[2][channel] as f64 * 255.0;
                        let high = f.colors[1][channel] as f64 * 255.0;
                        f.history[offset + 359 * 3 + channel] =
                            ((low * (1.0 - value) + high * value) * value) as u8;
                    }
                }
                f.history_revision += 1;
            }
        }
        if let Some(trace) = trace.or(ambient_trace) {
            f.waveform = trace;
        }
        f.wave_alpha = self.waveform.latest_alpha;
    }
}
