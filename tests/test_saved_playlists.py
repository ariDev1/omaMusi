"""Saved playlist persistence and keyboard workflows."""
import os
os.environ["QT_QPA_PLATFORM"] = "offscreen"

import json
import importlib.util
from pathlib import Path
import subprocess
import tempfile
import time
import unittest
from unittest.mock import patch

from PySide6.QtCore import Qt
from PySide6.QtTest import QTest
from PySide6.QtWidgets import QApplication, QDialog

from omamusi.app import PlayerWindow


class PlaylistStoreTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.file = Path(self.temp.name) / "saved" / "playlists.json"

    def store(self):
        self.assertIsNotNone(importlib.util.find_spec("omamusi.playlists"),
                             "saved playlist storage is missing")
        from omamusi.playlists import PlaylistStore
        return PlaylistStore(self.file)

    def test_saved_playlists_persist_order_and_prevent_duplicates(self):
        store = self.store()
        first, second = Path(self.temp.name) / "a.flac", Path(self.temp.name) / "b.mp3"
        store.create(" Favorites ", first)
        self.assertFalse(store.add("Favorites", first))
        self.assertTrue(store.add("Favorites", second))
        store.create("Other")
        reopened = self.store()
        self.assertEqual(reopened.names(), ["Favorites", "Other"])
        self.assertEqual(reopened.tracks("Favorites"), [first, second])
        reopened.rename("Favorites", "Night music")
        reopened.remove("Night music", first)
        reopened.delete("Other")
        self.assertEqual(self.store().names(), ["Night music"])
        self.assertEqual(self.store().tracks("Night music"), [second])
        self.assertFalse(first.exists())  # Operations only change references.

    def test_invalid_names_and_collisions_do_not_replace_playlists(self):
        store = self.store()
        store.create("Favorites")
        for name in ("", "  ", "favorites", "x" * 121):
            with self.assertRaises(ValueError):
                store.create(name)
        store.create("Other")
        with self.assertRaises(ValueError):
            store.rename("Other", "Favorites")
        self.assertEqual(self.store().names(), ["Favorites", "Other"])

    def test_bad_data_and_failed_save_preserve_existing_file(self):
        store = self.store()
        store.create("Favorites")
        original = self.file.read_bytes()
        with patch("omamusi.playlists.os.replace", side_effect=OSError("disk full")):
            with self.assertRaises(OSError):
                store.add("Favorites", Path("song.wav"))
        self.assertEqual(self.file.read_bytes(), original)
        self.assertEqual(store.tracks("Favorites"), [])
        for contents in ("not json", json.dumps({"version": 1, "playlists": {"bad": [42]}})):
            self.file.write_text(contents)
            with self.assertRaises(ValueError):
                self.store()
            self.assertEqual(self.file.read_text(), contents)


class SavedPlaylistKeyboardTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.app = QApplication.instance() or QApplication([])
        cls.app.setQuitOnLastWindowClosed(False)

    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.environment = patch.dict(os.environ, {"XDG_DATA_HOME": str(self.root)})
        self.environment.start()
        self.addCleanup(self.environment.stop)
        self.window = PlayerWindow([])
        self.addCleanup(self.window.close)
        self.window.show()
        self.window.activateWindow()
        self.window.setFocus()
        QTest.qWait(20)

    def store(self):
        self.assertIsNotNone(importlib.util.find_spec("omamusi.playlists"),
                             "saved playlist storage is missing")
        from omamusi.playlists import PlaylistStore
        return PlaylistStore()

    def dialog(self):
        dialogs = [d for d in self.window.findChildren(QDialog) if d.isVisible()]
        self.assertEqual(len(dialogs), 1, "shortcut should open a playlist chooser")
        self.addCleanup(dialogs[0].close)
        return dialogs[0]

    def test_add_selected_create_duplicate_and_cancel(self):
        first, second = self.root / "first.wav", self.root / "second.wav"
        self.window.tracks = [first, second]
        self.window.current = 0
        self.window.populate()
        self.window.playlist.setCurrentRow(1)
        QTest.keyClick(self.window, Qt.Key.Key_A)
        dialog = self.dialog()
        QTest.keyClick(dialog, Qt.Key.Key_Return)
        QTest.keyClicks(dialog.name_input, "Favorites")
        QTest.keyClick(dialog.name_input, Qt.Key.Key_Return)
        self.assertEqual(self.store().tracks("Favorites"), [second])
        self.assertEqual(self.window.current, 0)
        QTest.keyClick(self.window, Qt.Key.Key_A)
        dialog = self.dialog()
        QTest.keyClick(dialog, Qt.Key.Key_Return)
        self.assertIn("Already in playlist", self.window.status.text())
        self.window.playlist.setCurrentRow(-1)
        QTest.keyClick(self.window, Qt.Key.Key_A)
        dialog = self.dialog()
        QTest.keyClick(dialog, Qt.Key.Key_Escape)
        self.assertEqual(self.store().tracks("Favorites"), [second])
        QTest.keyClick(self.window, Qt.Key.Key_A)
        dialog = self.dialog()
        QTest.keyClick(dialog, Qt.Key.Key_Return)
        self.assertEqual(self.store().tracks("Favorites"), [second, first])

    def test_browser_marks_missing_loads_queue_and_manages_playlists(self):
        song = self.root / "song.wav"
        subprocess.run(["ffmpeg", "-v", "error", "-f", "lavfi", "-i",
                        "sine=frequency=440:duration=2", str(song)], check=True)
        missing = self.root / "missing.wav"
        store = self.store()
        store.create("Favorites", missing)
        store.add("Favorites", song)
        QTest.keyClick(self.window, Qt.Key.Key_B)
        dialog = self.dialog()
        QTest.keyClick(dialog, Qt.Key.Key_Right)
        self.assertIn("missing", dialog.listing.item(0).text().lower())
        QTest.keyClick(dialog, Qt.Key.Key_Delete)
        self.assertEqual(self.store().tracks("Favorites"), [song])
        QTest.keyClick(dialog, Qt.Key.Key_Left)
        QTest.keyClick(dialog, Qt.Key.Key_F2)
        QTest.keyClicks(dialog.name_input, "Night music")
        QTest.keyClick(dialog.name_input, Qt.Key.Key_Return)
        self.assertEqual(self.store().names(), ["Night music"])
        self.window.random_playback = True
        QTest.keyClick(dialog, Qt.Key.Key_Return)
        self.assertEqual(self.window.tracks, [song])
        self.assertTrue(self.window.random_playback)
        self.window.player.stop(clear=True)
        QTest.keyClick(self.window, Qt.Key.Key_B)
        dialog = self.dialog()
        QTest.keyClick(dialog, Qt.Key.Key_Delete)
        QTest.keyClick(dialog, Qt.Key.Key_Escape)
        self.assertEqual(self.store().names(), ["Night music"])
        QTest.keyClick(dialog, Qt.Key.Key_Delete)
        QTest.keyClick(dialog, Qt.Key.Key_Return)
        self.assertEqual(self.store().names(), [])
        self.assertTrue(song.exists())

    def test_loading_skips_missing_and_empty_playlist_preserves_playback(self):
        song = self.root / "song.wav"
        subprocess.run(["ffmpeg", "-v", "error", "-f", "lavfi", "-i",
                        "sine=frequency=440:duration=2", str(song)], check=True)
        store = self.store()
        store.create("Mixed", self.root / "missing.wav")
        store.add("Mixed", song)
        QTest.keyClick(self.window, Qt.Key.Key_B)
        dialog = self.dialog()
        QTest.keyClick(dialog, Qt.Key.Key_Return)
        self.assertEqual(self.window.tracks, [song])
        self.assertIn("1 missing", self.window.status.text())
        store.create("Empty")
        self.window.player.stop(clear=True)
        QTest.keyClick(self.window, Qt.Key.Key_B)
        dialog = self.dialog()
        QTest.keyClick(dialog, Qt.Key.Key_Return)
        self.assertEqual(self.window.tracks, [song])
        self.assertIn("no available tracks", dialog.message.text().lower())

    def test_shortcuts_leave_text_input_and_modified_keys_alone(self):
        self.window.begin_search()
        QTest.keyClicks(self.window.search, "ab")
        self.assertEqual(self.window.search.text(), "ab")
        self.assertFalse(self.window.findChildren(QDialog))
        QTest.keyClick(self.window.search, Qt.Key.Key_Escape)
        for key in (Qt.Key.Key_A, Qt.Key.Key_B):
            QTest.keyClick(self.window, key, Qt.KeyboardModifier.ControlModifier)
        self.assertFalse(self.window.findChildren(QDialog))
        QTest.keyClick(self.window, Qt.Key.Key_A)
        self.assertFalse(self.window.findChildren(QDialog))
        self.assertIn("select a track", self.window.status.text().lower())

    def test_folder_track_can_be_added_to_chosen_playlist(self):
        track = self.root / "selected.flac"
        track.write_bytes(b"reference only; never played in this test")
        store = self.store()
        store.create("Ambient")
        store.create("Favorites")
        self.window.show_folder(self.root)
        deadline = time.monotonic() + 3
        while self.window.browser_folder != self.root and time.monotonic() < deadline:
            QTest.qWait(10)
        self.assertEqual(self.window.browser_folder, self.root)
        for row in range(self.window.folder_list.count()):
            if self.window.folder_list.item(row).data(Qt.ItemDataRole.UserRole) == track:
                self.window.folder_list.setCurrentRow(row)
                break
        QTest.keyClick(self.window, Qt.Key.Key_A)
        dialog = self.dialog()
        QTest.keyClick(dialog, Qt.Key.Key_Down)
        QTest.keyClick(dialog, Qt.Key.Key_Return)
        self.assertEqual(self.store().tracks("Ambient"), [])
        self.assertEqual(self.store().tracks("Favorites"), [track])
        self.assertEqual(self.window.tracks, [])
        self.assertEqual(self.window.browser_folder, self.root)

    def test_write_error_keeps_chooser_open_and_saved_playlist_unchanged(self):
        store = self.store()
        store.create("Favorites")
        self.window.tracks = [self.root / "song.flac"]
        self.window.populate()
        self.window.playlist.setCurrentRow(0)
        QTest.keyClick(self.window, Qt.Key.Key_A)
        dialog = self.dialog()
        with patch("omamusi.playlists.os.replace", side_effect=OSError("disk full")):
            QTest.keyClick(dialog, Qt.Key.Key_Return)
        self.assertTrue(dialog.isVisible())
        self.assertIn("disk full", dialog.message.text())
        self.assertEqual(self.store().tracks("Favorites"), [])
        QTest.keyClick(dialog, Qt.Key.Key_Return)
        self.assertEqual(self.store().tracks("Favorites"), self.window.tracks)
