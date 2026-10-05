"""Import audio in a fresh process, before Qt's lazy enum aliases are loaded."""
import subprocess
import sys
import unittest


class AudioCompatibilityTests(unittest.TestCase):
    def test_audio_imports_without_preloading_multimedia_enum_aliases(self):
        result = subprocess.run(
            [sys.executable, "-c",
             "from omamusi.audio import AudioEnums; "
             "assert AudioEnums.State.IdleState is not None; "
             "assert AudioEnums.Error.NoError is not None"],
            capture_output=True, text=True, timeout=10)
        self.assertEqual(result.returncode, 0, result.stderr)
