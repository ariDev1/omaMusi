"""Music-structure analysis used only by the visual system."""

from dataclasses import dataclass
from collections import deque
import math
import numpy as np


@dataclass(frozen=True)
class MusicMetrics:
    onset: float = 0.0
    beat_phase: float = 0.0
    beat_confidence: float = 0.0
    bpm: float = 0.0
    density: float = 0.0
    low_transient: float = 0.0
    mid_transient: float = 0.0
    high_transient: float = 0.0


class MusicAnalyzer:
    """Deterministic downstream visual analyzer."""

    def __init__(self):
        self.previous_bands = np.zeros(96, dtype=np.float32)
        self.elapsed = 0.0
        self.last_onset_time = None
        self.intervals = deque(maxlen=12)
        self.bpm = 0.0
        self.beat_anchor = 0.0
        self.onset_env = 0.0
        self.density_env = 0.0

    def reset(self):
        self.previous_bands.fill(0)
        self.elapsed = 0.0
        self.last_onset_time = None
        self.intervals.clear()
        self.bpm = 0.0
        self.beat_anchor = 0.0
        self.onset_env = 0.0
        self.density_env = 0.0

    @staticmethod
    def _band_flux(current, previous, start, stop):
        delta = np.maximum(current[start:stop] - previous[start:stop], 0.0)
        return float(delta.mean()) if len(delta) else 0.0

    def update(self, dt, bands, energy, bass, treble):
        self.elapsed += dt
        current = np.asarray(bands, dtype=np.float32)

        low = self._band_flux(current, self.previous_bands, 0, 18)
        mid = self._band_flux(current, self.previous_bands, 18, 62)
        high = self._band_flux(current, self.previous_bands, 62, 96)

        weighted_flux = low * 1.45 + mid * 0.95 + high * 0.70
        onset_target = min(1.0, weighted_flux * 4.8 + max(0.0, bass - 0.18) * 0.35)

        if onset_target > self.onset_env:
            self.onset_env += (onset_target - self.onset_env) * (1 - math.exp(-dt * 38.0))
        else:
            self.onset_env *= math.exp(-dt * 9.0)

        onset_event = onset_target > 0.24 and self.onset_env > 0.18
        if onset_event:
            if self.last_onset_time is not None:
                interval = self.elapsed - self.last_onset_time
                if 0.27 <= interval <= 1.20:
                    self.intervals.append(interval)
                    if len(self.intervals) >= 3:
                        median = float(np.median(np.array(self.intervals, dtype=np.float32)))
                        candidate = 60.0 / median
                        while candidate < 70.0:
                            candidate *= 2.0
                        while candidate > 180.0:
                            candidate *= 0.5
                        self.bpm = candidate if self.bpm <= 0 else self.bpm + (candidate - self.bpm) * 0.18
            self.last_onset_time = self.elapsed
            self.beat_anchor = self.elapsed

        if self.bpm > 0:
            period = 60.0 / self.bpm
            beat_phase = ((self.elapsed - self.beat_anchor) / period) % 1.0
            confidence = min(1.0, len(self.intervals) / 8.0)
        else:
            beat_phase = 0.0
            confidence = 0.0

        density_target = min(
            1.0,
            float((current > 0.18).mean()) * 1.7 + energy * 0.35 + treble * 0.16,
        )
        self.density_env += (density_target - self.density_env) * (1 - math.exp(-dt * 3.2))
        self.previous_bands[:] = current

        return MusicMetrics(
            onset=self.onset_env,
            beat_phase=beat_phase,
            beat_confidence=confidence,
            bpm=self.bpm,
            density=self.density_env,
            low_transient=min(1.0, low * 8.0),
            mid_transient=min(1.0, mid * 8.0),
            high_transient=min(1.0, high * 8.0),
        )
