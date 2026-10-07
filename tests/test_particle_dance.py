"""Particle Dance keeps small dots and gives the gold ring a clear role."""
import os
os.environ['QT_QPA_PLATFORM'] = 'offscreen'

import unittest
from PySide6.QtGui import QColor, QImage, QPainter
from PySide6.QtWidgets import QApplication

from omamusi.visualizer import Visualizer


class RecordingPainter(QPainter):
    def __init__(self, image):
        super().__init__(image)
        self.dots = []

    def drawEllipse(self, center, radius_x, radius_y):
        self.dots.append((center.x(), center.y(), radius_x, radius_y, self.brush().color()))
        super().drawEllipse(center, radius_x, radius_y)


class ParticleDanceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.app = QApplication.instance() or QApplication([])

    def render(self, level=0, phase=12):
        visual = Visualizer()
        visual.timer.stop()
        self.addCleanup(visual.close)
        visual.resize(960, 640)
        visual.time = phase
        for name in ('energy', 'bass', 'treble', 'phi_impulse', 'phi_event',
                     'aether_onset', 'aether_beat_pulse', 'aether_density'):
            setattr(visual, name, level)
        visual.bands.fill(level)
        image = QImage(960, 640, QImage.Format.Format_ARGB32_Premultiplied)
        image.fill(QColor('black'))
        painter = RecordingPainter(image)
        visual.paint_particle_dance(painter, 960, 640)
        painter.end()
        return painter.dots

    def test_particles_stay_small_in_quiet_and_loud_passages(self):
        quiet, loud = self.render(0), self.render(1)
        self.assertTrue(quiet and loud)
        self.assertLessEqual(max(dot[2] for dot in loud), 1.5)
        self.assertEqual([dot[2:4] for dot in quiet], [dot[2:4] for dot in loud])

    def test_gold_ring_is_prominent_wide_and_near_the_center(self):
        for phase in (0, 12, 40):
            with self.subTest(phase=phase):
                dots = self.render(phase=phase)
                gold = [dot for dot in dots if dot[4].red() > 180
                        and .5 < dot[4].green() / dot[4].red() < .85
                        and dot[4].blue() / dot[4].red() < .3]
                self.assertGreater(len(gold), len(dots) * .25)
                xs, ys = [dot[0] for dot in gold], [dot[1] for dot in gold]
                self.assertGreater(max(xs) - min(xs), 640 * .55)
                self.assertAlmostEqual((min(xs) + max(xs)) / 2, 480, delta=960 * .05)
                self.assertAlmostEqual((min(ys) + max(ys)) / 2, 320, delta=640 * .05)
