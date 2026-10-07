"""Application menu installation and launch without touching the real desktop."""
import os
from pathlib import Path
import shlex
import sys
import tempfile
import unittest
from unittest.mock import patch

from omamusi import desktop


class DesktopTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='oma menu ')
        self.addCleanup(self.temp.cleanup)
        self.home = Path(self.temp.name)
        self.data = self.home / 'user data'
        env = patch.dict(os.environ, HOME=str(self.home), XDG_DATA_HOME=str(self.data))
        env.start()
        self.addCleanup(env.stop)
        self.entry = self.data / 'applications/io.github.aridev1.omamusi.desktop'
        self.icon = self.data / 'icons/hicolor/scalable/apps/io.github.aridev1.omamusi.svg'

    def test_install_and_remove_preserve_other_applications_and_music(self):
        self.entry.parent.mkdir(parents=True)
        other = self.entry.parent / 'other.desktop'
        other.write_text('keep')
        music = self.home / 'Music/song.flac'
        music.parent.mkdir()
        music.touch()
        desktop.install()
        desktop.install()
        text = self.entry.read_text()
        self.assertIn('Name=omaMusi\n', text)
        self.assertIn('Terminal=false\n', text)
        self.assertIn('Categories=AudioVideo;Audio;Player;\n', text)
        self.assertIn('Icon=io.github.aridev1.omamusi\n', text)
        self.assertTrue(self.icon.is_file())
        desktop.remove()
        desktop.remove()
        self.assertFalse(self.entry.exists())
        self.assertFalse(self.icon.exists())
        self.assertEqual(other.read_text(), 'keep')
        self.assertTrue(music.is_file())

    def test_refuses_to_overwrite_unrelated_desktop_entry(self):
        self.entry.parent.mkdir(parents=True)
        self.entry.write_text('[Desktop Entry]\nName=Other\n')
        with self.assertRaises(ValueError):
            desktop.install()
        self.assertEqual(self.entry.read_text(), '[Desktop Entry]\nName=Other\n')
        desktop.remove()
        self.assertTrue(self.entry.exists())

    def test_removal_does_not_remove_another_installations_launcher(self):
        desktop.install()
        with patch.object(sys, 'executable', '/other environment/bin/python'):
            desktop.remove()
        self.assertTrue(self.entry.exists())
        self.assertTrue(self.icon.exists())

    def test_launch_opens_localized_music_folder_recursively(self):
        music = self.home / 'My Music'
        music.mkdir()
        with patch('PySide6.QtCore.QStandardPaths.writableLocation', return_value=str(music)), \
             patch('omamusi.app.main', return_value=0) as player:
            self.assertEqual(desktop.launch(), 0)
        self.assertEqual(player.call_args.args[0],
                         ['--view', 'event horizon', str(music), '--recursive'])

    def test_missing_music_folder_launches_empty_without_scanning_home(self):
        with patch('PySide6.QtCore.QStandardPaths.writableLocation', return_value=str(self.home)), \
             patch('omamusi.app.main', return_value=0) as player:
            def verify_empty(args):
                self.assertTrue(Path(args[2]).is_dir())
                self.assertEqual(list(Path(args[2]).iterdir()), [])
                self.assertNotEqual(Path(args[2]), self.home)
                return 0
            player.side_effect = verify_empty
            self.assertEqual(desktop.launch(), 0)
        self.assertFalse(Path(player.call_args.args[0][2]).exists())

    def test_installed_command_handles_spaces_and_percent_in_interpreter_path(self):
        with patch.object(sys, 'executable', str(self.home / '50% player/bin/python')):
            desktop.install()
        command = next(line[5:] for line in self.entry.read_text().splitlines()
                       if line.startswith('Exec='))
        # The desktop launcher expands %% after unquoting the command.
        self.assertEqual(shlex.split(command.replace('%%', '%')),
                         [str(self.home / '50% player/bin/python'), '-m', 'omamusi.desktop', 'launch'])

    def test_relative_xdg_data_home_uses_standard_user_directory(self):
        with patch.dict(os.environ, XDG_DATA_HOME='relative/data'):
            desktop.install()
        self.assertTrue((self.home / '.local/share/applications/io.github.aridev1.omamusi.desktop').is_file())
