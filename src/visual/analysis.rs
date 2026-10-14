//! Baseline onset, beat and density analysis.
use std::collections::VecDeque;
#[derive(Clone, Copy, Debug, Default)]
pub struct MusicMetrics {
    pub onset: f64,
    pub beat_phase: f64,
    pub beat_confidence: f64,
    pub bpm: f64,
    pub density: f64,
    pub low_transient: f64,
    pub mid_transient: f64,
    pub high_transient: f64,
}
pub struct MusicAnalyzer {
    previous: [f32; 96],
    elapsed: f64,
    last_onset: Option<f64>,
    intervals: VecDeque<f32>,
    bpm: f64,
    beat_anchor: f64,
    onset_env: f64,
    density_env: f64,
}
impl Default for MusicAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
impl MusicAnalyzer {
    pub fn new() -> Self {
        Self {
            previous: [0.0; 96],
            elapsed: 0.0,
            last_onset: None,
            intervals: VecDeque::new(),
            bpm: 0.0,
            beat_anchor: 0.0,
            onset_env: 0.0,
            density_env: 0.0,
        }
    }
    pub fn reset(&mut self) {
        *self = Self::new();
    }
    pub fn update(
        &mut self,
        dt: f64,
        bands: &[f64; 96],
        energy: f64,
        bass: f64,
        treble: f64,
    ) -> MusicMetrics {
        self.elapsed += dt;
        let current = bands.map(|v| v as f32);
        let flux = |start: usize, stop: usize| {
            current[start..stop]
                .iter()
                .zip(&self.previous[start..stop])
                .map(|(a, b)| (a - b).max(0.0))
                .sum::<f32>() as f64
                / (stop - start) as f64
        };
        let low = flux(0, 18);
        let mid = flux(18, 62);
        let high = flux(62, 96);
        let target = ((low * 1.45 + mid * 0.95 + high * 0.70) * 4.8
            + (bass - 0.18).max(0.0) * 0.35)
            .min(1.0);
        if target > self.onset_env {
            self.onset_env += (target - self.onset_env) * (1.0 - (-dt * 38.0).exp());
        } else {
            self.onset_env *= (-dt * 9.0).exp();
        }
        if target > 0.24 && self.onset_env > 0.18 {
            if let Some(last) = self.last_onset {
                let interval = self.elapsed - last;
                if (0.27..=1.20).contains(&interval) {
                    if self.intervals.len() == 12 {
                        self.intervals.pop_front();
                    }
                    self.intervals.push_back(interval as f32);
                    if self.intervals.len() >= 3 {
                        let mut values: Vec<_> = self.intervals.iter().copied().collect();
                        values.sort_by(f32::total_cmp);
                        let n = values.len();
                        let median = if n % 2 == 0 {
                            (values[n / 2 - 1] + values[n / 2]) / 2.0
                        } else {
                            values[n / 2]
                        };
                        let mut candidate = 60.0 / median as f64;
                        while candidate < 70.0 {
                            candidate *= 2.0;
                        }
                        while candidate > 180.0 {
                            candidate *= 0.5;
                        }
                        self.bpm = if self.bpm <= 0.0 {
                            candidate
                        } else {
                            self.bpm + (candidate - self.bpm) * 0.18
                        };
                    }
                }
            }
            self.last_onset = Some(self.elapsed);
            self.beat_anchor = self.elapsed;
        }
        let (beat_phase, confidence) = if self.bpm > 0.0 {
            (
                ((self.elapsed - self.beat_anchor) / (60.0 / self.bpm)).rem_euclid(1.0),
                (self.intervals.len() as f64 / 8.0).min(1.0),
            )
        } else {
            (0.0, 0.0)
        };
        let density_target = (current.iter().filter(|v| **v > 0.18).count() as f64 / 96.0 * 1.7
            + energy * 0.35
            + treble * 0.16)
            .min(1.0);
        self.density_env += (density_target - self.density_env) * (1.0 - (-dt * 3.2).exp());
        self.previous = current;
        MusicMetrics {
            onset: self.onset_env,
            beat_phase,
            beat_confidence: confidence,
            bpm: self.bpm,
            density: self.density_env,
            low_transient: (low * 8.0).min(1.0),
            mid_transient: (mid * 8.0).min(1.0),
            high_transient: (high * 8.0).min(1.0),
        }
    }
}
