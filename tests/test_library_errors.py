"""Discovery must report filesystem failures rather than return partial queues."""
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from omamusi.library import discover


class LibraryErrorTests(unittest.TestCase):
    def test_permission_failure_is_not_misreported_as_missing(self):
        with patch.object(Path, "stat", side_effect=PermissionError("access denied")):
            with self.assertRaises(PermissionError):
                discover(["/unreadable/song.wav"])

    def test_recursive_unreadable_directory_does_not_return_partial_queue(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "good.wav").touch()
            blocked = root / "blocked"
            blocked.mkdir()
            scan = os.scandir
            def guarded(path):
                if Path(path) == blocked:
                    raise PermissionError("cannot read subfolder")
                return scan(path)
            with patch("os.scandir", side_effect=guarded):
                with self.assertRaises(PermissionError):
                    discover([str(root)], recursive=True)

    def test_explicit_special_file_is_rejected(self):
        with tempfile.TemporaryDirectory() as temporary:
            fifo = Path(temporary) / "audio.wav"
            os.mkfifo(fifo)
            with self.assertRaises(ValueError):
                discover([str(fifo)])
