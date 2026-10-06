"""CPU rendering must work without desktop theme files and release painters."""
import os
os.environ["QT_QPA_PLATFORM"] = "offscreen"

from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from PySide6.QtGui import QImage, QPainter
from PySide6.QtWidgets import QApplication

from omamusi.theme import load_theme
from omamusi.visualizer import Visualizer


class ThemeFallbackTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.app = QApplication.instance() or QApplication([])

    def test_all_cpu_visuals_render_without_omarchy_theme_files(self):
        visual = Visualizer()
        visual.timer.stop()
        self.addCleanup(visual.close)
        with tempfile.TemporaryDirectory() as temporary, patch.object(Path, "home", return_value=Path(temporary)):
            visual.colors, _ = load_theme()
        visual.ensure_sprites()
        for paint in (visual.paint_warp, visual.paint_spectrum, visual.paint_waveform,
                      visual.paint_spectrogram, visual.paint_phi, visual.paint_reference_horizon,
                      visual.paint_particle_dance):
            with self.subTest(visual=paint.__name__):
                image = QImage(640, 420, QImage.Format.Format_ARGB32_Premultiplied)
                image.fill(0)
                painter = QPainter(image)
                try:
                    paint(painter, 640, 420)
                finally:
                    painter.end()

    def test_paint_failure_releases_painter_before_propagating(self):
        visual = Visualizer()
        visual.timer.stop()
        visual.mode = 1
        self.addCleanup(visual.close)
        image = QImage(640, 420, QImage.Format.Format_ARGB32_Premultiplied)
        painter = QPainter(image)
        class ImagePainter:
            RenderHint = QPainter.RenderHint
            def __new__(cls, widget):
                return painter
        try:
            with patch("omamusi.visualizer.QPainter", ImagePainter), \
                 patch.object(visual, "paint_spectrum", side_effect=RuntimeError("render failed")):
                with self.assertRaisesRegex(RuntimeError, "render failed"):
                    visual.paintEvent(None)
            self.assertFalse(painter.isActive(), "Render exception left the paint device locked")
        finally:
            if painter.isActive():
                painter.end()
