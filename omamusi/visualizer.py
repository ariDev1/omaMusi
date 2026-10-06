"""Audio-reactive warp tunnel, spectrum, waveform, and spectrogram."""

import math
import sys

import numpy as np
from PySide6.QtCore import QElapsedTimer, QPointF, QRectF, Qt, QTimer
from PySide6.QtGui import (
    QColor, QImage, QLinearGradient, QPainter, QPainterPath, QPen, QPixmap, QRadialGradient,
)
from PySide6.QtWidgets import QWidget

from .audio import SAMPLE_RATE
from .theme import DEFAULTS
from .visual.analysis import MusicAnalyzer
from .visual.director import VisualDirector
from .visual.horizon import HorizonHotspots
from .visual.waveform import WaveformPersistence
from .visual.cover import paint_gallery


class Visualizer(QWidget):
    PARTICLE_COUNT = 384  # Conservative fallback; GPU draws 4,096 streaks.
    modes = ("Warp", "Spectrum", "Waveform", "Spectrogram", "Phi Cathedral", "Event Horizon",
               "Particle Dance", "Cover Art")

    def __init__(self, parent=None):
        super().__init__(parent)
        self.setAttribute(Qt.WidgetAttribute.WA_TransparentForMouseEvents)
        self.colors = dict(DEFAULTS)
        self.active = False
        self.mode = 0
        self.cover_art = QImage()
        self.sample_rate = SAMPLE_RATE
        self.buffer = np.zeros(4096, dtype=np.float32)
        self.wave_persistence = WaveformPersistence()
        self._wave_audio_revision = 0
        self._wave_seen_revision = 0
        self._wave_exposure = 0.0
        self.fft_window = np.hanning(4096)
        self.energy = 0.0
        self.bass = 0.0
        self.treble = 0.0
        self.phi_pulse = 0.0
        self.phi_bloom = 0.0
        self.phi_tension = 0.0
        self.phi_event = 0.0
        self.phi_event_cooldown = 0.0
        self.phi_velocity = 0.0
        self.phi_impulse = 0.0
        self.previous_bass = 0.0
        self.beat_cooldown = 0.0
        self.bursts = []
        self.time = 0.0
        self.sprites = []
        self.sprite_palette = None
        self.bands = np.zeros(96)
        self.peaks = np.zeros(96)
        self.history = np.zeros((96, 360, 3), dtype=np.uint8)
        self.history_elapsed = 0.0
        self.history_revision = 0
        self.analysis_rate = None
        self.music_analyzer = MusicAnalyzer()
        self.visual_director = VisualDirector()
        self.horizon_hotspots = HorizonHotspots()
        self.aether_onset = 0.0
        self.aether_beat_phase = 0.0
        self.aether_beat_confidence = 0.0
        self.aether_density = 0.0
        self.aether_scene_morph = 0.0
        self.aether_world_turn = 0.0
        self.aether_beat_pulse = 0.0
        self.aether_phrase_progress = 0.0
        # Fixed star seeds preserve the tunnel when cycling away and back.
        rng = np.random.default_rng(21)
        angles = rng.uniform(0, math.tau, self.PARTICLE_COUNT)
        radii = rng.uniform(1.6, 8.6, self.PARTICLE_COUNT)
        self.particle_vertices = np.column_stack((np.cos(angles)*radii, np.sin(angles)*radii,
                                                 rng.uniform(0, 28, self.PARTICLE_COUNT)))
        self.particle_vertices.flags.writeable = False
        self.radius = radii
        self.tint = rng.integers(0, 4, self.PARTICLE_COUNT)
        self._gpu = None
        self.renderer = "cpu"
        self.renderer_detail = ""
        try:
            from .gpu import GpuCanvas, hardware_available
            available, detail = hardware_available()
            self.renderer_detail = detail
            if available:
                self._gpu = GpuCanvas(self)
                self._gpu.failed.connect(self.use_cpu, Qt.ConnectionType.QueuedConnection)
                self.renderer = "gpu"
        except Exception as error:
            self.renderer_detail = str(error)
        if self.renderer == "cpu" and self.renderer_detail != "headless platform":
            print(f"omaMusi: CPU visual fallback ({self.renderer_detail})", file=sys.stderr)
        self.clock = QElapsedTimer()
        self.clock.start()
        self.timer = QTimer(self)
        self.timer.setTimerType(Qt.TimerType.PreciseTimer)
        self.timer.setInterval(16 if self._gpu else 33)
        self.timer.timeout.connect(self.tick)
        self.timer.start()

    def feed(self, samples):
        n = min(len(samples), len(self.buffer))
        if n:
            self.buffer = np.roll(self.buffer, -n)
            self.buffer[-n:] = samples[-n:]
            self._wave_audio_revision += 1

    def set_cover_art(self, image):
        self.cover_art = QImage(image)
        if self._gpu:
            self._gpu.update()
        self.update()

    def paint_cover_art(self, painter, width, height):
        paint_gallery(painter, self.cover_art, width, height)

    def reset(self):
        self.buffer.fill(0)
        self.wave_persistence.reset()
        self._wave_seen_revision = self._wave_audio_revision
        self._wave_exposure = 0.0
        self.energy = self.bass = self.treble = self.previous_bass = 0
        self.phi_pulse = self.phi_bloom = self.phi_tension = self.phi_event = 0
        self.phi_event_cooldown = 0
        self.phi_velocity = self.phi_impulse = 0
        self.bursts.clear()
        self.bands.fill(0)
        self.peaks.fill(0)
        self.history.fill(0)
        self.history_elapsed = 0
        self.history_revision += 1
        self.music_analyzer.reset()
        self.visual_director.reset()
        self.horizon_hotspots.reset()
        self.aether_onset = 0.0
        self.aether_beat_phase = 0.0
        self.aether_beat_confidence = 0.0
        self.aether_density = 0.0
        self.aether_scene_morph = 0.0
        self.aether_world_turn = 0.0
        self.aether_beat_pulse = 0.0
        self.aether_phrase_progress = 0.0

    def cycle(self, step=1):
        self.mode = (self.mode + step) % len(self.modes)
        if self._gpu:
            self._gpu.update()
        else:
            self.update()
        return self.modes[self.mode]

    def tick(self):
        elapsed = max(0.001, self.clock.nsecsElapsed() / 1e9)
        dt = min(0.05, elapsed)
        self.clock.restart()
        fresh_wave = self.active and self.mode == 2 and self._wave_seen_revision != self._wave_audio_revision
        self._wave_exposure = min(0.1, self._wave_exposure + elapsed)
        self.wave_persistence.step(elapsed, self.waveform() if fresh_wave else None,
                                   exposure=self._wave_exposure)
        if fresh_wave or not self.active or self.mode != 2:
            self._wave_exposure = 0.0
        self._wave_seen_revision = self._wave_audio_revision
        if self.analysis_rate != self.sample_rate:
            self.analysis_rate = self.sample_rate
            self.frequencies = np.fft.rfftfreq(len(self.buffer), 1 / self.sample_rate)
            edges = np.geomspace(30, min(18000, self.sample_rate / 2), len(self.bands) + 1)
            self.band_bins = [np.flatnonzero((self.frequencies >= low) & (self.frequencies < high))
                              for low, high in zip(edges[:-1], edges[1:])]
            self.band_centers = np.sqrt(edges[:-1] * edges[1:])
        levels = np.zeros_like(self.bands)
        if self.active:
            rms = float(np.sqrt(np.mean(self.buffer ** 2)))
            spectrum = np.abs(np.fft.rfft(self.buffer * self.fft_window)) / 1024
            frequencies = self.frequencies
            bass = float(np.sqrt(np.mean(spectrum[(frequencies >= 30) & (frequencies < 220)] ** 2)))
            treble = float(np.mean(spectrum[(frequencies > 2500) & (frequencies < 14000)]))
            target_energy = min(1, rms * 5)
            target_bass = min(1, bass * 8)
            target_treble = min(1, treble * 70)
            magnitudes = np.array([spectrum[bins].max() if len(bins)
                                   else np.interp(center, frequencies, spectrum)
                                   for bins, center in zip(self.band_bins, self.band_centers)])
            levels = np.clip((20 * np.log10(magnitudes + 1e-7) + 66) / 66, 0, 1)
            self.beat_cooldown -= dt
            if target_bass > self.previous_bass + 0.12 and self.beat_cooldown <= 0:
                self.bursts.append([0.0, target_bass])
                self.bursts = self.bursts[-5:]
                self.beat_cooldown = 0.24
            self.previous_bass += (target_bass - self.previous_bass) * min(1, dt * 5)
        else:
            target_energy = target_bass = target_treble = 0
        # Fast attack, softer decay: transients should feel immediate.
        smooth = 1 - math.exp(-dt * (22 if target_energy > self.energy else 8))
        self.energy += (target_energy - self.energy) * smooth
        self.bass += (target_bass - self.bass) * smooth
        self.treble += (target_treble - self.treble) * smooth

        # Phi Cathedral has its own slowly evolving musical state. These values
        # remain downstream of audio playback and never alter decoded samples.
        pulse_target = min(1.0, target_bass * 1.25 + target_energy * 0.35)
        bloom_target = min(1.0, target_energy * 0.75 + target_bass * 0.45)
        tension_target = min(1.0, target_treble * 0.55 + target_energy * 0.35
                             + float(levels[24:72].mean()) * 0.65)
        pulse_rate = 28 if pulse_target > self.phi_pulse else 7
        self.phi_pulse += (pulse_target - self.phi_pulse) * (1 - math.exp(-dt * pulse_rate))
        self.phi_bloom += (bloom_target - self.phi_bloom) * (1 - math.exp(-dt * 2.4))
        self.phi_tension += (tension_target - self.phi_tension) * (1 - math.exp(-dt * 4.0))
        self.phi_event_cooldown = max(0.0, self.phi_event_cooldown - dt)
        fresh_burst = bool(self.bursts and self.bursts[-1][0] < 0.06)
        if (self.active and fresh_burst and self.phi_event_cooldown <= 0
                and target_bass > 0.42 and target_energy > 0.12):
            self.phi_event = 1.0
            self.phi_event_cooldown = 3.2 + (1.0 - target_energy) * 2.8
        self.phi_event *= math.exp(-dt * 1.45)

        transient = max(0.0, target_bass - self.previous_bass)
        self.phi_impulse += transient * 2.8
        self.phi_impulse *= math.exp(-dt * 7.5)
        self.phi_impulse = min(1.0, max(0.0, self.phi_impulse))
        drive_target = 0.18 + target_energy * 1.25 + target_bass * 0.65 + self.phi_impulse
        self.phi_velocity += (drive_target - self.phi_velocity) * (1 - math.exp(-dt * 4.2))
        self.phi_velocity = min(2.6, max(0.08, self.phi_velocity))

        self.time += dt * (0.16 + (0.50 if self.active else 0) + self.energy * 2.6 + self.bass * 0.90 + self.treble * 0.40)
        self.bursts = [[age + dt, strength] for age, strength in self.bursts if age + dt < 1.8]
        self.bands = np.maximum(levels, self.bands * math.exp(-dt * 11))
        self.peaks = np.maximum(self.bands, self.peaks - dt * 0.4)

        metrics = self.music_analyzer.update(
            dt, self.bands, self.energy, self.bass, self.treble
        )
        directed = self.visual_director.update(metrics, dt)
        self.horizon_hotspots.update(dt, metrics, self.active)
        self.aether_onset = metrics.onset
        self.aether_beat_phase = metrics.beat_phase
        self.aether_beat_confidence = metrics.beat_confidence
        self.aether_density = metrics.density
        self.aether_scene_morph = directed.scene_morph
        self.aether_world_turn = directed.world_turn
        self.aether_beat_pulse = directed.beat_pulse
        self.aether_phrase_progress = directed.phrase_progress

        # Keep history warm so switching to the spectrogram isn't a blank view.
        if self.active:
            self.history_elapsed += dt
            if self.history_elapsed >= 1 / 30:
                self.history_elapsed %= 1 / 30
                self.history = np.roll(self.history, -1, axis=1)
                v = self.bands[::-1, None]
                low = np.array(QColor(self.colors["cyan"]).getRgb()[:3])
                high = np.array(QColor(self.colors["accent"]).getRgb()[:3])
                self.history[:, -1] = ((low * (1 - v) + high * v) * v).astype(np.uint8)
                self.history_revision += 1
        if self._gpu:
            self._gpu.update()
        else:
            self.update()

    def use_cpu(self, reason):
        if self._gpu:
            self._gpu.hide()
            self._gpu.cleanup()
            self._gpu.deleteLater()
            self._gpu = None
        self.renderer = "cpu"
        self.renderer_detail = reason
        self.timer.setInterval(33)
        print(f"omaMusi: CPU visual fallback ({reason})", file=sys.stderr)
        self.update()

    def resizeEvent(self, event):
        if self._gpu:
            self._gpu.setGeometry(self.rect())
        super().resizeEvent(event)

    def waveform(self):
        data = self.buffer[-2048:]
        crossings = np.flatnonzero((data[:-1] <= 0) & (data[1:] > 0))
        start = int(crossings[0]) if len(crossings) else 0
        data = data[start:start + 1024]
        gain = min(4, 0.45 / (float(np.sqrt(np.mean(data ** 2))) + 0.04))
        return np.interp(np.linspace(0, len(data)-1, 1024), np.arange(len(data)), data*gain).astype(np.float32)

    def ensure_sprites(self):
        palette = tuple(self.colors.get(key, "#ffffff") for key in ("accent", "cyan", "green", "magenta"))
        if self.sprite_palette == palette:
            return
        self.sprite_palette = palette
        self.sprites = []
        for value in palette:
            image = QPixmap(64, 64)
            image.fill(Qt.GlobalColor.transparent)
            painter = QPainter(image)
            painter.setRenderHint(QPainter.RenderHint.Antialiasing)
            gradient = QRadialGradient(32, 32, 31)
            tint = QColor(value)
            for stop, alpha in ((0, 245), (0.08, 230), (0.22, 100), (0.52, 23), (1, 0)):
                color = QColor(tint)
                color.setAlpha(alpha)
                gradient.setColorAt(stop, color)
            painter.fillRect(image.rect(), gradient)
            painter.setPen(QPen(QColor(value), 0.65))
            painter.drawLine(QPointF(32, 23), QPointF(32, 41))
            painter.drawLine(QPointF(23, 32), QPointF(41, 32))
            painter.end()
            self.sprites.append(image)

    def paintEvent(self, event):
        if self._gpu:
            return
        p = QPainter(self)
        try:
            p.setRenderHint(QPainter.RenderHint.Antialiasing)
            p.setRenderHint(QPainter.RenderHint.SmoothPixmapTransform)
            w, h = self.width(), self.height()
            if self.mode == 7:
                self.paint_cover_art(p, w, h)
                return
            if self.mode != 2:
                p.fillRect(self.rect(), QColor(self.colors["background"]))
            if self.mode == 0:
                self.ensure_sprites()
                self.paint_warp(p, w, h)
            elif self.mode == 1:
                self.paint_spectrum(p, w, h)
            elif self.mode == 2:
                self.paint_waveform(p, w, h)
            elif self.mode == 3:
                self.paint_spectrogram(p, w, h)
            elif self.mode == 4:
                self.paint_phi(p, w, h)
            elif self.mode == 5:
                self.paint_reference_horizon(p, w, h)
            else:
                self.paint_particle_dance(p, w, h)
        finally:
            p.end()

    def color(self, key, alpha=255):
        color = QColor(self.colors[key])
        color.setAlpha(alpha)
        return color

    def paint_spectrum(self, p, w, h):
        baseline = h * 0.76
        step = w / len(self.bands)
        gradient = QLinearGradient(0, baseline, 0, h * 0.2)
        gradient.setColorAt(0, self.color("cyan", 80))
        gradient.setColorAt(0.5, self.color("accent", 190))
        gradient.setColorAt(1, self.color("bright_foreground", 210))
        p.setPen(Qt.PenStyle.NoPen)
        p.setBrush(gradient)
        for i, level in enumerate(self.bands):
            height = max(2, float(level) * h * 0.52)
            p.drawRoundedRect(QRectF(i * step + 2, baseline - height,
                                     max(2, step - 4), height), 2, 2)
        p.setPen(QPen(self.color("accent", 185), 1.5))
        for i, peak in enumerate(self.peaks):
            y = baseline - float(peak) * h * 0.52 - 3
            p.drawLine(QPointF(i * step + 2, y), QPointF((i + 1) * step - 2, y))

    def paint_waveform(self, p, w, h):
        pixels = self.wave_persistence.rgba()
        image = QImage(pixels.data, self.wave_persistence.WIDTH, self.wave_persistence.HEIGHT,
                       self.wave_persistence.WIDTH * 4, QImage.Format.Format_RGBA8888)
        p.drawImage(QRectF(0, 0, w, h), image)
        if self.wave_persistence.latest_alpha < 0.004:
            return
        data = self.wave_persistence.latest
        stride = max(1, len(data) // max(1, w))
        data = data[::stride]
        path = QPainterPath()
        for i, value in enumerate(data):
            x = i * w / max(1, len(data) - 1)
            y = h * 0.47 - float(value) * h * 0.28
            if i:
                path.lineTo(x, y)
            else:
                path.moveTo(x, y)
        p.setPen(QPen(QColor(235, 252, 255, int(255 * self.wave_persistence.latest_alpha)), 1.1))
        p.drawPath(path)

    def paint_spectrogram(self, p, w, h):
        pixels = np.ascontiguousarray(self.history)
        image = QImage(pixels.data, 360, 96, 360 * 3, QImage.Format.Format_RGB888)
        p.setOpacity(0.85)
        p.drawImage(QRectF(0, 0, w, h), image)
        p.setOpacity(1)

    def paint_reference_horizon(self, p, w, h):
        """Reduced reference composition for systems without hardware GL."""
        p.fillRect(self.rect(), QColor(1, 3, 4))
        p.save()
        p.translate(w * 0.82, h * 0.36)
        p.rotate(-18)
        radius = h * 0.49
        glow = QRadialGradient(QPointF(0, 0), radius * 1.5)
        for stop, color in ((0.0, QColor(0, 0, 0)),
                            (0.665, QColor(0, 0, 0)),
                            (0.69, QColor(255, 233, 186)),
                            (0.80, QColor(255, 214, 152)),
                            (0.91, QColor(132, 65, 25)),
                            (1.0, QColor(0, 0, 0, 0))):
            glow.setColorAt(stop, color)
        p.setPen(Qt.PenStyle.NoPen)
        p.setBrush(glow)
        p.drawEllipse(QPointF(0, 0), radius * 1.5, radius * 1.5)
        # Real bass-hit ages, matching the GPU's outward travel and decay.
        p.setBrush(Qt.BrushStyle.NoBrush)
        for age, strength in self.bursts:
            fade = min(1.0, age / 0.07) * max(0.0, 1.0 - age / 1.8)
            wave_radius = radius + h * (0.065 + age * 0.38)
            p.setPen(QPen(QColor(204, 135, 71, int(55 * strength * fade)), 1.5))
            p.drawEllipse(QPointF(0, 0), wave_radius, wave_radius)
        p.setPen(Qt.PenStyle.NoPen)
        # Draw the foreground disk after the shadow and bent-light band.
        baseline = h * 0.19
        disk = QLinearGradient(0, baseline - h * 0.14, 0, baseline + h * 0.14)
        for stop, color in ((0.0, QColor(0, 0, 0, 0)),
                            (0.25, QColor(0, 0, 0, 0)),
                            (0.43, QColor(255, 209, 149, 0)),
                            (0.5, QColor(255, 248, 221)),
                            (0.57, QColor(255, 209, 149, 220)),
                            (0.75, QColor(166, 89, 40, 80)),
                            (1.0, QColor(0, 0, 0, 0))):
            disk.setColorAt(stop, color)
        p.fillRect(QRectF(-w * 1.5, baseline - h * 0.14, w * 3, h * 0.28), disk)
        for age, strength, seed in self.horizon_hotspots.spots:
            orbit = seed * math.tau + age * (1.35 + seed * 0.40)
            orbit_r = h * (0.58 + seed * 0.21)
            fade = min(1.0, age / 0.09) * max(0.0, min(1.0, (4.2 - age) / 2.4))
            x = math.cos(orbit) * orbit_r
            y = baseline - math.sin(orbit) * h * 0.12
            visible = math.sin(orbit) < 0 or math.hypot(x, y) > radius
            if visible:
                p.setBrush(QColor(255, 232, 179, int(210 * strength * fade)))
                p.drawEllipse(QPointF(x, y), h * 0.009, h * 0.009)
                for step in range(1, 6):
                    trail = orbit - step * 0.06
                    p.setBrush(QColor(255, 117, 36,
                                      int(100 * strength * fade * math.exp(-step * 0.45))))
                    p.drawEllipse(QPointF(math.cos(trail) * orbit_r,
                                         baseline - math.sin(trail) * h * 0.12),
                                  h * 0.009, h * 0.009)
            for delay, attenuation in ((0.26, 0.75), (0.62, 0.28)):
                echo_age = age - delay
                if echo_age <= 0:
                    continue
                past = seed * math.tau + echo_age * (1.35 + seed * 0.40)
                if math.sin(past) <= 0:
                    continue
                echo_angle = math.atan2(abs(math.sin(past)) * 0.9 + 0.18, math.cos(past))
                echo_r = radius + h * (0.040 + seed * 0.09)
                p.setBrush(QColor(255, 184, 97, int(170 * strength * fade * attenuation)))
                for side in (-1, 1):
                    p.drawEllipse(QPointF(math.cos(echo_angle) * echo_r,
                                         side * math.sin(echo_angle) * echo_r),
                                  h * 0.010, h * 0.010)
        p.restore()

    def paint_event_horizon(self, p, w, h):
        """CPU fallback for the Event Horizon cinematic accretion-lens visual."""
        scale = min(w, h)
        fly = self.time * 0.038
        center = QPointF(
            w * (0.50
                 + 0.14 * math.sin(fly)
                 + 0.045 * math.sin(self.time * 0.11 + self.phi_bloom * 1.2)),
            h * (0.50
                 + 0.060 * math.sin(fly * 0.73 + 0.8)),
        )
        camera_zoom = 0.88 + 0.24 * (0.5 + 0.5 * math.cos(fly - 0.55))
        view_tilt = 0.18 + 0.70 * (0.5 + 0.5 * math.sin(fly * 0.78 + 0.85))
        foreshorten = 0.36 + (0.97 - 0.36) * view_tilt

        shadow_r = scale * (0.092 + self.bass * 0.028 + self.phi_pulse * 0.015) * camera_zoom
        disc_rx = scale * (0.46 + 0.03 * camera_zoom)
        disc_ry = scale * (0.018 + self.bass * 0.010 + self.phi_pulse * 0.007) * foreshorten

        p.fillRect(self.rect(), QColor("#020100"))
        p.setRenderHint(QPainter.RenderHint.Antialiasing, True)

        haze = QRadialGradient(center, scale * 0.95)
        haze.setColorAt(0.0, QColor(42, 10, 2, int(20 + self.energy * 28)))
        haze.setColorAt(0.55, QColor(18, 4, 1, 10))
        haze.setColorAt(1.0, QColor(0, 0, 0, 0))
        p.fillRect(self.rect(), haze)

        p.setCompositionMode(QPainter.CompositionMode.CompositionMode_Plus)

        # Rich accretion disc.
        band_count = 30
        for i in range(band_count):
            t = i / max(1, band_count - 1)
            rx = disc_rx * (0.62 + t * 0.42)
            ry = disc_ry * (0.54 + t * 0.92)
            wobble = math.sin(self.time * (0.42 + t * 0.55) + t * 8.0) * scale * 0.002
            rect = QRectF(center.x() - rx, center.y() - ry - wobble, rx * 2, ry * 2)
            alpha = int(26 + 130 * (1.0 - t) + self.energy * 26 + self.treble * 16)
            col = QColor(255, int(92 + 120 * (1.0 - t)), int(16 + 20 * (1.0 - t)), min(220, alpha))
            p.setPen(QPen(col, 1.0 + (1.0 - t) * 1.1))
            p.setBrush(Qt.BrushStyle.NoBrush)
            p.drawEllipse(rect)

        # Upper and lower lens arcs.
        p.setPen(QPen(QColor(255, 130, 35, int(44 + self.bass * 24 + self.phi_event * 20)), 2.2))
        p.drawEllipse(QPointF(center.x(), center.y() - shadow_r * 0.01),
                      shadow_r * 2.00, shadow_r * 1.15 * foreshorten)
        p.setPen(QPen(QColor(255, 230, 170, int(12 + self.phi_event * 44)), 1.2))
        p.drawEllipse(QPointF(center.x(), center.y() - shadow_r * 0.03),
                      shadow_r * 1.64, shadow_r * 0.88 * foreshorten)
        p.setPen(QPen(QColor(255, 110, 35, int(18 + self.bass * 18)), 1.1))
        p.drawEllipse(QPointF(center.x(), center.y() + shadow_r * 0.19),
                      shadow_r * 1.40, shadow_r * 0.58 * foreshorten)

        # Sparse sparks.
        p.setPen(Qt.PenStyle.NoPen)
        for i in range(26):
            seed = i * 1.61803398875
            lane = (math.sin(seed * 12.9898) * 43758.5453) % 1.0
            depth = 0.25 + (((math.sin(seed * 4.31) * 9412.3) % 1.0) * 0.75)
            progress = (1.0 - self.time * (0.022 + lane * 0.040) / depth + lane) % 1.0
            radius = scale * (0.16 + progress * 0.44)
            angle = self.time * (0.10 + lane * 0.14) / depth + seed * 2.39996322973
            x = center.x() + math.cos(angle) * radius
            y = center.y() + math.sin(angle) * radius * (0.62 + 0.16 * math.sin(seed))
            size = 0.5 + (1.0 - progress) * 1.2
            alpha = int(10 + 95 * (1.0 - progress))
            p.setBrush(QColor(255, int(100 + 100 * lane), 24, max(6, min(160, alpha))))
            p.drawEllipse(QPointF(x, y), size, size)

        p.setCompositionMode(QPainter.CompositionMode.CompositionMode_SourceOver)

        # Black sphere with a soft limb and subtle penumbra shadow.
        p.setPen(Qt.PenStyle.NoPen)
        for grow, alpha in ((1.45, 26), (1.22, 52), (1.08, 90)):
            p.setBrush(QColor(0, 0, 0, alpha))
            p.drawEllipse(center, shadow_r * 0.72 * grow, shadow_r * 0.72 * grow)
        p.setBrush(QColor("#000000"))
        p.drawEllipse(center, shadow_r * 0.72, shadow_r * 0.72)
        # Inner rim light just inside the limb, brighter toward the bottom.
        p.setPen(QPen(QColor(255, 200, 140, 90), 1.5))
        p.setBrush(Qt.BrushStyle.NoBrush)
        p.drawEllipse(center, shadow_r * 0.68, shadow_r * 0.68)
        p.setPen(QPen(QColor(255, 145, 65, 28), 1.0))
        p.setBrush(Qt.BrushStyle.NoBrush)
        p.drawEllipse(center, shadow_r * 0.75, shadow_r * 0.75)

    def paint_particle_dance(self, p, w, h):
        """CPU fallback for the standalone Particle Dance swarm visual."""
        dance = self.time + self.aether_world_turn * 1.5 + self.aether_beat_phase * math.tau * 0.05
        beat_kick = self.aether_beat_pulse * 0.6 + self.phi_event * 0.4
        boom = (self.aether_beat_pulse * 1.4 + self.aether_onset * 1.2
                + self.phi_impulse * 1.6 + self.phi_event * 1.8)
        drive = 1.0 - math.exp(-boom * 0.9)
        center = QPointF(w * 0.5, h * 0.5)
        scale = min(w, h) * 0.42
        p.setCompositionMode(QPainter.CompositionMode.CompositionMode_SourceOver)
        p.setPen(Qt.PenStyle.NoPen)
        count = 500
        for i in range(count):
            group = i % 13
            seed = (i * 0.61803398875) % 1.0
            per_particle = 0.45 + 1.10 * ((seed * 7.31) % 1.0)
            heading = self.aether_world_turn * (0.70 + group / 13.0 * 1.05) + group / 13.0 * math.tau
            orbit = dance * (0.07 + 0.022 * group) + heading
            cx = math.cos(orbit) * scale * (0.45 + boom * 0.22 * per_particle)
            cy = math.sin(orbit / 1.61803398875) * scale * (0.35 + boom * 0.18 * per_particle)
            shell = (0.38 + (1.48 - 0.38) * (0.5 + 0.5 * math.sin(dance * 0.12 + group * 0.43)))
            shell *= 1.0 + self.aether_beat_pulse * 0.28 + beat_kick * 0.2 + drive * 0.30
            ang = seed * math.tau + dance * (0.70 + self.phi_velocity * 0.20) + beat_kick * 0.8
            rad = shell * scale * (0.25 + 0.75 * ((i * 0.754877666) % 1.0))
            rad *= 1.0 + drive * 0.50 * per_particle
            jiggle = (0.10 + 0.22 * math.sin(dance * 0.7 + i * 0.031 + boom)) * scale * 0.08
            x = center.x() + cx + math.cos(ang) * rad * 0.35 + jiggle * math.sin(i * 1.7)
            y = center.y() + cy + math.sin(ang) * rad * 0.35 + jiggle * math.cos(i * 2.3)
            band = float(self.bands[(i * 13) % 96])
            band_sat = 1.0 - math.exp(-band * 2.5)
            grow = 1.0 + drive * 0.45 + band_sat * 0.35
            size = (2.0 + band * 3.0 + self.bass * 2.5 + self.energy * 2.0
                    + self.phi_impulse * 2.5 + self.aether_beat_pulse * 2.5 + self.phi_event * 3.0) * grow * 0.55
            size = max(1.5, min(14.0, size))
            alpha = int(50 + band * 110 + self.aether_beat_pulse * 60 + self.phi_impulse * 60
                        + self.phi_event * 50)
            # Torus riders: every 6th dot runs a fast tilted ring at ~3x speed.
            # The ring drifts through space and stays dimmer than the shells.
            if i % 6 == 0:
                u = seed * math.tau + dance * 3.1 * (0.8 + drive * 0.6) + beat_kick * 0.3
                v = (seed * 7.77 % 1.0) * math.tau * 3.0 + dance * 1.2
                ring_r = scale * 0.55 * (1.0 + drive * 0.22)
                tube_r = scale * 0.13 * (1.0 + drive * 0.45)
                tilt = 0.42 + 0.16 * math.sin(dance * 0.09 + 1.0)
                lx = (ring_r + tube_r * math.cos(v)) * math.cos(u)
                ly = (ring_r + tube_r * math.cos(v)) * math.sin(u)
                lz = tube_r * math.sin(v)
                drift_x = scale * 0.28 * math.sin(dance * 0.11 + self.aether_world_turn * 0.4)
                drift_y = scale * 0.22 * math.cos(dance * 0.083 + 1.2)
                x = center.x() + drift_x + lx
                y = center.y() + drift_y + (ly * math.cos(tilt) - lz * math.sin(tilt)) * 0.9
                size = min(16.0, size * 1.15)
                alpha = max(10, alpha - 50)
            hue = (seed + self.time * 0.03 + drive * 0.30 + band_sat * 0.45) % 1.0
            tint = QColor.fromHsvF(hue, 0.85, min(0.96, 0.78 + drive * 0.12))
            tint.setAlpha(max(10, min(170, alpha)))
            p.setBrush(tint)
            p.drawEllipse(QPointF(x, y), size, size)
        p.setCompositionMode(QPainter.CompositionMode.CompositionMode_SourceOver)

    def paint_phi(self, p, w, h):
        """CPU fallback: Fibonacci phyllotaxis, logarithmic spirals and pulse rings."""
        phi = (1 + math.sqrt(5)) / 2
        golden_angle = math.tau * (1 - 1 / phi)
        # Same dance conductor as the GPU shaders: beat + director ride
        # on the ambient time so spirals and dots pulse together.
        beat_turn = self.aether_beat_phase * math.tau
        beat_kick = self.aether_beat_pulse * 0.6 + self.phi_event * 0.4
        scene_sway = self.aether_scene_morph - 0.5
        dance = self.time + self.aether_world_turn * 1.5 + beat_turn * 0.05
        center = QPointF(
            w * (0.5 + 0.025 * math.sin(dance / phi + scene_sway * 0.2) + beat_kick * 0.008),
            h * (0.5 + 0.025 * math.cos(dance / (phi * phi)) + beat_kick * 0.008),
        )
        scale = min(w, h) * (0.43 + self.phi_bloom * 0.08 + self.aether_beat_pulse * 0.02)
        p.setCompositionMode(QPainter.CompositionMode.CompositionMode_Plus)

        # Fibonacci symmetry families breathe through 5, 8 and 13-fold structure.
        symmetry = 5 + 3 * (0.5 + 0.5 * math.sin(dance / phi))
        symmetry += 5 * self.phi_tension
        for arm in range(13):
            phase = arm * math.tau / max(5.0, symmetry)
            path = QPainterPath()
            for step in range(96):
                theta = step * 0.12 + phase + dance * 0.18 + beat_turn * 0.03
                radius = 2.0 * math.exp(step * 0.018 * phi)
                radius *= 1 + self.phi_pulse * 0.08 * math.sin(theta * 8 - dance * phi)
                x = center.x() + math.cos(theta) * radius
                y = center.y() + math.sin(theta) * radius
                if step:
                    path.lineTo(x, y)
                else:
                    path.moveTo(x, y)
            key = ("cyan", "accent", "green", "magenta")[arm % 4]
            p.setPen(QPen(self.color(key, int(16 + 36*self.energy + 22*self.phi_event + 20*self.aether_beat_pulse)), 1.0))
            p.drawPath(path)

        count = 377
        wave = self.waveform()
        for i in range(1, count + 1):
            n = i / count
            angle = i * golden_angle + dance * 0.12 + beat_turn * 0.03 + beat_kick * 0.20
            band = float(self.bands[(i * 13) % 96])
            wav = float(wave[(i * 21) % 1024])
            radius = math.sqrt(n) * scale
            radius *= 1 + self.phi_bloom * 0.16 + band * 0.09 + beat_kick * 0.06
            angle += wav * 0.22 + band * 0.18
            x = center.x() + math.cos(angle) * radius
            y = center.y() + math.sin(angle) * radius
            key = ("cyan", "accent", "green", "magenta")[i % 4]
            size = 1.1 + band * 3.0 + self.treble * 1.4 + self.phi_event * 1.8 + self.aether_beat_pulse * 0.9
            p.setPen(Qt.PenStyle.NoPen)
            p.setBrush(self.color(key, int(45 + 125*band + 55*self.energy + 40*self.aether_beat_pulse)))
            p.drawEllipse(QPointF(x, y), size, size)

        for fib in (5, 8, 13, 21):
            radius = scale * (fib / 21) * (0.65 + self.phi_pulse * 0.18 + beat_kick * 0.08)
            alpha = int(8 + self.phi_event * 45 + self.bass * 18 + self.aether_beat_pulse * 30)
            p.setPen(QPen(self.color("accent", alpha), 1.0))
            p.setBrush(Qt.BrushStyle.NoBrush)
            p.drawEllipse(center, radius, radius)

        p.setCompositionMode(QPainter.CompositionMode.CompositionMode_SourceOver)

    def paint_warp(self, p, w, h):
        t = self.time
        flight_amp = 0.7 + self.energy*0.9
        fx = (math.sin(t*0.45)*0.16 + math.sin(t*1.10+1.7)*0.07) * flight_amp
        fy = (math.cos(t*0.33)*0.13 + math.sin(t*0.80+0.6)*0.05) * flight_amp
        center = QPointF(w * (0.57 + fx), h * (0.47 + fy))
        curve_x = math.sin(t*0.65)*h*0.22*(0.5+self.energy*1.0)
        curve_y = math.cos(t*0.52)*h*0.22*(0.5+self.energy*1.0)
        bank = math.sin(t*0.31)*0.65 + self.bass*0.18 + self.energy*0.10
        zoom = 1.0 + 0.12*math.sin(t*0.9) + self.energy*0.15 + self.bass*0.08
        scale = h * 0.72 * zoom
        glow = QRadialGradient(center, h * 0.65)
        tint = QColor(self.colors["accent"])
        tint.setAlpha(int(16 + self.energy * 22))
        glow.setColorAt(0, tint)
        tint.setAlpha(0)
        glow.setColorAt(1, tint)
        p.fillRect(self.rect(), glow)

        p.setCompositionMode(QPainter.CompositionMode.CompositionMode_Plus)
        p.setBrush(Qt.BrushStyle.NoBrush)
        for i in range(18):
            z = (i*28/18-self.time*8) % 28 + 0.7
            radius = scale*1.6/z
            p.setPen(QPen(self.color("cyan" if i % 2 else "accent", int(12+self.energy*18)), 1.2))
            p.drawEllipse(center, radius, radius)
        wave = self.waveform()
        angle = self.time*0.12 + bank
        ca, sa = math.cos(angle), math.sin(angle)
        seeds = self.particle_vertices
        x = seeds[:, 0]*ca-seeds[:, 1]*sa
        y = seeds[:, 0]*sa+seeds[:, 1]*ca
        for i in range(self.PARTICLE_COUNT):
            band = float(self.bands[i % 96])
            wv = float(wave[(i*7) % 1024])
            # Audio waves bend each stream; louder bands run faster and wider.
            # Dive surges stretch the whole field with the music.
            surge = 1.0 + 0.35*math.sin(self.time*0.9) + self.energy*0.6
            perp = wv*0.45*scale/max(1.0, seeds[i, 2])
            speed = (8.0 + band*10.0 + self.energy*3.5 + self.bass*2.0) * surge
            z = (seeds[i, 2]-self.time*speed) % 28 + 0.7
            tail_z = z+0.7+self.energy*2.1+self.bass*0.8+band*1.6
            bend_head = 1.0-z/28.0
            bend_tail = 1.0-tail_z/28.0
            head = QPointF(center.x()+(x[i]+perp)*scale/z+curve_x*bend_head,
                           center.y()-(y[i]+perp*0.6)*scale/z+curve_y*bend_head)
            tail = QPointF(center.x()+x[i]*scale/tail_z+curve_x*bend_tail,
                           center.y()-y[i]*scale/tail_z+curve_y*bend_tail)
            if not self.rect().adjusted(-100, -100, 100, 100).contains(head.toPoint()):
                continue
            color = self.color(("accent", "cyan", "green", "magenta")[int(self.tint[i] % 4)],
                               int((0.30+self.energy*0.40+band*0.30)*255))
            p.setPen(QPen(color, 1.0+self.energy+band*1.7))
            p.drawLine(tail, head)
            size = 4+self.energy*6+band*4
            p.drawPixmap(QRectF(head.x()-size/2, head.y()-size/2, size, size),
                         self.sprites[int(self.tint[i])], QRectF(0, 0, 64, 64))
        p.setOpacity(1)
        p.setCompositionMode(QPainter.CompositionMode.CompositionMode_SourceOver)
        # Subtle breathing core that drifts with the music instead of a hard disc.
        core = h*(0.038+self.bass*0.012+self.energy*0.006+math.sin(self.time*2.3)*0.003)
        p.setPen(QPen(self.color("cyan", int(28+self.energy*20+self.bass*10)), 1.0))
        p.setBrush(QColor(self.colors["background"]).darker(180))
        p.drawEllipse(center, core, core)
