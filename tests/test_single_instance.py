"""Exercise repeat launches through a real local socket and separate process."""
import os
os.environ["QT_QPA_PLATFORM"] = "offscreen"

import subprocess
import sys
import tempfile
import time
import unittest
from unittest.mock import patch

from PySide6.QtTest import QTest
from PySide6.QtWidgets import QApplication, QWidget

from omamusi import app as player_app


class SingleInstanceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.app = QApplication.instance() or QApplication([])
        cls.app.setQuitOnLastWindowClosed(False)

    def test_can_launch_again_after_owner_crashes(self):
        with tempfile.TemporaryDirectory() as directory:
            with patch.dict(os.environ, {"XDG_RUNTIME_DIR": directory}):
                child = subprocess.Popen(
                    [sys.executable, "-c",
                     "from PySide6.QtCore import QCoreApplication; "
                     "from omamusi.instance import SingleInstance; "
                     "app = QCoreApplication([]); owner = SingleInstance(app); "
                     "assert owner.start_or_activate(); print('ready', flush=True); "
                     "app.exec()"],
                    stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
                try:
                    self.assertEqual(child.stdout.readline().strip(), "ready")
                finally:
                    child.kill()
                    child.communicate(timeout=5)
                replacement = player_app.SingleInstance(self.app)
                try:
                    self.assertTrue(replacement.start_or_activate())
                finally:
                    replacement.close()

    def test_second_launch_restores_existing_window_before_scanning(self):
        self.assertTrue(hasattr(player_app, "SingleInstance"),
                        "omaMusi needs a single-instance guard")
        with tempfile.TemporaryDirectory() as directory:
            with patch.dict(os.environ, {"XDG_RUNTIME_DIR": directory,
                                         "HYPRLAND_INSTANCE_SIGNATURE": ""}):
                instance = player_app.SingleInstance(self.app)
                self.assertTrue(instance.start_or_activate())
                window = QWidget()
                window.showMinimized()
                instance.window = window
                child = subprocess.Popen(
                    [sys.executable, "-m", "omamusi.app", "/does/not/exist"],
                    stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
                try:
                    deadline = time.monotonic() + 10
                    while child.poll() is None and time.monotonic() < deadline:
                        QTest.qWait(20)
                    self.assertIsNotNone(child.poll(), "repeat launch must exit")
                    stdout, stderr = child.communicate(timeout=1)
                    self.assertEqual(child.returncode, 0, stderr)
                    QTest.qWait(50)
                    self.assertFalse(window.isMinimized())
                    self.assertTrue(window.isVisible())
                finally:
                    if child.poll() is None:
                        child.kill()
                        child.communicate()
                    window.close()
                    instance.close()
                replacement = player_app.SingleInstance(self.app)
                try:
                    self.assertTrue(replacement.start_or_activate())
                finally:
                    replacement.close()
