"""Artwork grouping, gradual rotation, and click-to-play gallery behavior."""
import os
os.environ['QT_QPA_PLATFORM'] = 'offscreen'

from pathlib import Path
import tempfile
import threading
import time
import unittest
from unittest.mock import patch

from PySide6.QtCore import QPointF, QRectF, Qt
from PySide6.QtGui import QColor, QImage, QPainter
from PySide6.QtTest import QTest
from PySide6.QtWidgets import QApplication

from omamusi.app import PlayerWindow, parser
from omamusi.gallery import CoverGallery, load_gallery_artwork
from omamusi.visual.gallery import paint_gallery
from omamusi.visualizer import Visualizer


def art(color):
    image = QImage(80, 80, QImage.Format.Format_RGB32)
    image.fill(QColor(color))
    return image


class GalleryModelTests(unittest.TestCase):
    def test_identical_artwork_groups_songs_even_in_different_folders(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            gallery = CoverGallery()
            tracks = []
            for folder in ('Album', 'Other copy'):
                album = root / folder
                album.mkdir()
                self.assertTrue(art('red').save(str(album / 'cover.png')))
                path = album / 'song.wav'
                path.touch()
                tracks.append(path)
                gallery.add(path, load_gallery_artwork(path))
            self.assertEqual(len(gallery.covers), 1)
            cover = next(iter(gallery.covers.values()))
            self.assertEqual(set(cover.tracks), set(tracks))
            self.assertEqual(cover.pick(tracks[0]), tracks[1])
            with patch('omamusi.gallery.random.choice', side_effect=lambda values: values[-1]):
                self.assertEqual(cover.pick(), tracks[1])

    def test_rotation_changes_one_cover_and_hover_keeps_click_target_still(self):
        gallery = CoverGallery()
        for index, color in enumerate(('red', 'blue', 'green', 'yellow', 'cyan', 'magenta', 'white')):
            gallery.add(Path(f'/music/album{index}/song.flac'), art(color))
        bounds = QRectF(0, 0, 650, 400)
        rectangles = gallery.layout(bounds)
        self.assertEqual(len(rectangles), 6)
        before = [tile.key for tile in gallery.tiles]
        gallery.tick(6)
        self.assertEqual(sum(a != tile.key for a, tile in zip(before, gallery.tiles)), 1)
        gallery.hover(rectangles[0].center(), bounds)
        key = gallery.tiles[0].key
        for _ in range(30):
            gallery.tick(1)
        self.assertEqual(gallery.tiles[0].key, key)
        self.assertIs(gallery.hit(rectangles[0].center(), bounds), gallery.covers[key])

    def test_small_collection_never_repeats_tiles_and_memory_is_bounded(self):
        gallery = CoverGallery()
        gallery.add(Path('/music/song.flac'), QImage())
        self.assertEqual(gallery.layout(QRectF(0, 0, 600, 400)), [])
        for index in range(gallery.MAX_COVERS + 10):
            image = art(QColor.fromHsv(index % 360, 255, 255))
            gallery.add(Path(f'/music/{index}.flac'), image)
        self.assertLessEqual(sum(not cover.image.isNull() for cover in gallery.covers.values()),
                             gallery.MAX_COVERS)
        gallery.layout(QRectF(0, 0, 600, 400))
        self.assertEqual(len({tile.key for tile in gallery.tiles}), len(gallery.tiles))

    def test_two_covers_can_still_rotate_without_duplicate_tiles(self):
        gallery = CoverGallery()
        gallery.add(Path('/music/one.flac'), art('red'))
        gallery.add(Path('/music/two.flac'), art('blue'))
        gallery.layout(QRectF(0, 0, 650, 400))
        before = [tile.key for tile in gallery.tiles]
        gallery.tick(6)
        self.assertNotEqual([tile.key for tile in gallery.tiles], before)

    def test_new_tiles_do_not_reuse_artwork_still_fading_out(self):
        gallery = CoverGallery()
        gallery.add(Path('/music/one.flac'), art('red'))
        gallery.add(Path('/music/two.flac'), art('blue'))
        area = QRectF(0, 0, 650, 400)
        with patch('omamusi.gallery.random.choice', side_effect=lambda values: values[0]):
            gallery.layout(area)
            gallery.tick(6)
            outgoing = gallery.tiles[0].previous
            gallery.add(Path('/music/three.flac'), art('green'))
            rectangles = gallery.layout(area)
        self.assertNotEqual(gallery.tiles[1].key, outgoing)
        gallery.hover(rectangles[0].center(), area)
        self.assertEqual(len({tile.key for tile in gallery.tiles}), len(gallery.tiles))

    def test_crossfade_preserves_brightness_at_midpoint(self):
        gallery = CoverGallery()
        gallery.add(Path('/music/one.flac'), art('red'))
        gallery.add(Path('/music/two.flac'), art('blue'))
        area = QRectF(0, 0, 300, 300)
        rectangle = gallery.layout(area)[0]
        gallery.tick(6)
        gallery.tiles[0].fade = .5
        image = QImage(300, 300, QImage.Format.Format_ARGB32_Premultiplied)
        painter = QPainter(image)
        paint_gallery(painter, gallery, 300, 300, area)
        painter.end()
        color = image.pixelColor(rectangle.center().toPoint())
        self.assertAlmostEqual(color.red(), 128, delta=2)
        self.assertAlmostEqual(color.blue(), 128, delta=2)
        self.assertEqual(color.alpha(), 255)


class GalleryWindowTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.app = QApplication.instance() or QApplication([])
        cls.app.setQuitOnLastWindowClosed(False)

    def window(self):
        window = PlayerWindow([], Visualizer.modes.index('Cover Gallery'))
        self.addCleanup(window.close)
        window.visualizer.timer.stop()
        window.show()
        return window

    def wait_until(self, predicate):
        deadline = time.monotonic() + 4
        while not predicate() and time.monotonic() < deadline:
            QTest.qWait(10)
        self.assertTrue(predicate(), 'Gallery did not finish loading')

    def test_click_plays_a_random_song_sharing_the_cover(self):
        window = self.window()
        window.tracks = [Path('/music/one.flac'), Path('/music/two.flac')]
        with patch('omamusi.app.load_gallery_artwork', return_value=QImage()):
            window.populate()
            self.wait_until(lambda: not window.visualizer.gallery.scanning)
        for path in window.tracks:
            window.visualizer.gallery.add(path, art('red'))
        window.current = 0
        QTest.qWait(20)
        rectangles = window.visualizer.gallery.layout(window.gallery_bounds())
        with patch.object(window, 'play_track') as play:
            QTest.mouseClick(window, Qt.MouseButton.LeftButton, pos=rectangles[0].center().toPoint())
            play.assert_called_once_with(1)
        self.assertEqual(parser().parse_args(['--view', 'cover gallery']).view, 'cover gallery')

    def test_late_gallery_artwork_cannot_repopulate_a_replaced_queue(self):
        window = self.window()
        entered, release = threading.Event(), threading.Event()
        def slow_load(path):
            entered.set()
            release.wait(2)
            return art('red')
        try:
            with patch('omamusi.app.load_gallery_artwork', side_effect=slow_load):
                window.tracks = [Path('/music/old.flac')]
                window.populate()
                self.wait_until(entered.is_set)
                window.load_folder(Path('/music'), tracks=[])
                release.set()
                QTest.qWait(80)
                self.assertEqual(window.visualizer.gallery.covers, {})
        finally:
            release.set()

    def test_tiles_fit_different_window_shapes_and_stay_clear_of_controls(self):
        window = self.window()
        for index, color in enumerate(('red', 'blue', 'green', 'cyan', 'yellow', 'magenta')):
            window.visualizer.gallery.add(Path(f'/music/{index}.flac'), art(color))
        for size in ((1060, 680), (640, 420), (640, 900), (1600, 500)):
            window.resize(*size)
            QTest.qWait(10)
            bounds = window.gallery_bounds()
            rectangles = window.visualizer.gallery.layout(bounds)
            self.assertTrue(rectangles)
            for rectangle in rectangles:
                self.assertTrue(bounds.contains(rectangle))
                self.assertFalse(rectangle.intersects(QRectF(window.hint.geometry())))
            image = window.visualizer.grab().toImage()
            self.assertEqual(image.pixelColor(1, 1), QColor('black'))

    def test_gallery_stays_below_folder_path_input(self):
        window = self.window()
        window.begin_folder_change()
        QTest.qWait(20)
        self.assertGreaterEqual(window.gallery_bounds().top(), window.folder_prompt.geometry().bottom())
