"""Waveform density, decay, and transparency through real CPU rendering."""
import importlib.util
import os
os.environ["QT_QPA_PLATFORM"] = "offscreen"

from pathlib import Path
import subprocess
import tempfile
import unittest
import numpy as np
from PySide6.QtCore import Qt
from PySide6.QtTest import QTest
from PySide6.QtWidgets import QApplication

from omamusi.app import PlayerWindow


class PersistenceTests(unittest.TestCase):
    def model(self):
        self.assertIsNotNone(importlib.util.find_spec("omamusi.visual.waveform"),
                             "waveform persistence is missing")
        from omamusi.visual.waveform import WaveformPersistence
        return WaveformPersistence()

    def test_repeated_traces_heat_up_and_old_traces_remain_then_fade(self):
        model = self.model()
        model.step(1 / 30, np.zeros(1024))
        cold = model.rgba()
        row = int(np.argmax(cold[:, 100, 3]))
        self.assertGreater(cold[row, 100, 2], cold[row, 100, 0])
        for _ in range(60):
            model.step(1 / 30, np.zeros(1024))
        hot = model.rgba()
        self.assertGreater(hot[row, 100, 0], hot[row, 100, 2])
        self.assertGreater(hot[row, 100, 3], cold[row, 100, 3])
        model.step(1 / 30, np.full(1024, 0.5))
        combined = model.rgba()
        self.assertGreater(combined[row, 100, 3], 0)
        self.assertGreater(np.count_nonzero(combined[:, 100, 3]),
                           np.count_nonzero(hot[:, 100, 3]))
        model.step(2)
        self.assertLess(int(model.rgba()[:, :, 3].max()), 20)
        model.reset()
        self.assertFalse(model.rgba().any())

    def test_density_is_independent_of_frame_rate_and_remains_bounded(self):
        slow, fast = self.model(), self.model()
        for _ in range(60):
            slow.step(1 / 30, np.zeros(1024))
        for _ in range(120):
            fast.step(1 / 60, np.zeros(1024))
        np.testing.assert_allclose(slow.density, fast.density, rtol=0.04, atol=0.001)
        self.assertTrue(np.isfinite(slow.density).all())
        self.assertLessEqual(int(slow.rgba()[:, :, 3].max()), 255)

    def test_sparse_audio_updates_use_accumulated_exposure(self):
        slow, fast = self.model(), self.model()
        for _ in range(60):
            slow.step(1 / 30, np.zeros(1024))
            fast.step(1 / 60)
            fast.step(1 / 60, np.zeros(1024), exposure=1 / 30)
        np.testing.assert_allclose(slow.density, fast.density, rtol=0.001, atol=0.001)


class WaveformWindowTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.app = QApplication.instance() or QApplication([])
        cls.app.setQuitOnLastWindowClosed(False)

    def window(self, mode=2):
        window = PlayerWindow([], mode)
        self.addCleanup(window.close)
        window.visualizer.timer.stop()
        window.show()
        return window

    def test_only_waveform_background_is_transparent_across_switches(self):
        window = self.window()
        image = window.grab().toImage()
        self.assertEqual(image.pixelColor(1, 1).alpha(), 0)
        for _ in range(2 * len(window.visualizer.modes)):
            window.cycle_view()
            image = window.grab().toImage()
            expected = 0 if window.visualizer.mode == 2 else 255
            self.assertEqual(image.pixelColor(1, 1).alpha(), expected)
        self.assertEqual(window.windowOpacity(), 1.0)

    def test_new_audio_populates_map_repainting_does_not_redeposit_and_reset_clears(self):
        window = self.window()
        visual = window.visualizer
        self.assertTrue(hasattr(visual, "wave_persistence"), "waveform needs a persistence map")
        visual.active = True
        visual.feed(np.sin(np.linspace(0, 16 * np.pi, 4096)).astype(np.float32) * 0.3)
        QTest.qWait(20)
        visual.active = True
        visual.tick()
        self.assertGreater(visual.wave_persistence.density.max(), 0)
        snapshot = visual.wave_persistence.density.copy()
        window.grab()
        window.grab()
        np.testing.assert_array_equal(visual.wave_persistence.density, snapshot)
        QTest.qWait(20)
        visual.tick()
        self.assertLess(visual.wave_persistence.density.max(), snapshot.max())
        self.assertEqual(window.grab().toImage().pixelColor(1, 1).alpha(), 0)
        visual.reset()
        self.assertFalse(visual.wave_persistence.rgba().any())

    def test_changing_tracks_and_seeking_clear_the_persistence(self):
        window = self.window()
        self.assertTrue(hasattr(window.visualizer, "wave_persistence"))
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "tone.wav"
            subprocess.run(["ffmpeg", "-v", "error", "-f", "lavfi", "-i",
                            "sine=frequency=440:duration=3", str(path)], check=True)
            window.tracks = [path]
            window.populate()
            window.player.set_volume(0)
            persistence = window.visualizer.wave_persistence
            persistence.step(1 / 30, np.zeros(1024))
            window.play_track(0)
            self.assertFalse(persistence.rgba().any())
            for _ in range(100):
                if window.player.sink is not None:
                    break
                QTest.qWait(20)
            self.assertIsNotNone(window.player.sink)
            persistence.step(1 / 30, np.zeros(1024))
            window.seek_relative(1)
            self.assertFalse(persistence.rgba().any())
            window.player.stop(clear=True)
