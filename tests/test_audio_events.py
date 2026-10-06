"""Delayed worker events must belong to the active playback request."""
import os
os.environ["QT_QPA_PLATFORM"] = "offscreen"

from pathlib import Path
import unittest
from unittest.mock import patch

from PySide6.QtWidgets import QApplication

from omamusi.audio import AudioPlayer, _AudioEngine


class AudioEventTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.app = QApplication.instance() or QApplication([])

    def setUp(self):
        self.player = AudioPlayer()
        self.addCleanup(self.player.shutdown)
        self.finished = []
        self.failed = []
        self.player.finished.connect(lambda: self.finished.append(True))
        self.player.failed.connect(self.failed.append)

    def queue_event(self, failure=False):
        # Emit on the actual worker thread, leaving delivery to the UI pending.
        # The helper belongs to the test, not the production engine.
        engine = self.player._engine
        def emit():
            generation = getattr(engine, "generation", None)
            args = () if generation is None else (generation,)
            if failure:
                engine.failed.emit(*args, "old failure")
            else:
                engine.finished.emit(*args)
        engine.test_emit = emit
        self.player._call("test_emit")

    def test_stop_discards_queued_completion_and_failure(self):
        self.queue_event()
        self.queue_event(failure=True)
        self.player.stop(clear=True)
        self.app.processEvents()
        self.assertEqual(self.finished, [])
        self.assertEqual(self.failed, [])

    def test_replay_same_path_discards_old_events(self):
        # A null device avoids depending on the host audio service; real play
        # still publishes state and emits its own current-request failure.
        class NullDevice:
            def isNull(self):
                return True
        with patch("omamusi.audio.QMediaDevices.defaultAudioOutput", return_value=NullDevice()):
            for change in ("play", "seek"):
                with self.subTest(change=change):
                    self.player.play(Path("/tmp/test.wav"), sample_rate=48000)
                    self.app.processEvents()
                    self.finished.clear()
                    self.failed.clear()
                    self.queue_event()
                    self.queue_event(failure=True)
                    if change == "play":
                        self.player.play(Path("/tmp/test.wav"), sample_rate=48000)
                    else:
                        self.player.seek(0)
                    self.app.processEvents()
                    self.assertEqual(self.finished, [])
                    self.assertEqual(self.failed, ["No audio output device is available."])

    def test_volume_change_keeps_current_events(self):
        self.queue_event()
        self.queue_event(failure=True)
        self.player.set_volume(0.2)
        self.app.processEvents()
        self.assertEqual(self.finished, [True])
        self.assertEqual(self.failed, ["old failure"])

    def test_stop_discards_buffered_output_without_waiting_for_playback(self):
        # Model a suspended backend: draining cannot make progress. The
        # driver's reset operation must discard the audio instead.
        class SuspendedSink:
            buffered = b"old audio"
            def processedUSecs(self):
                return 12000
            def stop(self):
                raise AssertionError("Draining a suspended sink can block")
            def reset(self):
                self.buffered = b""
            def deleteLater(self):
                pass
        engine = _AudioEngine()
        sink = SuspendedSink()
        engine.sink = sink
        engine.pending = b"more old audio"
        engine.paused = True
        engine.stop(clear=True)
        self.assertEqual(sink.buffered, b"")
        self.assertEqual(engine.pending, b"")
        self.assertIsNone(engine.sink)
        self.assertFalse(engine.paused)
