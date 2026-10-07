"""Local artwork extraction and a single reactive cover on a black stage."""
import importlib.util
import os
os.environ["QT_QPA_PLATFORM"] = "offscreen"

from pathlib import Path
import subprocess
import tempfile
import threading
import time
import unittest
import numpy as np
from unittest.mock import patch

from PySide6.QtGui import QColor, QImage
from PySide6.QtTest import QTest
from PySide6.QtWidgets import QApplication

from omamusi.app import PlayerWindow, parser
from omamusi.visualizer import Visualizer


class ArtworkTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.picture = self.root / "source.png"
        image = QImage(200, 100, QImage.Format.Format_RGB32)
        image.fill(QColor("#ed633f"))
        self.assertTrue(image.save(str(self.picture)))

    def load(self, path):
        self.assertIsNotNone(importlib.util.find_spec("omamusi.artwork"), "Artwork loading is missing")
        from omamusi.artwork import load_artwork
        return load_artwork(path)

    def test_embedded_mp3_and_flac_artwork(self):
        alternate = QImage(80, 80, QImage.Format.Format_RGB32)
        alternate.fill(QColor("blue"))
        self.assertTrue(alternate.save(str(self.root / "cover.png")))
        for extension, codec in (("mp3", "libmp3lame"), ("flac", "flac")):
            with self.subTest(format=extension):
                path = self.root / f"track.{extension}"
                subprocess.run(["ffmpeg", "-v", "error", "-f", "lavfi", "-i",
                                "sine=frequency=440:duration=0.2", "-i", str(self.picture),
                                "-map", "0:a", "-map", "1:v", "-c:a", codec, "-c:v", "copy",
                                "-disposition:v", "attached_pic", str(path)], check=True)
                image = self.load(path)
                self.assertFalse(image.isNull())
                self.assertEqual(image.width(), 2 * image.height())
                self.assertEqual(image.pixelColor(image.width() // 2, image.height() // 2), QColor("#ed633f"))

    def test_local_cover_file_and_invalid_cover_fallback(self):
        path = self.root / "track.wav"
        path.touch()
        (self.root / "cover.jpg").write_bytes(b"invalid image")
        self.picture.rename(self.root / "folder.png")
        self.assertFalse(self.load(path).isNull())

    def test_no_artwork_returns_empty_image(self):
        path = self.root / "track.wav"
        path.touch()
        self.assertTrue(self.load(path).isNull())

    def test_large_sidecar_is_downscaled(self):
        image = QImage(2400, 1200, QImage.Format.Format_RGB32)
        image.fill(QColor("red"))
        self.assertTrue(image.save(str(self.root / "cover.png")))
        image = self.load(self.root / "track.wav")
        self.assertFalse(image.isNull())
        self.assertLessEqual(max(image.width(), image.height()), 1200)
        self.assertEqual(image.width(), 2 * image.height())


class CoverArtWindowTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.app = QApplication.instance() or QApplication([])
        cls.app.setQuitOnLastWindowClosed(False)

    def window(self):
        self.assertIn("Cover Art", Visualizer.modes, "Cover Art visual is missing")
        window = PlayerWindow([], Visualizer.modes.index("Cover Art"))
        window.player.set_volume(0)
        self.addCleanup(window.close)
        window.visualizer.timer.stop()
        window.show()
        return window

    def wait_until(self, predicate):
        deadline = time.monotonic() + 4
        while not predicate() and time.monotonic() < deadline:
            QTest.qWait(10)
        self.assertTrue(predicate())

    def test_empty_stage_is_opaque_black(self):
        visual = self.window().visualizer
        image = visual.grab().toImage()
        for x, y in ((1, 1), (image.width() // 2, image.height() // 2),
                     (image.width() - 2, image.height() - 2)):
            self.assertEqual(image.pixelColor(x, y), QColor("black"))

    def test_single_cover_stays_on_right_and_preserves_aspect_at_window_shapes(self):
        window = self.window()
        self.assertIsNotNone(importlib.util.find_spec("omamusi.visual.cover"))
        from omamusi.visual.cover import cover_rectangle
        art = QImage(200, 100, QImage.Format.Format_RGB32)
        art.fill(QColor("red"))
        window.visualizer.set_cover_art(art)
        for width, height in ((1060, 680), (640, 900), (1600, 500)):
            with self.subTest(shape=(width, height)):
                window.resize(width, height)
                visual = window.visualizer
                image = visual.grab().toImage()
                rgba = image.convertToFormat(QImage.Format.Format_RGBA8888)
                pixels = np.frombuffer(rgba.bits(), dtype=np.uint8).reshape(rgba.height(), rgba.width(), 4)
                left = pixels[:, :rgba.width() // 2]
                self.assertFalse(np.any((left[:, :, 0] == 255) & (left[:, :, 1] == 0)
                                        & (left[:, :, 2] == 0)), "Artwork appeared on the left")
                for bass in (0, 1):
                    rectangle = cover_rectangle(visual.width(), visual.height(), art.size(), bass)
                    self.assertGreaterEqual(rectangle.left(), visual.width() / 2)
                    self.assertAlmostEqual(rectangle.width() / rectangle.height(), 2)
                    self.assertTrue(visual.rect().contains(rectangle.toAlignedRect()))
                center = cover_rectangle(visual.width(), visual.height(), art.size()).center().toPoint()
                self.assertEqual(image.pixelColor(center), QColor("red"))

    def test_music_changes_cover_scale_on_a_black_stage(self):
        window = self.window()
        visual = window.visualizer
        art = QImage(200, 200, QImage.Format.Format_RGB32)
        art.fill(QColor("red"))
        visual.set_cover_art(art)
        quiet = visual.grab().toImage().convertToFormat(QImage.Format.Format_RGBA8888)
        visual.bass = visual.energy = 1.0
        loud = visual.grab().toImage().convertToFormat(QImage.Format.Format_RGBA8888)
        def cover_pixels(image):
            pixels = np.frombuffer(image.bits(), dtype=np.uint8).reshape(image.height(), image.width(), 4)
            return np.count_nonzero((pixels[:, :, 0] == 255) & (pixels[:, :, 1] == 0)
                                    & (pixels[:, :, 2] == 0))
        self.assertGreater(cover_pixels(loud), cover_pixels(quiet) * 1.02)
        self.assertNotEqual(bytes(quiet.bits()), bytes(loud.bits()))
        from omamusi.visual.cover import cover_rectangle
        rectangle = cover_rectangle(visual.width(), visual.height(), art.size())
        x, y = int(rectangle.left() - 30), int(rectangle.center().y())
        self.assertEqual(quiet.pixelColor(x, y), QColor("black"))
        self.assertEqual(loud.pixelColor(x, y), QColor("black"))

    def test_cover_survives_seek_reset_and_cycles_with_other_visuals(self):
        window = self.window()
        art = QImage(80, 80, QImage.Format.Format_RGB32)
        art.fill(QColor("red"))
        window.visualizer.set_cover_art(art)
        window.visualizer.reset()
        self.assertFalse(window.visualizer.cover_art.isNull())
        window.cycle_view()
        self.assertEqual(window.visualizer.mode, Visualizer.modes.index("Cover Gallery"))
        window.cycle_view(-1)
        self.assertEqual(window.visualizer.mode, Visualizer.modes.index("Cover Art"))
        self.assertEqual(parser().parse_args(["--view", "cover art"]).view, "cover art")

    def test_late_artwork_cannot_replace_new_track_or_empty_queue(self):
        window = self.window()
        self.assertIsNotNone(importlib.util.find_spec("omamusi.artwork"))
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            first, second = root / "one.wav", root / "two.wav"
            first.touch()
            second.touch()
            window.tracks = [first, second]
            window.populate()
            release, started = threading.Event(), threading.Event()
            second_release, second_started = threading.Event(), threading.Event()
            red = QImage(80, 80, QImage.Format.Format_RGB32)
            red.fill(QColor("red"))
            def load(path):
                if path == first:
                    started.set()
                    release.wait(1)
                    return red
                second_started.set()
                second_release.wait(1)
                return QImage()
            try:
                with patch("omamusi.app.load_artwork", side_effect=load):
                    window.play_track(0)
                    self.wait_until(started.is_set)
                    window.play_track(1)
                    release.set()
                    self.wait_until(second_started.is_set)
                    # The first result has finished; the second stays blocked
                    # while the UI has time to receive and reject the old art.
                    QTest.qWait(40)
                    self.assertTrue(window.visualizer.cover_art.isNull())
                    window.visualizer.set_cover_art(red)
                    window.load_folder(root, tracks=[])
                    self.assertTrue(window.visualizer.cover_art.isNull())
            finally:
                release.set()
                second_release.set()
