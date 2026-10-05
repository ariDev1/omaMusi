"""Time-decaying oscilloscope density shared by CPU and GPU renderers."""
import math
import numpy as np


class WaveformPersistence:
    WIDTH, HEIGHT = 768, 256

    def __init__(self):
        self.density = np.zeros((self.HEIGHT, self.WIDTH), dtype=np.float32)
        self.latest = np.zeros(1024, dtype=np.float32)
        self.latest_alpha = 0.0
        self.revision = 0
        self._rgba = None
        self._rows = np.arange(self.HEIGHT, dtype=np.float32)[:, None]
        stops = np.array([0, 0.18, 0.36, 0.56, 0.78, 1.0])
        colors = np.array([[84, 32, 175], [35, 95, 255], [0, 225, 245],
                           [60, 245, 120], [255, 225, 50], [255, 65, 30]])
        self._palette = np.column_stack([
            np.interp(np.linspace(0, 1, 256), stops, colors[:, channel])
            for channel in range(3)
        ]).astype(np.uint8)

    def reset(self):
        self.density.fill(0)
        self.latest.fill(0)
        self.latest_alpha = 0.0
        self.revision += 1
        self._rgba = None

    def step(self, dt, trace=None, exposure=None):
        dt = max(0.0, float(dt))
        decay = math.exp(-dt / 0.5)
        self.density *= decay
        self.latest_alpha *= math.exp(-dt / 0.3)
        if trace is not None and len(trace):
            self.latest = np.nan_to_num(np.asarray(trace, dtype=np.float32)).copy()
            self.latest_alpha = 0.85
            data = np.interp(np.linspace(0, len(trace) - 1, self.WIDTH),
                             np.arange(len(trace)), self.latest)
            y = np.clip(0.47 - data * 0.28, -1, 2) * self.HEIGHT
            previous, following = np.r_[y[0], y[:-1]], np.r_[y[1:], y[-1]]
            low = np.minimum(y, np.minimum(previous, following))
            high = np.maximum(y, np.maximum(previous, following))
            distance = np.maximum(np.maximum(low - self._rows, self._rows - high), 0)
            coverage = np.exp(-0.5 * (distance / 0.55) ** 2).astype(np.float32)
            # Exact exponential integration keeps continuous traces equally hot
            # at 30 or 60 fps; repainting never deposits additional traces.
            deposit_decay = decay if exposure is None else math.exp(-max(0.0, exposure) / 0.5)
            self.density += coverage * (2.5 * (1 - deposit_decay))
            np.minimum(self.density, 4.0, out=self.density)
        self.revision += 1
        self._rgba = None

    def rgba(self):
        if self._rgba is None:
            pixels = np.empty((self.HEIGHT, self.WIDTH, 4), dtype=np.uint8)
            intensity = np.clip(self.density * 0.45 * 255, 0, 255).astype(np.uint8)
            pixels[:, :, :3] = self._palette[intensity]
            pixels[:, :, 3] = (np.clip(self.density * 1.4, 0, 0.92) * 255).astype(np.uint8)
            pixels[pixels[:, :, 3] == 0] = 0
            self._rgba = np.ascontiguousarray(pixels)
        return self._rgba
