"""Search music by filename and nested folder names."""
import os
os.environ["QT_QPA_PLATFORM"] = "offscreen"

from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from PySide6.QtWidgets import QApplication
from PySide6.QtCore import Qt
from PySide6.QtTest import QTest
from omamusi.app import PlayerWindow, parser, run_player


class SearchTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.app = QApplication.instance() or QApplication([])

    def test_search_arrows_focus_matching_results_and_enter_plays(self):
        window = PlayerWindow([])
        try:
            window.tracks = [Path('/music/other.flac'), Path('/music/match-one.flac'),
                             Path('/music/match-two.flac')]
            window.populate()
            window.show()
            window.activateWindow()
            QTest.qWait(20)
            for key, expected in [(Qt.Key.Key_Down, 1), (Qt.Key.Key_Up, 2)]:
                window.playlist.setCurrentRow(0)
                window.begin_search()
                window.search.setText('match')
                self.assertTrue(window.search.hasFocus())
                QTest.keyClick(window.search, key)
                self.assertFalse(window.search.hasFocus())
                self.assertTrue(window.hasFocus())
                self.assertEqual(window.playlist.currentRow(), expected)
                self.assertEqual(window.search.text(), 'match')
                with patch.object(window, 'play_track') as play:
                    QTest.keyClick(window, Qt.Key.Key_Return)
                    play.assert_called_once_with(expected)
            window.begin_search()
            window.search.setText('no-results')
            QTest.keyClick(window.search, Qt.Key.Key_Down)
            self.assertTrue(window.search.hasFocus())
        finally:
            window.close()

    def test_nested_folder_matches_all_descendant_songs(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            paths = [root / "Artist" / "Album" / "song.flac",
                     root / "Artist" / "Album" / "Disc 2" / "other.mp3",
                     root / "Elsewhere" / "song.flac"]
            window = PlayerWindow([])
            try:
                window.current_folder = root
                window.library_root = root
                window.tracks = paths
                window.populate()
                for query, expected in [("aRtIsT", [True, True, False]),
                                        ("album", [True, True, False]),
                                        ("SONG", [True, False, True]),
                                        ("missing", [False, False, False]),
                                        ("", [True, True, True])]:
                    window.filter_tracks(query)
                    self.assertEqual([not window.playlist.item(i).isHidden()
                                      for i in range(3)], expected)
                self.assertIn("Artist/Album/song", window.playlist.item(0).text())
                self.assertIn("Elsewhere/song", window.playlist.item(2).text())
                window.current_folder = paths[0].parent
                window.update_markers()
                self.assertIn("Artist/Album/song", window.playlist.item(0).text())
                self.assertIn("Elsewhere/song", window.playlist.item(2).text())
            finally:
                window.close()

    def test_default_startup_loads_nested_music(self):
        # Exercise startup through the actual background loader, without entering
        # the application's main event loop or starting audio playback.
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            track = root / "Artist" / "Album" / "Disc 2" / "song.flac"
            track.parent.mkdir(parents=True)
            track.touch()
            window = PlayerWindow([])
            class Instance:
                pass
            try:
                with patch("omamusi.app.Path.cwd", return_value=root), \
                     patch("omamusi.app.PlayerWindow", return_value=window), \
                     patch.object(window, "play_track"), \
                     patch.object(self.app, "exec", return_value=0):
                    cli = parser()
                    run_player(self.app, Instance(), cli, cli.parse_args([]))
                    from PySide6.QtTest import QTest
                    for _ in range(100):
                        if window.tracks:
                            break
                        QTest.qWait(10)
                    self.assertEqual(window.tracks, [track])
            finally:
                window.close()
