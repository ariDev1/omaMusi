//! Transient identities and upload payloads for Event Horizon.
use super::analysis::MusicMetrics;
#[derive(Default)]
pub struct HorizonHotspots {
    spots: Vec<[f64; 3]>,
    cooldown: f64,
    previous_onset: f64,
    serial: u64,
}
impl HorizonHotspots {
    pub fn reset(&mut self) {
        *self = Self::default();
    }
    pub fn update(&mut self, dt: f64, metrics: MusicMetrics, active: bool) {
        for spot in &mut self.spots {
            spot[0] += dt;
        }
        self.spots.retain(|s| s[0] < 4.2);
        self.cooldown = (self.cooldown - dt).max(0.0);
        let transient = metrics
            .mid_transient
            .max(metrics.high_transient * 0.7)
            .max(metrics.low_transient * 0.6);
        let rise = (metrics.onset - self.previous_onset).max(0.0);
        self.previous_onset = metrics.onset;
        if active && self.cooldown == 0.0 && (transient > 0.10 || rise > 0.10) {
            let strength = (transient * 2.0).max(rise * 2.0).min(1.0);
            let seed = (self.serial as f64 * 0.61803398875 + 0.23) % 1.0;
            self.serial += 1;
            self.spots.push([0.0, strength, seed]);
            if self.spots.len() > 4 {
                self.spots.remove(0);
            }
            self.cooldown = 0.24;
        }
    }
    pub fn payload(&self) -> [[f32; 3]; 4] {
        let mut result = [[-1.0, 0.0, 0.0]; 4];
        for (slot, spot) in result.iter_mut().zip(&self.spots) {
            *slot = spot.map(|v| v as f32);
        }
        result
    }
}
pub fn wave_payload(bursts: &[[f64; 2]]) -> [[f32; 2]; 5] {
    let mut result = [[-1.0, 0.0]; 5];
    let offset = bursts.len().saturating_sub(5);
    for (slot, wave) in result.iter_mut().zip(&bursts[offset..]) {
        *slot = wave.map(|v| v as f32);
    }
    result
}
