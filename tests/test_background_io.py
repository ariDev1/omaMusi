"""Slow or failed file access must not freeze or overwrite newer UI requests."""
import os
os.environ["QT_QPA_PLATFORM"] = "offscreen"

from pathlib import Path
import tempfile
import threading
import time
import unittest
from unittest.mock import patch

from PySide6.QtCore import QTimer
from PySide6.QtTest import QTest
from PySide6.QtWidgets import QApplication

from omamusi import app as player_app
from omamusi.library import discover


class BackgroundIoTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.app = QApplication.instance() or QApplication([])
        cls.app.setQuitOnLastWindowClosed(False)

    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.first = self.root / "first.wav"
        self.second = self.root / "second.wav"
        self.first.touch()
        self.second.touch()
        self.window = player_app.PlayerWindow([])
        self.window.player.set_volume(0)
        self.addCleanup(self.window.close)
        self.window.show()

    def wait_until(self, predicate):
        deadline = time.monotonic() + 3
        while not predicate() and time.monotonic() < deadline:
            QTest.qWait(10)
        self.assertTrue(predicate(), "Background operation did not complete")

    def test_slow_probe_keeps_ui_responsive_and_latest_track_wins(self):
        release = threading.Event()
        started = threading.Event()
        def slow_probe(path):
            if path == self.first:
                started.set()
                release.wait(0.5)
            return {"title": path.stem, "artist": "", "album": "", "duration": 2, "sample_rate": 48000}
        self.window.tracks = [self.first, self.second]
        self.window.populate()
        try:
            with patch("omamusi.app.probe", side_effect=slow_probe):
                before = time.monotonic()
                self.window.play_track(0)
                self.assertLess(time.monotonic() - before, 0.2)
                self.wait_until(started.is_set)
                ticked = []
                QTimer.singleShot(0, lambda: ticked.append(True))
                self.wait_until(lambda: bool(ticked))
                self.window.play_track(1)
                release.set()
                self.wait_until(lambda: self.window.player.path == self.second)
                self.assertEqual(self.window.title.text(), "second")
                self.assertEqual(self.window.current, 1)
        finally:
            release.set()

    def test_slow_scan_preserves_queue_until_success(self):
        folder = self.root / "album"
        folder.mkdir()
        (folder / "song.wav").touch()
        self.window.tracks = [self.first]
        self.window.populate()
        release = threading.Event()
        def slow_scan(*args, **kwargs):
            release.wait(0.5)
            return discover(*args, **kwargs)
        try:
            with patch("omamusi.app.discover", side_effect=slow_scan):
                before = time.monotonic()
                self.window.load_folder(folder)
                self.assertLess(time.monotonic() - before, 0.2)
                self.assertEqual(self.window.tracks, [self.first])
                release.set()
                self.wait_until(lambda: self.window.tracks == [folder / "song.wav"])
        finally:
            release.set()

    def test_failed_add_reports_error_and_preserves_queue(self):
        self.window.tracks = [self.first]
        self.window.populate()
        with patch("omamusi.app.discover", side_effect=PermissionError("folder is unreadable")):
            self.window.add_paths([str(self.root)])
            self.wait_until(lambda: "unreadable" in self.window.status.text())
        self.assertEqual(self.window.tracks, [self.first])

    def test_missing_track_reports_error_without_replacing_queue(self):
        self.window.tracks = [self.first]
        self.window.populate()
        self.first.unlink()
        self.window.play_track(0)
        self.wait_until(lambda: "error" in self.window.status.text())
        self.assertEqual(self.window.tracks, [self.first])
        self.assertIsNone(self.window.player.sink)

    def test_closing_does_not_wait_for_slow_probe(self):
        release = threading.Event()
        started = threading.Event()
        def slow_probe(path):
            started.set()
            release.wait(0.5)
            return {"title": "late", "artist": "", "album": "", "duration": 2, "sample_rate": 48000}
        self.window.tracks = [self.first]
        self.window.populate()
        try:
            with patch("omamusi.app.probe", side_effect=slow_probe):
                self.window.play_track(0)
                self.wait_until(started.is_set)
                before = time.monotonic()
                self.window.close()
                self.assertLess(time.monotonic() - before, 0.2)
                release.set()
                QTest.qWait(30)
                self.assertIsNone(self.window.player.path)
        finally:
            release.set()

    def test_cancel_pending_folder_change_preserves_queue(self):
        self.window.tracks = [self.first]
        self.window.populate()
        self.window.begin_folder_change()
        self.window.folder_input.setText(str(self.root))
        release = threading.Event()
        started = threading.Event()
        done = threading.Event()
        def slow_scan(*args, **kwargs):
            started.set()
            release.wait(0.5)
            result = discover(*args, **kwargs)
            done.set()
            return result
        try:
            with patch("omamusi.app.discover", side_effect=slow_scan):
                self.window.change_folder()
                self.wait_until(started.is_set)
                self.window.escape()
                release.set()
                self.wait_until(done.is_set)
                QTest.qWait(40)
                self.assertEqual(self.window.tracks, [self.first])
        finally:
            release.set()

    def test_cancel_pending_browser_does_not_reopen_it(self):
        release = threading.Event()
        started = threading.Event()
        done = threading.Event()
        from omamusi.library import directory_entries
        def slow_list(folder):
            started.set()
            release.wait(0.5)
            result = directory_entries(folder)
            done.set()
            return result
        try:
            with patch("omamusi.app.directory_entries", side_effect=slow_list):
                before = time.monotonic()
                self.window.show_folder(self.root)
                self.assertLess(time.monotonic() - before, 0.2)
                self.wait_until(started.is_set)
                self.window.escape()
                release.set()
                self.wait_until(done.is_set)
                QTest.qWait(40)
                self.assertIsNone(self.window.browser_folder)
                self.assertFalse(self.window.folder_list.isVisible())
        finally:
            release.set()

    def test_repeated_add_requests_keep_all_files(self):
        release = threading.Event()
        started = threading.Event()
        def slow_scan(*args, **kwargs):
            started.set()
            release.wait(0.5)
            return discover(*args, **kwargs)
        try:
            with patch("omamusi.app.discover", side_effect=slow_scan):
                self.window.add_paths([str(self.first)])
                self.wait_until(started.is_set)
                self.window.add_paths([str(self.second)])
                release.set()
                self.wait_until(lambda: self.window.tracks == [self.first, self.second])
        finally:
            release.set()

    def test_add_requests_preserve_each_requests_recursion_setting(self):
        nested = self.root / "nested"
        nested.mkdir()
        subfolder = nested / "subfolder"
        subfolder.mkdir()
        third = subfolder / "third.wav"
        third.touch()
        other = self.root / "other"
        other.mkdir()
        fourth = other / "fourth.wav"
        fourth.touch()
        deeper = other / "deeper"
        deeper.mkdir()
        (deeper / "excluded.wav").touch()
        release = threading.Event()
        started = threading.Event()
        def slow_scan(*args, **kwargs):
            started.set()
            release.wait(0.5)
            return discover(*args, **kwargs)
        try:
            with patch("omamusi.app.discover", side_effect=slow_scan):
                self.window.add_paths([str(nested)], recursive=True)
                self.wait_until(started.is_set)
                self.window.add_paths([str(other)], recursive=False)
                release.set()
                self.wait_until(lambda: len(self.window.tracks) >= 2)
                self.assertEqual(self.window.tracks, [third, fourth])
        finally:
            release.set()

    def test_add_does_not_replace_track_waiting_for_metadata(self):
        self.window.tracks = [self.first]
        self.window.populate()
        release = threading.Event()
        started = threading.Event()
        def slow_probe(path):
            started.set()
            release.wait(0.5)
            return {"title": path.stem, "artist": "", "album": "", "duration": 2, "sample_rate": 48000}
        try:
            with patch("omamusi.app.probe", side_effect=slow_probe):
                self.window.play_track(0)
                self.wait_until(started.is_set)
                self.window.add_paths([str(self.second)])
                self.wait_until(lambda: len(self.window.tracks) == 2)
                self.assertEqual(self.window.current, 0)
                release.set()
                self.wait_until(lambda: self.window.player.path == self.first)
        finally:
            release.set()
