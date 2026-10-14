//! Time-decaying density shared by accelerated and software rendering.
pub const WIDTH: usize = 768;
pub const HEIGHT: usize = 256;
pub struct WaveformPersistence {
    pub density: Vec<f32>,
    pub latest: Vec<f32>,
    pub latest_alpha: f64,
    pub revision: u64,
    palette: [[u8; 3]; 256],
}
pub fn interpolate(values: &[f32], position: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let pos = position.clamp(0.0, (values.len() - 1) as f64);
    let lo = pos.floor() as usize;
    let hi = (lo + 1).min(values.len() - 1);
    values[lo] as f64 + (values[hi] as f64 - values[lo] as f64) * (pos - lo as f64)
}
impl Default for WaveformPersistence {
    fn default() -> Self {
        Self::new()
    }
}
impl WaveformPersistence {
    pub fn new() -> Self {
        let stops = [0.0, 0.18, 0.36, 0.56, 0.78, 1.0];
        let colors: [[f64; 3]; 6] = [
            [84.0, 32.0, 175.0],
            [35.0, 95.0, 255.0],
            [0.0, 225.0, 245.0],
            [60.0, 245.0, 120.0],
            [255.0, 225.0, 50.0],
            [255.0, 65.0, 30.0],
        ];
        let palette = std::array::from_fn(|i| {
            let value = i as f64 / 255.0;
            let upper = stops.iter().position(|v| *v >= value).unwrap_or(5).max(1);
            let lower = upper - 1;
            let mix = (value - stops[lower]) / (stops[upper] - stops[lower]);
            std::array::from_fn(|c| {
                (colors[lower][c] + (colors[upper][c] - colors[lower][c]) * mix) as u8
            })
        });
        Self {
            density: vec![0.0; WIDTH * HEIGHT],
            latest: vec![0.0; 1024],
            latest_alpha: 0.0,
            revision: 0,
            palette,
        }
    }
    pub fn reset(&mut self) {
        self.density.fill(0.0);
        self.latest.fill(0.0);
        self.latest_alpha = 0.0;
        self.revision += 1;
    }
    pub fn step(&mut self, dt: f64, trace: Option<&[f32]>, exposure: Option<f64>) {
        let dt = if dt.is_finite() { dt.max(0.0) } else { 0.0 };
        let decay = (-dt / 0.5).exp();
        for v in &mut self.density {
            *v *= decay as f32;
        }
        self.latest_alpha *= (-dt / 0.3).exp();
        if let Some(trace) = trace.filter(|v| !v.is_empty()) {
            self.latest = trace
                .iter()
                .map(|v| {
                    if v.is_nan() {
                        0.0
                    } else if *v == f32::INFINITY {
                        f32::MAX
                    } else if *v == f32::NEG_INFINITY {
                        f32::MIN
                    } else {
                        *v
                    }
                })
                .collect();
            self.latest_alpha = 0.85;
            let y: [f64; WIDTH] = std::array::from_fn(|i| {
                (0.47
                    - interpolate(
                        &self.latest,
                        i as f64 * (trace.len() - 1) as f64 / (WIDTH - 1) as f64,
                    ) * 0.28)
                    .clamp(-1.0, 2.0)
                    * HEIGHT as f64
            });
            let deposit_decay = exposure.map(|v| (-v.max(0.0) / 0.5).exp()).unwrap_or(decay);
            let amount = (2.5 * (1.0 - deposit_decay)) as f32;
            for column in 0..WIDTH {
                let low = y[column]
                    .min(y[column.saturating_sub(1)])
                    .min(y[(column + 1).min(WIDTH - 1)]);
                let high = y[column]
                    .max(y[column.saturating_sub(1)])
                    .max(y[(column + 1).min(WIDTH - 1)]);
                for row in 0..HEIGHT {
                    let distance = (low - row as f64).max(row as f64 - high).max(0.0);
                    let coverage = (-0.5 * (distance / 0.55).powi(2)).exp() as f32;
                    let cell = &mut self.density[row * WIDTH + column];
                    *cell = (*cell + coverage * amount).min(4.0);
                }
            }
        }
        self.revision += 1;
    }
    pub fn rgba(&self) -> Vec<u8> {
        let mut pixels = vec![0u8; WIDTH * HEIGHT * 4];
        for (density, pixel) in self.density.iter().zip(pixels.as_chunks_mut::<4>().0) {
            let intensity = (*density * 0.45 * 255.0).clamp(0.0, 255.0) as usize;
            let alpha = (*density * 1.4).clamp(0.0, 0.92) * 255.0;
            if alpha >= 1.0 {
                pixel[..3].copy_from_slice(&self.palette[intensity]);
                pixel[3] = alpha as u8;
            }
        }
        pixels
    }
}
