"""Exercise user-local installation and launch boundaries without downloading packages."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[1]


class PluginInstallTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="oma plugin ")
        self.addCleanup(self.temp.cleanup)
        self.home = Path(self.temp.name)
        self.env = dict(os.environ, HOME=str(self.home),
                        XDG_CONFIG_HOME=str(self.home / "config folder"),
                        XDG_DATA_HOME=str(self.home / "data folder"),
                        XDG_CACHE_HOME=str(self.home / "cache folder"))
        self.fakebin = self.home / "fakebin"
        self.fakebin.mkdir()
        self.env["PATH"] = f"{self.fakebin}:/usr/bin:/bin"
        self.env["LAUNCH_LOG"] = str(self.home / "launch.json")
        self.env["PYTHONPATH"] = str(ROOT)
        for name in ("ffmpeg", "ffprobe", "xdg-user-dir"):
            self.executable(self.fakebin / name, "#!/bin/sh\nexit 0\n")
        self.executable(self.fakebin / "python", '''#!/usr/bin/python3
import pathlib, sys
if sys.argv[1:] == ['--version']:
    print('Python 3.13.0')
elif sys.argv[1:3] == ['-m', 'venv']:
    target = pathlib.Path(sys.argv[3]) / 'bin'
    target.mkdir(parents=True, exist_ok=True)
    (target / 'python').write_text('#!/usr/bin/python3\\nimport runpy, sys\\nif sys.argv[1:3] == ["-m", "omamusi.desktop"]:\\n    sys.executable = sys.argv[0]\\n    sys.argv = sys.argv[2:]\\n    runpy.run_module("omamusi.desktop", run_name="__main__")\\n')
    (target / 'python').chmod(0o755)
    (target / 'omaMusi').write_text("#!/usr/bin/python3\\nimport json, os, sys, pathlib\\nassert pathlib.Path(sys.argv[3]).is_dir()\\nassert not list(pathlib.Path(sys.argv[3]).iterdir())\\nopen(os.environ['LAUNCH_LOG'], 'w').write(json.dumps(sys.argv[1:]))\\n")
    (target / 'omaMusi').chmod(0o755)
else:
    raise SystemExit(1)
''')

    def executable(self, path, content):
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content)
        path.chmod(0o755)

    def run_script(self, name):
        return subprocess.run(["/usr/bin/bash", str(ROOT / "scripts" / name)],
                              env=self.env, capture_output=True, text=True)

    def test_setup_repeated_and_remove_preserve_unrelated_files(self):
        self.assertEqual(self.run_script("setup-player.sh").returncode, 0)
        self.assertEqual(self.run_script("setup-player.sh").returncode, 0)
        launcher = self.home / ".local/bin/omaMusi"
        self.assertTrue(launcher.is_symlink())
        unrelated = self.home / ".local/bin/other"
        unrelated.write_text("keep")
        self.assertEqual(self.run_script("remove-player.sh").returncode, 0)
        self.assertFalse(launcher.exists())
        self.assertEqual(unrelated.read_text(), "keep")
        self.assertEqual(self.run_script("remove-player.sh").returncode, 0)

    def test_setup_installs_menu_entry_and_removal_cleans_it(self):
        result = self.run_script("setup-player.sh")
        self.assertEqual(result.returncode, 0, result.stderr)
        entry = self.home / 'data folder/applications/io.github.aridev1.omamusi.desktop'
        icon = self.home / 'data folder/icons/hicolor/scalable/apps/io.github.aridev1.omamusi.svg'
        self.assertTrue(entry.is_file())
        self.assertIn(str(self.home / 'data folder/omamusi/venv/bin/python'), entry.read_text())
        self.assertTrue(icon.is_file())
        result = self.run_script('remove-player.sh')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertFalse(entry.exists())
        self.assertFalse(icon.exists())

    def test_removal_accepts_older_player_without_desktop_module(self):
        result = self.run_script('setup-player.sh')
        self.assertEqual(result.returncode, 0, result.stderr)
        # Older installations have neither the menu module nor a menu entry.
        entry = self.home / 'data folder/applications/io.github.aridev1.omamusi.desktop'
        entry.unlink()
        self.executable(self.home / 'data folder/omamusi/venv/bin/python',
                        '#!/bin/sh\nexit 1\n')
        result = self.run_script('remove-player.sh')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertFalse((self.home / 'data folder/omamusi/venv').exists())

    def test_setup_refuses_unrelated_launcher(self):
        launcher = self.home / ".local/bin/omaMusi"
        launcher.parent.mkdir(parents=True)
        launcher.write_text("unrelated")
        result = self.run_script("setup-player.sh")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("already exists", result.stderr)
        self.assertEqual(launcher.read_text(), "unrelated")

    def test_remove_preserves_saved_playlists_and_other_user_data(self):
        self.assertEqual(self.run_script("setup-player.sh").returncode, 0)
        install = self.home / "data folder/omamusi"
        saved = install / "playlists.json"
        saved.write_text('{"version": 1, "playlists": {"Favorites": []}}')
        (install / "notes.txt").write_text("keep this too")
        result = self.run_script("remove-player.sh")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(saved.read_text())["playlists"], {"Favorites": []})
        self.assertEqual((install / "notes.txt").read_text(), "keep this too")
        self.assertFalse((install / "venv").exists())
        result = self.run_script("setup-player.sh")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(saved.read_text())["playlists"], {"Favorites": []})
        self.assertEqual((install / "notes.txt").read_text(), "keep this too")

    def test_setup_accepts_existing_standalone_playlists(self):
        install = self.home / "data folder/omamusi"
        install.mkdir(parents=True)
        saved = install / "playlists.json"
        saved.write_text('{"version": 1, "playlists": {"Favorites": []}}')
        result = self.run_script("setup-player.sh")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(saved.read_text())["playlists"], {"Favorites": []})

    def test_setup_refuses_unmarked_virtual_environment(self):
        install = self.home / "data folder/omamusi"
        (install / "venv").mkdir(parents=True)
        (install / "venv/keep").write_text("unmanaged")
        self.assertNotEqual(self.run_script("setup-player.sh").returncode, 0)
        self.assertEqual((install / "venv/keep").read_text(), "unmanaged")

    def test_launch_preserves_music_path_with_spaces(self):
        self.assertEqual(self.run_script("setup-player.sh").returncode, 0)
        (self.home / "Music").mkdir()
        self.assertEqual(self.run_script("launch-player.sh").returncode, 0)
        args = json.loads(Path(self.env["LAUNCH_LOG"]).read_text())
        self.assertEqual(args, ["--view", "event horizon", str(self.home / "Music"), "--recursive"])

    def test_launch_without_music_uses_empty_directory(self):
        self.assertEqual(self.run_script("setup-player.sh").returncode, 0)
        self.assertEqual(self.run_script("launch-player.sh").returncode, 0)
        args = json.loads(Path(self.env["LAUNCH_LOG"]).read_text())
        self.assertEqual(args[:2], ["--view", "event horizon"])
        self.assertFalse(Path(args[2]).exists(), "Temporary empty folder must be cleaned up")
        self.assertNotEqual(Path(args[2]), ROOT)

    def test_missing_player_reports_setup(self):
        self.executable(self.fakebin / "notify-send", "#!/bin/sh\nexit 0\n")
        result = self.run_script("launch-player.sh")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("setup-player.sh", result.stderr)

    def test_removal_refuses_unmanaged_installation(self):
        install = self.home / "data folder/omamusi"
        install.mkdir(parents=True)
        (install / "keep").write_text("unrelated")
        self.assertNotEqual(self.run_script("remove-player.sh").returncode, 0)
        self.assertTrue((install / "keep").exists())

    def test_setup_reports_missing_ffprobe_before_installing(self):
        os.symlink("/usr/bin/dirname", self.fakebin / "dirname")
        (self.fakebin / "ffprobe").unlink()
        self.env["PATH"] = str(self.fakebin)
        result = self.run_script("setup-player.sh")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Missing ffprobe", result.stderr)
        self.assertFalse((self.home / "data folder/omamusi").exists())

    def test_remove_cleans_matching_dangling_launcher(self):
        launcher = self.home / ".local/bin/omaMusi"
        launcher.parent.mkdir(parents=True)
        launcher.symlink_to(self.home / "data folder/omamusi/venv/bin/omaMusi")
        self.assertEqual(self.run_script("remove-player.sh").returncode, 0)
        self.assertFalse(launcher.is_symlink())
