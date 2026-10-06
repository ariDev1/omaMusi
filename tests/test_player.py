"""Integration tests using real FFmpeg decoding; GUI runs offscreen by default."""

import os
os.environ["QT_QPA_PLATFORM"] = "offscreen"

from pathlib import Path
import queue
import random
import subprocess
import tempfile
import threading
import time
import unittest
import warnings
from unittest.mock import patch

import numpy as np
from PySide6.QtCore import Qt
from PySide6.QtTest import QTest
from PySide6.QtWidgets import QApplication, QPushButton, QSlider, QFrame, QWidget
from PySide6.QtMultimedia import QAudioFormat, QMediaDevices
from PySide6.QtGui import QColor

from omamusi.app import PlayerWindow
from omamusi.audio import BYTES_PER_SECOND, Decoder, probe
from omamusi.library import discover
from omamusi.visualizer import Visualizer


class PlayerTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.app = QApplication.instance() or QApplication([])
        cls.app.setQuitOnLastWindowClosed(False)
        cls.temp = tempfile.TemporaryDirectory(prefix="omamusi-test-")
        cls.root = Path(cls.temp.name)
        cls.tone = cls.root / "02 tone.wav"
        subprocess.run(["ffmpeg", "-v", "error", "-f", "lavfi", "-i",
                        "sine=frequency=440:duration=1.2", "-metadata", "title=Test tone",
                        str(cls.tone)], check=True)

    @classmethod
    def tearDownClass(cls):
        cls.temp.cleanup()

    def wait_until(self, predicate, timeout=5000):
        deadline = time.monotonic() + timeout / 1000
        while time.monotonic() < deadline:
            if predicate():
                return
            QTest.qWait(20)
        self.fail("Timed out waiting for playback condition")

    def focus_window(self, window):
        # Offscreen has no compositor to complete activation requests.
        with warnings.catch_warnings():
            warnings.simplefilter("ignore", DeprecationWarning)
            self.app.setActiveWindow(window)
        window.setFocus()

    def test_discovery_natural_sort_dedup_recursion(self):
        folder = self.root / "discovery"
        folder.mkdir(exist_ok=True)
        for name in ("10.mp3", "2.FLAC", "readme.txt"):
            (folder / name).touch()
        nested = folder / "nested"
        nested.mkdir(exist_ok=True)
        (nested / "a.opus").touch()
        self.assertEqual([p.name for p in discover([str(folder), str(folder / "2.FLAC")])],
                         ["2.FLAC", "10.mp3"])
        self.assertEqual(len(discover([str(folder)], recursive=True)), 3)
        with self.assertRaises(ValueError):
            discover([str(folder / "missing")])

    def decode(self, offset=0):
        decoder = Decoder(self.tone, offset)
        chunks = []
        deadline = time.monotonic() + 5
        try:
            while time.monotonic() < deadline:
                try:
                    chunks.append(decoder.chunks.get(timeout=0.05))
                except queue.Empty:
                    if decoder.done.is_set():
                        break
            self.assertTrue(decoder.done.is_set())
            self.assertFalse(decoder.error)
            return b"".join(chunks)
        finally:
            decoder.close()

    def test_real_decode_and_seek(self):
        data = self.decode()
        self.assertAlmostEqual(len(data) / BYTES_PER_SECOND, 1.2, places=2)
        pcm = np.frombuffer(data, dtype="<i2").reshape(-1, 2)
        self.assertGreater(np.max(np.abs(pcm)), 1000)
        self.assertTrue(np.array_equal(pcm[:, 0], pcm[:, 1]))
        self.assertAlmostEqual(len(self.decode(0.7)) / BYTES_PER_SECOND, 0.5, places=2)
        self.assertEqual(probe(self.tone)["title"], "Test tone")

    def test_decoder_error_and_cancellation(self):
        broken = self.root / "broken.mp3"
        broken.write_bytes(b"not an audio file")
        decoder = Decoder(broken)
        self.assertTrue(decoder.done.wait(5))
        self.assertTrue(decoder.error)
        decoder.close()
        decoder = Decoder(self.tone)
        # Let the queue fill, then cancel while its producer is blocked.
        time.sleep(0.25)
        decoder.close()
        self.assertTrue(decoder.done.is_set())
        self.assertIsNotNone(decoder.process.poll())

    def test_window_and_real_playback(self):
        window = PlayerWindow([self.tone, self.tone])
        window.player.set_volume(0)
        errors = []
        window.player.failed.connect(errors.append)
        window.show()
        try:
            self.wait_until(lambda: window.player.position > 0.15)
            self.assertFalse(errors)
            self.assertGreater(window.visualizer.buffer.max(), 0)
            window.toggle_play()
            QTest.qWait(100)
            paused_at = window.player.position
            QTest.qWait(150)
            self.assertAlmostEqual(window.player.position, paused_at, delta=0.04)
            window.seek_relative(0.2)
            self.assertTrue(window.player.paused)
            window.toggle_play()
            self.wait_until(lambda: window.current == 1)
            for _ in range(3):
                QTest.qWait(50)
                self.assertFalse(window.grab().isNull())
            window.search.setText("no match")
            self.assertTrue(window.playlist.item(0).isHidden())
            window.search.clear()
            self.wait_until(lambda: window.current == 1 and "finished" in window.status.text())
            self.assertIsNone(window.player.sink)
            self.assertIn("finished", window.status.text())
            self.assertFalse(errors)
        finally:
            window.close()

    def test_pause_during_pending_metadata_pauses_the_selected_track(self):
        window = PlayerWindow([])
        window.tracks = [self.tone]
        window.populate()
        window.player.set_volume(0)
        release = threading.Event()
        started = threading.Event()
        def slow_probe(path):
            started.set()
            release.wait(0.5)
            return probe(path)
        try:
            with patch("omamusi.app.probe", side_effect=slow_probe):
                window.play_track(0)
                self.wait_until(started.is_set)
                window.toggle_play()
                window.playlist_notice("Saved playlist updated")
                release.set()
                self.wait_until(lambda: window.player.sink is not None)
                self.assertTrue(window.player.paused)
                self.assertEqual(window.status.text(), "Saved playlist updated")
                position = window.player.position
                QTest.qWait(100)
                self.assertAlmostEqual(window.player.position, position, delta=0.04)
        finally:
            release.set()
            window.close()

    def test_text_only_overlay_and_keyboard_filter(self):
        window = PlayerWindow([])
        window.tracks = [self.tone, self.tone]
        window.populate()
        window.show()
        self.focus_window(window)
        window.setFocus()
        QTest.qWait(100)
        try:
            self.assertEqual(window.findChildren(QPushButton), [])
            self.assertEqual(window.findChildren(QSlider), [])
            self.assertEqual(window.playlist.frameShape(), QFrame.Shape.NoFrame)
            QTest.keyClick(window, Qt.Key.Key_J)
            self.assertEqual(window.playlist.currentRow(), 0)
            QTest.keyClick(window, Qt.Key.Key_J)
            self.assertEqual(window.playlist.currentRow(), 1)
            volume = window.player.volume
            QTest.keyClick(window, Qt.Key.Key_Up)
            self.assertEqual(window.playlist.currentRow(), 0)
            QTest.keyClick(window, Qt.Key.Key_Down)
            self.assertEqual(window.playlist.currentRow(), 1)
            self.assertEqual(window.player.volume, volume)
            QTest.keyClick(window, Qt.Key.Key_Plus)
            self.assertAlmostEqual(window.player.volume, volume + 0.05)
            QTest.keyClick(window, Qt.Key.Key_Minus)
            self.assertAlmostEqual(window.player.volume, volume)
            QTest.keyClick(window, Qt.Key.Key_Slash)
            self.assertTrue(window.search.isVisible())
            QTest.keyClicks(window.search, "snvpq")
            self.assertEqual(window.search.text(), "snvpq")
            self.assertTrue(window.isVisible())
            QTest.keyClick(window.search, Qt.Key.Key_Escape)
            window.search.clear()
            QTest.keyClick(window, Qt.Key.Key_Down)
            self.assertEqual(window.playlist.currentRow(), 0)
        finally:
            window.close()

    def test_random_shortcut_advances_and_preserves_previous_history(self):
        window = PlayerWindow([])
        window.tracks = [self.tone, self.tone]
        window.populate()
        window.player.set_volume(0)
        window.show()
        self.focus_window(window)
        window.setFocus()
        try:
            window.play_track(1)
            QTest.keyClick(window, Qt.Key.Key_R)
            self.assertIn("random on", window.hint.text())
            self.assertEqual(window.current, 1)  # Toggling preserves playback.
            QTest.keyClick(window, Qt.Key.Key_N)
            self.assertEqual(window.current, 0)
            self.assertEqual(window.history, [1])
            QTest.keyClick(window, Qt.Key.Key_P)
            self.assertEqual(window.current, 1)
            window.on_finished()
            self.assertEqual(window.current, 0)
            QTest.keyClick(window, Qt.Key.Key_R)
            self.assertIn("random off", window.hint.text())
            window.current = 1
            self.assertIsNone(window.next_index(automatic=True))
        finally:
            window.close()

    def test_random_mode_handles_empty_single_and_all_playlist_tracks(self):
        window = PlayerWindow([])
        window.show()
        self.focus_window(window)
        window.setFocus()
        try:
            QTest.keyClick(window, Qt.Key.Key_R)
            self.assertIn("random on", window.hint.text())
            self.assertIsNone(window.next_index())
            window.tracks = [self.tone]
            window.current = 0
            self.assertEqual(window.next_index(automatic=True), 0)
            window.tracks = [self.tone] * 4
            window.populate()
            window.search.setText("no match")
            for current in range(4):
                window.current = current
                for automatic in (False, True):
                    index = window.next_index(automatic=automatic)
                    self.assertIn(index, set(range(4)) - {current})
            # A seeded real generator proves selection is not fixed or sequential.
            state = random.getstate()
            try:
                random.seed(17)
                window.current = 0
                choices = {window.next_index() for _ in range(30)}
                self.assertEqual(choices, {1, 2, 3})
            finally:
                random.setstate(state)
        finally:
            window.close()

    def test_random_shortcut_respects_text_input_modifiers_and_folder_hint(self):
        window = PlayerWindow([])
        window.show()
        self.focus_window(window)
        window.setFocus()
        try:
            self.assertIn("random off", window.hint.text())
            for modifier in (Qt.KeyboardModifier.ControlModifier,
                             Qt.KeyboardModifier.AltModifier,
                             Qt.KeyboardModifier.MetaModifier):
                QTest.keyClick(window, Qt.Key.Key_R, modifier)
                self.assertIn("random off", window.hint.text())
            window.begin_search()
            QTest.keyClicks(window.search, "random")
            self.assertEqual(window.search.text(), "random")
            self.assertIn("random off", window.hint.text())
            QTest.keyClick(window.search, Qt.Key.Key_Escape)
            window.begin_folder_change()
            window.folder_input.clear()
            QTest.keyClicks(window.folder_input, "random")
            self.assertEqual(window.folder_input.text(), "random")
            self.assertIn("random off", window.hint.text())
            QTest.keyClick(window.folder_input, Qt.Key.Key_Escape)
            window.begin_folder_browse()
            QTest.keyClick(window, Qt.Key.Key_R)
            self.assertIn("random on", window.hint.text())
            window.end_folder_browse()
            self.assertIn("random on", window.hint.text())
        finally:
            window.close()

    def test_change_folder_keyboard_cancel_errors_and_empty_folder(self):
        folder = self.root / "next album"
        folder.mkdir()
        track = folder / "new song.wav"
        track.write_bytes(self.tone.read_bytes())
        empty = self.root / "empty album"
        empty.mkdir()
        window = PlayerWindow([])
        window.player.set_volume(0)
        window.tracks = [self.tone]
        window.populate()
        window.show()
        self.focus_window(window)
        window.setFocus()
        QTest.qWait(100)
        try:
            window.play_track(0)
            self.wait_until(lambda: window.player.position > 0.1)
            window.toggle_play()
            original_sink = window.player.sink
            QTest.keyClick(window, Qt.Key.Key_L, Qt.KeyboardModifier.ControlModifier)
            self.assertTrue(window.folder_input.hasFocus())
            QTest.keyClicks(window.folder_input, "missing folder")
            QTest.keyClick(window.folder_input, Qt.Key.Key_Return)
            self.wait_until(lambda: "Not a folder" in window.status.text())
            self.assertIn("Not a folder", window.status.text())
            self.assertEqual(window.tracks, [self.tone])
            self.assertIs(window.player.sink, original_sink)
            self.assertTrue(window.player.paused)
            QTest.keyClick(window.folder_input, Qt.Key.Key_Escape)
            self.assertFalse(window.folder_prompt.isVisible())
            self.assertEqual(window.current_folder, self.root)

            window.history = [0]
            window.search.setText("no match")
            QTest.keyClick(window, Qt.Key.Key_L, Qt.KeyboardModifier.ControlModifier)
            QTest.keyClicks(window.folder_input, "next album")
            QTest.keyClick(window.folder_input, Qt.Key.Key_Return)
            self.wait_until(lambda: window.player.path == track)
            self.assertEqual(window.current_folder, folder)
            self.assertEqual(window.tracks, [track])
            self.assertEqual(window.current, 0)
            self.assertEqual(window.history, [])
            self.assertEqual(window.search.text(), "")
            self.assertFalse(window.folder_prompt.isVisible())
            self.assertFalse(window.playlist.item(0).isHidden())
            self.assertFalse(window.player.paused)
            self.assertEqual(window.player.path, track)
            self.wait_until(lambda: window.player.position > 0.1)

            window.begin_folder_change()
            window.folder_input.setText(str(track))
            window.change_folder()
            self.wait_until(lambda: "Not a folder" in window.status.text())
            self.assertEqual(window.tracks, [track])
            self.assertTrue(window.folder_prompt.isVisible())
            window.folder_input.setText(str(empty))
            window.change_folder()
            self.wait_until(lambda: window.current_folder == empty)
            self.assertEqual(window.current_folder, empty)
            self.assertEqual(window.folder_label.text(), str(empty))
            self.assertEqual(window.tracks, [])
            self.assertEqual(window.current, -1)
            self.assertIsNone(window.player.sink)
            self.assertIsNone(window.player.path)
            self.assertEqual(window.player.position, 0)
            self.assertIn("No audio files", window.title.text())
            self.assertEqual(window.playlist.count(), 0)
        finally:
            window.close()

    def test_volume_shift_keypad_text_entry_and_window_isolation(self):
        window = PlayerWindow([])
        other = QWidget()
        window.tracks = [self.tone]
        window.populate()
        window.player.set_volume(0)
        window.show()
        self.focus_window(window)
        window.setFocus()
        QTest.qWait(100)
        try:
            window.play_track(0)
            self.wait_until(lambda: window.player.position > 0.1)
            window.toggle_play()
            for key, modifiers, expected in (
                (Qt.Key.Key_Plus, Qt.KeyboardModifier.ShiftModifier, 0.05),
                (Qt.Key.Key_Plus, Qt.KeyboardModifier.KeypadModifier, 0.10),
                (Qt.Key.Key_Equal, Qt.KeyboardModifier.NoModifier, 0.15),
                (Qt.Key.Key_Minus, Qt.KeyboardModifier.KeypadModifier, 0.10),
            ):
                QTest.keyClick(window, key, modifiers)
                self.assertAlmostEqual(window.player.volume, expected)
                self.assertAlmostEqual(window.player.sink.volume(), expected)
                self.assertIn(f"vol {expected:.0%}", window.settings_label.text())
            window.player.seek(0.2)
            self.assertTrue(window.player.paused)
            self.assertAlmostEqual(window.player.sink.volume(), 0.10)
            for key, modifiers in (
                (Qt.Key.Key_Plus, Qt.KeyboardModifier.MetaModifier),
                (Qt.Key.Key_Minus, Qt.KeyboardModifier.ControlModifier),
                (Qt.Key.Key_VolumeUp, Qt.KeyboardModifier.NoModifier),
                (Qt.Key.Key_VolumeDown, Qt.KeyboardModifier.NoModifier),
                (Qt.Key.Key_VolumeMute, Qt.KeyboardModifier.NoModifier),
            ):
                QTest.keyClick(window, key, modifiers)
                self.assertAlmostEqual(window.player.volume, 0.10)
            window.begin_search()
            QTest.keyClicks(window.search, "+-=")
            self.assertEqual(window.search.text(), "+-=")
            self.assertAlmostEqual(window.player.volume, 0.10)
            window.escape()
            window.begin_folder_change()
            QTest.keyClicks(window.folder_input, "+-=")
            self.assertEqual(window.folder_input.text(), "+-=")
            self.assertAlmostEqual(window.player.volume, 0.10)
            window.escape()
            other.show()
            self.focus_window(other)
            other.setFocus()
            QTest.qWait(100)
            QTest.keyClick(other, Qt.Key.Key_Plus, Qt.KeyboardModifier.ShiftModifier)
            QTest.keyClick(other, Qt.Key.Key_Q)
            self.assertAlmostEqual(window.player.volume, 0.10)
            self.assertTrue(window.isVisible())
            self.focus_window(window)
            window.setFocus()
            QTest.qWait(100)
            for _ in range(25):
                QTest.keyClick(window, Qt.Key.Key_Plus, Qt.KeyboardModifier.ShiftModifier)
            self.assertEqual(window.player.volume, 1)
            self.assertEqual(window.player.sink.volume(), 1)
            for _ in range(25):
                QTest.keyClick(window, Qt.Key.Key_Minus)
            self.assertEqual(window.player.volume, 0)
            self.assertEqual(window.player.sink.volume(), 0)
        finally:
            window.close()
            other.close()

    def test_folder_browser_navigation_play_and_cancel(self):
        base = self.root / "browser"
        base.mkdir()
        album = base / "Album 2"
        album.mkdir()
        (base / "Album 10").mkdir()
        start = base / "start.wav"
        start.write_bytes(self.tone.read_bytes())
        track = album / "song.wav"
        track.write_bytes(self.tone.read_bytes())
        window = PlayerWindow([])
        window.player.set_volume(0)
        window.show()
        self.focus_window(window)
        window.setFocus()
        QTest.qWait(100)
        try:
            window.load_folder(base)
            self.wait_until(lambda: window.player.position > 0.1)
            window.toggle_play()
            sink = window.player.sink
            QTest.keyClick(window, Qt.Key.Key_C)
            self.wait_until(lambda: window.browser_folder == base)
            self.assertEqual(window.browser_folder, base)
            self.assertTrue(window.folder_list.isVisible())
            self.assertFalse(window.playlist.isVisible())
            self.assertIn("↑↓ select", window.hint.text())
            self.assertEqual(window.folder_list.item(2).text().strip(), "Album 2/")
            QTest.keyClick(window, Qt.Key.Key_Down)
            self.assertEqual(window.folder_list.currentRow(), 1)
            QTest.keyClick(window, Qt.Key.Key_Up)
            self.assertEqual(window.folder_list.currentRow(), 0)
            QTest.keyClick(window, Qt.Key.Key_Down)
            QTest.keyClick(window, Qt.Key.Key_Down)
            self.assertEqual(window.player.volume, 0)
            QTest.keyClick(window, Qt.Key.Key_Right)
            self.wait_until(lambda: window.browser_folder == album)
            self.assertEqual(window.browser_folder, album)
            self.assertEqual(window.current_folder, base)
            self.assertIs(window.player.sink, sink)
            self.assertTrue(window.player.paused)
            self.assertEqual(window.folder_list.item(2).text().strip(), "song.wav")
            self.assertIn("1 audio files", window.count.text())
            # The first entry is "play this folder"; Enter loads it.
            QTest.keyClick(window, Qt.Key.Key_Return)
            self.wait_until(lambda: window.player.path == track)
            self.assertIsNone(window.browser_folder)
            self.assertEqual(window.tracks, [track])
            self.assertFalse(window.player.paused)
            window.toggle_play()
            QTest.keyClick(window, Qt.Key.Key_C, Qt.KeyboardModifier.ShiftModifier)
            self.wait_until(lambda: window.browser_folder == album)
            self.assertEqual(window.browser_folder, album)
            QTest.keyClick(window, Qt.Key.Key_Left)
            self.wait_until(lambda: window.browser_folder == base)
            self.assertEqual(window.browser_folder, base)
            QTest.keyClick(window, Qt.Key.Key_Escape)
            self.assertIsNone(window.browser_folder)
            self.assertEqual(window.tracks, [track])
            self.assertEqual(window.folder_label.text(), str(album))
            self.assertTrue(window.player.paused)
            window.resize(640, 420)
            QTest.qWait(50)
            self.assertTrue(window.rect().contains(window.hint.geometry()))
            self.assertEqual(window.findChildren(QPushButton), [])
        finally:
            window.close()

    def test_compact_layout_and_audio_reactive_sprites(self):
        window = PlayerWindow([])
        window.tracks = [self.tone]
        window.populate()
        window.show()
        QTest.qWait(50)
        try:
            self.assertFalse(window.panel.isVisible())
            self.assertFalse(window.count.isVisible())
            self.assertFalse(window.settings_label.isVisible())
            window.tracks = [self.tone] * 3
            window.populate()
            QTest.qWait(50)
            self.assertTrue(window.panel.isVisible())
            self.assertLess(window.playlist.height(), window.height() / 4)
            self.assertNotIn("01", window.playlist.item(0).text())
            visual = window.visualizer
            window.state_timer.stop()
            window.player.timer.stop()
            phase = np.arange(4096) / visual.sample_rate
            visual.feed((0.3 * np.sin(phase * 80 * 2 * np.pi)).astype(np.float32))
            visual.active = True
            QTest.qWait(300)
            self.assertGreater(visual.energy, 0.1)
            self.assertGreater(visual.bass, 0.1)
            self.assertTrue(visual.bursts)
            self.assertEqual(len(visual.sprites), 4)
            self.assertEqual(len(visual.radius), Visualizer.PARTICLE_COUNT)
            self.assertFalse(window.grab().isNull())
            visual.active = False
            energy = visual.energy
            QTest.qWait(300)
            self.assertLess(visual.energy, energy / 2)
        finally:
            window.close()

    def test_high_resolution_float_decode_and_native_playback(self):
        path = self.root / "high resolution.wav"
        subprocess.run(["ffmpeg", "-v", "error", "-f", "lavfi", "-i",
                        "sine=frequency=997:sample_rate=96000:duration=0.3",
                        "-c:a", "pcm_s24le", str(path)], check=True)
        self.assertEqual(probe(path)["sample_rate"], 96000)
        decoder = Decoder(path, sample_rate=96000, floating=True)
        chunks = []
        try:
            deadline = time.monotonic() + 5
            while time.monotonic() < deadline:
                try:
                    chunks.append(decoder.chunks.get(timeout=0.05))
                except queue.Empty:
                    if decoder.done.is_set():
                        break
            self.assertTrue(decoder.done.is_set())
            pcm = np.frombuffer(b"".join(chunks), dtype="<f4")
            self.assertAlmostEqual(len(pcm) / (96000 * 2), 0.3, places=3)
            self.assertGreater(float(np.abs(pcm).max()), 0.05)
        finally:
            decoder.close()
        window = PlayerWindow([])
        window.tracks = [path]
        window.populate()
        window.player.set_volume(0)
        try:
            window.play_track(0)
            self.wait_until(lambda: window.player.position > 0.05)
            candidate = QAudioFormat()
            candidate.setSampleRate(96000)
            candidate.setChannelCount(2)
            candidate.setSampleFormat(QAudioFormat.SampleFormat.Float)
            if QMediaDevices.defaultAudioOutput().isFormatSupported(candidate):
                self.assertEqual(window.player.sink.format().sampleRate(), 96000)
                self.assertEqual(window.player.sink.format().sampleFormat(), QAudioFormat.SampleFormat.Float)
                self.assertEqual(window.visualizer.sample_rate, 96000)
        finally:
            window.close()

    def test_long_playlist_uses_window_height_and_spectrogram_fills_background(self):
        window = PlayerWindow([], mode=3)
        window.tracks = [self.root / f"Track {i:02}.wav" for i in range(60)]
        window.populate()
        window.show()
        QTest.qWait(100)
        try:
            row_height = window.playlist.sizeHintForRow(0)
            self.assertGreater(window.playlist.viewport().height() / row_height, 8)
            self.assertGreater(window.playlist.height(), window.height() * 0.55)
            self.assertLess(window.panel.geometry().bottom(), window.title.geometry().top())
            old_height = window.playlist.height()
            window.resize(1060, 900)
            QTest.qWait(100)
            self.assertGreater(window.playlist.height(), old_height + 150)
            window.playlist.setCurrentRow(59)
            QTest.qWait(50)
            self.assertTrue(window.playlist.viewport().rect().intersects(
                window.playlist.visualItemRect(window.playlist.item(59))))
            visual = window.visualizer
            visual.history[:] = (160, 100, 50)
            image = visual.grab().toImage()
            background = QColor(visual.colors["background"])
            self.assertNotEqual(image.pixelColor(image.width() // 2, 1), background)
            self.assertNotEqual(image.pixelColor(image.width() // 2, image.height() - 2), background)
        finally:
            window.close()

    def test_visualization_keys_render_all_modes_without_restarting_audio(self):
        window = PlayerWindow([])
        window.tracks = [self.tone]
        window.populate()
        window.player.set_volume(0)
        window.show()
        self.focus_window(window)
        QTest.qWait(100)
        try:
            window.play_track(0)
            self.wait_until(lambda: window.player.position > 0.15)
            window.toggle_play()
            sink = window.player.sink
            position = window.player.position
            visual = window.visualizer
            geometry = visual.particle_vertices.copy()
            frames = []
            for expected in range(len(Visualizer.modes)):
                self.assertEqual(visual.mode, expected)
                image = visual.grab().toImage()
                self.assertFalse(image.isNull())
                frames.append(bytes(image.bits()))
                QTest.keyClick(window, Qt.Key.Key_V)
                self.assertEqual(window.visual_notice.text(), Visualizer.modes[(expected + 1) % len(Visualizer.modes)])
                self.assertTrue(window.visual_notice.isVisible())
                self.assertIs(window.player.sink, sink)
                self.assertTrue(window.player.paused)
            self.assertEqual(len(set(frames)), len(Visualizer.modes))
            self.assertEqual(visual.mode, 0)
            np.testing.assert_array_equal(visual.particle_vertices, geometry)
            self.assertFalse(visual.particle_vertices.flags.writeable)
            self.assertGreater(visual.bands.max(), 0)
            self.assertGreater(visual.history.max(), 0)
            self.assertAlmostEqual(window.player.position, position, delta=0.04)
            QTest.keyClick(window, Qt.Key.Key_V, Qt.KeyboardModifier.ShiftModifier)
            self.assertEqual(visual.mode, len(Visualizer.modes) - 1)
            window.begin_search()
            QTest.keyClicks(window.search, "vV")
            self.assertEqual(window.search.text(), "vV")
            self.assertEqual(visual.mode, len(Visualizer.modes) - 1)
        finally:
            window.close()

    def test_audio_feeding_continues_when_ui_thread_is_blocked(self):
        window = PlayerWindow([])
        window.tracks = [self.tone]
        window.populate()
        window.player.set_volume(0)
        try:
            window.play_track(0)
            self.wait_until(lambda: window.player.position > 0.15)
            before = window.player.worker_snapshot
            self.assertEqual(before["underruns"], 0)
            # No Qt UI events or visual timers run during this stall.
            time.sleep(0.45)
            after = window.player.worker_snapshot
            self.assertGreater(after["position"], before["position"] + 0.35)
            self.assertGreater(after["written"], before["written"])
            self.assertEqual(after["underruns"], 0)
            self.assertLessEqual(len(window.player._engine.mailbox[1]), 4096)
        finally:
            window.close()

    def test_folder_browser_shows_files_and_plays_selected_song(self):
        folder = self.root / "file browser"
        folder.mkdir()
        first = folder / "2 song.wav"
        selected = folder / "10 song.WAV"
        for path in (first, selected):
            path.write_bytes(self.tone.read_bytes())
        window = PlayerWindow([])
        window.player.set_volume(0)
        window.current_folder = folder
        window.show()
        self.focus_window(window)
        window.setFocus()
        QTest.qWait(100)
        try:
            QTest.keyClick(window, Qt.Key.Key_C)
            self.wait_until(lambda: window.browser_folder == folder)
            self.assertEqual(window.folder_list.item(2).text().strip(), first.name)
            self.assertEqual(window.folder_list.item(3).text().strip(), selected.name)
            self.assertIn("2 audio files", window.count.text())
            for _ in range(3):
                QTest.keyClick(window, Qt.Key.Key_Down)
            QTest.keyClick(window, Qt.Key.Key_Return)
            self.wait_until(lambda: window.player.path == selected)
            self.assertIsNone(window.browser_folder)
            self.assertEqual(window.tracks, [first, selected])
            self.assertEqual(window.current, 1)
            self.assertEqual(window.player.path, selected)
            self.assertEqual(window.playlist.currentRow(), 1)
            self.wait_until(lambda: window.player.position > 0.1)
        finally:
            window.close()


    def test_phi_cathedral_state_is_audio_reactive_and_bounded(self):
        window = PlayerWindow([], mode=4)
        window.show()
        QTest.qWait(50)
        try:
            visual = window.visualizer
            visual.timer.stop()
            window.state_timer.stop()
            window.player.timer.stop()
            phase = np.arange(4096) / visual.sample_rate
            signal = (0.38*np.sin(phase*75*2*np.pi)
                      + 0.16*np.sin(phase*997*2*np.pi)).astype(np.float32)
            visual.feed(signal)
            visual.active = True
            for _ in range(20):
                visual.clock.restart()
                QTest.qWait(12)
                visual.tick()
            self.assertGreater(visual.phi_pulse, 0.05)
            self.assertGreater(visual.phi_bloom, 0.03)
            self.assertGreaterEqual(visual.phi_tension, 0.0)
            self.assertGreater(visual.phi_velocity, 0.08)
            for value in (visual.phi_pulse, visual.phi_bloom, visual.phi_tension,
                          visual.phi_event, visual.phi_impulse):
                self.assertTrue(np.isfinite(value))
                self.assertGreaterEqual(value, 0.0)
                self.assertLessEqual(value, 1.05)

            self.assertTrue(np.isfinite(visual.phi_velocity))
            self.assertGreaterEqual(visual.phi_velocity, 0.08)
            self.assertLessEqual(visual.phi_velocity, 2.6)
            self.assertFalse(window.grab().isNull())
        finally:
            window.close()


if __name__ == "__main__":
    unittest.main()
