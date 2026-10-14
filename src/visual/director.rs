//! Musical timing drives the visual choreography without affecting audio.
use super::analysis::MusicMetrics;
#[derive(Clone, Copy, Debug, Default)]
pub struct DirectorState {
    pub scene_index: usize,
    pub scene_morph: f64,
    pub world_turn: f64,
    pub beat_pulse: f64,
    pub phrase_progress: f64,
}
#[derive(Default)]
pub struct VisualDirector {
    scene_index: usize,
    scene_age: f64,
    world_turn: f64,
    beat_count: usize,
    last_phase: f64,
    beat_pulse: f64,
}
impl VisualDirector {
    pub fn reset(&mut self) {
        *self = Self::default();
    }
    pub fn update(&mut self, metrics: MusicMetrics, dt: f64) -> DirectorState {
        const PHRASES: [usize; 5] = [13, 21, 34, 21, 13];
        self.scene_age += dt;
        if metrics.beat_confidence > 0.15 && self.last_phase > 0.72 && metrics.beat_phase < 0.28 {
            self.beat_count += 1;
            self.beat_pulse = 1.0;
            if self.beat_count >= PHRASES[self.scene_index] {
                self.scene_index = (self.scene_index + 1) % 5;
                self.beat_count = 0;
                self.scene_age = 0.0;
            }
        }
        self.last_phase = metrics.beat_phase;
        self.beat_pulse *= (-dt * 7.0).exp();
        let progress = (self.beat_count as f64 / PHRASES[self.scene_index] as f64).min(1.0);
        self.world_turn += dt
            * (0.10
                + metrics.density * 0.22
                + metrics.onset * 0.32
                + metrics.high_transient * 0.18);
        let slow = 0.5 + 0.5 * (self.scene_age * (0.15 + self.scene_index as f64 * 0.017)).sin();
        DirectorState {
            scene_index: self.scene_index,
            scene_morph: (0.58 * slow + 0.42 * self.beat_pulse).min(1.0),
            world_turn: self.world_turn,
            beat_pulse: self.beat_pulse,
            phrase_progress: progress,
        }
    }
}
