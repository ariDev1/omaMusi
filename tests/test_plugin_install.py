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
        for name in ("ffmpeg", "ffprobe", "xdg-user-dir"):
            self.executable(self.fakebin / name, "#!/bin/sh\nexit 0\n")
        self.executable(self.fakebin / "python", '''#!/usr/bin/python3
import pathlib, sys
if sys.argv[1:] == ['--version']:
    print('Python 3.13.0')
elif sys.argv[1:3] == ['-m', 'venv']:
    target = pathlib.Path(sys.argv[3]) / 'bin'
    target.mkdir(parents=True, exist_ok=True)
    (target / 'python').write_text('#!/bin/sh\\nexit 0\\n')
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

    def test_setup_refuses_unrelated_launcher(self):
        launcher = self.home / ".local/bin/omaMusi"
        launcher.parent.mkdir(parents=True)
        launcher.write_text("unrelated")
        result = self.run_script("setup-player.sh")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("already exists", result.stderr)
        self.assertEqual(launcher.read_text(), "unrelated")

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
