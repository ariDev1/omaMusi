"""Bounded FFmpeg decoding and Qt audio output."""

from collections import deque
import json
from pathlib import Path
import queue
import subprocess
import threading

import numpy as np
from PySide6.QtCore import QObject, QThread, QTimer, Qt, Signal, Slot
from PySide6 import QtMultimedia
from PySide6.QtMultimedia import QAudioFormat, QAudioSink, QMediaDevices

# Prefer the canonical enums. PySide lazily exposes legacy aliases, so do
# not evaluate QAudio as a getattr default before looking up QtAudio.
AudioEnums = getattr(QtMultimedia, "QtAudio", None) or QtMultimedia.QAudio


SAMPLE_RATE = 48000
CHANNELS = 2
FRAME_BYTES = CHANNELS * 2
BYTES_PER_SECOND = SAMPLE_RATE * FRAME_BYTES
CHUNK_BYTES = 8192


def probe(path: Path) -> dict:
    try:
        result = subprocess.run(
            ["ffprobe", "-v", "error", "-select_streams", "a:0", "-show_streams",
             "-show_format", "-of", "json", str(path)],
            capture_output=True, text=True, timeout=10, check=True,
        )
        document = json.loads(result.stdout)
        data = document.get("format", {})
        stream = next(iter(document.get("streams", [])), {})
        tags = {k.lower(): v for k, v in data.get("tags", {}).items()}
        return {"duration": float(data.get("duration", 0)),
                "title": tags.get("title", path.stem),
                "artist": tags.get("artist", ""), "album": tags.get("album", ""),
                "sample_rate": int(stream.get("sample_rate", SAMPLE_RATE))}
    except (subprocess.SubprocessError, ValueError, OSError):
        return {"duration": 0, "title": path.stem, "artist": "", "album": "",
                "sample_rate": SAMPLE_RATE}


class Decoder:
    """A producer thread with a bounded queue, so long songs don't fill RAM."""

    def __init__(self, path: Path, offset: float = 0, *, sample_rate=SAMPLE_RATE, floating=False):
        self.chunks = queue.Queue(maxsize=16)
        self.cancelled = threading.Event()
        self.done = threading.Event()
        self.error = ""
        self.process = subprocess.Popen(
            ["ffmpeg", "-nostdin", "-v", "error", "-ss", str(offset),
             "-i", str(path), "-vn", "-f", "f32le" if floating else "s16le",
             "-acodec", "pcm_f32le" if floating else "pcm_s16le",
             "-ar", str(sample_rate), "-ac", str(CHANNELS), "pipe:1"],
            stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        )
        self.thread = threading.Thread(target=self._run, daemon=True)
        self.thread.start()

    def _run(self):
        # Drain stderr concurrently so a malformed file cannot deadlock FFmpeg.
        errors = deque(maxlen=16)

        def read_errors():
            while line := self.process.stderr.readline():
                errors.append(line.decode(errors="replace").strip())

        reader = threading.Thread(target=read_errors, daemon=True)
        reader.start()
        try:
            while not self.cancelled.is_set():
                data = self.process.stdout.read(CHUNK_BYTES)
                if not data:
                    break
                while not self.cancelled.is_set():
                    try:
                        self.chunks.put(data, timeout=0.1)
                        break
                    except queue.Full:
                        pass
            code = self.process.wait()
            reader.join(timeout=1)
            if code and not self.cancelled.is_set():
                self.error = "\n".join(errors) or "FFmpeg could not decode this file."
        finally:
            self.process.stdout.close()
            reader.join(timeout=1)
            self.process.stderr.close()
            self.done.set()

    def close(self):
        self.cancelled.set()
        if self.process.poll() is None:
            try:
                self.process.terminate()
            except ProcessLookupError:
                pass
        self.thread.join(timeout=0.5)
        if self.process.poll() is None:
            try:
                self.process.kill()
            except ProcessLookupError:
                pass
            self.thread.join(timeout=0.5)


class _AudioEngine(QObject):
    """Own the sink and feed timer in a dedicated audio thread."""
    finished = Signal()
    failed = Signal(str)

    def __init__(self, parent=None):
        super().__init__(parent)
        self.sink = None
        self.decoder = None
        self.output = None
        self.pending = b""
        self.path = None
        self.offset = 0.0
        self.paused = False
        self.volume = 0.7
        self.source_rate = self.sample_rate = SAMPLE_RATE
        self.frame_bytes = FRAME_BYTES
        self.dtype = "<i2"
        self.divisor = 32768
        self.written = 0
        self.sink_serial = 0
        self.underruns = 0
        self.was_underrun = False
        self.analysis_buffer = np.zeros(4096, dtype=np.float32)
        self.analysis_revision = 0
        self.mailbox = (0, self.analysis_buffer.copy())
        self.published = {}
        self.timer = QTimer(self)
        self.timer.setTimerType(Qt.TimerType.PreciseTimer)
        self.timer.setInterval(5)
        self.timer.timeout.connect(self._pump)
        self._publish()

    @Slot(str, object)
    def dispatch(self, method, arguments):
        args, kwargs = arguments
        getattr(self, method)(*args, **kwargs)
        self._publish()

    def _publish(self):
        # Publish an immutable snapshot. The UI never calls the live sink.
        self.published = {
            "path": self.path, "offset": self.offset, "position": self.position,
            "paused": self.paused, "volume": self.volume,
            "source_rate": self.source_rate, "sample_rate": self.sample_rate,
            "sink_id": self.sink_serial if self.sink else None,
            "format": QAudioFormat(self.sink.format()) if self.sink else None,
            "written": self.written, "underruns": self.underruns,
        }

    @property
    def position(self):
        return self.offset + (self.sink.processedUSecs() / 1_000_000 if self.sink else 0)

    def play(self, path: Path, offset=0.0, *, sample_rate=None):
        if sample_rate is not None:
            self.source_rate = sample_rate
        elif path != self.path:
            self.source_rate = probe(path)["sample_rate"]
        self.stop()
        self.path = path
        self.offset = offset
        self.underruns = 0
        self.was_underrun = False
        self.analysis_buffer.fill(0)
        self.analysis_revision += 1
        self.mailbox = (self.analysis_revision, self.analysis_buffer.copy())
        device = QMediaDevices.defaultAudioOutput()
        if device.isNull():
            self.failed.emit("No audio output device is available.")
            return
        fmt = QAudioFormat()
        fmt.setChannelCount(CHANNELS)
        supported = False
        for rate in dict.fromkeys((self.source_rate, device.preferredFormat().sampleRate(), SAMPLE_RATE)):
            for sample_format in (QAudioFormat.SampleFormat.Float, QAudioFormat.SampleFormat.Int16):
                fmt.setSampleRate(rate)
                fmt.setSampleFormat(sample_format)
                if device.isFormatSupported(fmt):
                    supported = True
                    break
            if supported:
                break
        if not supported:
            self.failed.emit("The audio output does not support stereo PCM.")
            return
        floating = fmt.sampleFormat() == QAudioFormat.SampleFormat.Float
        self.sample_rate = fmt.sampleRate()
        self.frame_bytes = CHANNELS * (4 if floating else 2)
        self.dtype = "<f4" if floating else "<i2"
        self.divisor = 1 if floating else 32768
        try:
            self.decoder = Decoder(path, offset, sample_rate=self.sample_rate, floating=floating)
        except OSError as error:
            self.failed.emit(str(error))
            return
        self.sink = QAudioSink(device, fmt, self)
        self.sink_serial += 1
        # Keep enough time buffered even for 96/192 kHz floating-point audio.
        self.sink.setBufferSize(max(CHUNK_BYTES * 2, int(self.sample_rate * self.frame_bytes * 0.12)))
        self.sink.setVolume(self.volume)
        self.output = self.sink.start()
        if self.output is None:
            self.stop()
            self.failed.emit("Could not open the audio output.")
            return
        self.timer.start()

    def stop(self, clear=False):
        self.timer.stop()
        if self.sink:
            self.offset = self.position
            self.sink.stop()
            self.sink.deleteLater()
        if self.decoder:
            self.decoder.close()
        self.sink = self.decoder = self.output = None
        self.pending = b""
        self.written = 0
        self.paused = False
        if clear:
            self.path = None
            self.offset = 0

    def toggle_pause(self):
        if self.sink:
            self.paused = not self.paused
            if self.paused:
                self.sink.suspend()
            else:
                self.sink.resume()

    def seek(self, seconds):
        if self.path:
            paused = self.paused
            self.play(self.path, max(0, seconds))
            if paused:
                self.toggle_pause()

    def set_volume(self, value):
        self.volume = max(0, min(1, value))
        if self.sink:
            self.sink.setVolume(self.volume)

    def _pump(self):
        if not self.sink or self.paused:
            return
        if self.sink.error() not in (AudioEnums.Error.NoError, AudioEnums.Error.UnderrunError):
            self.stop()
            self._publish()
            self.failed.emit("Audio output failed. Check your output device.")
            return
        capacity = self.sink.bytesFree()
        idle = self.sink.state() == AudioEnums.State.IdleState
        underrun = idle and self.written > 0 and not self.decoder.done.is_set()
        if underrun and not self.was_underrun:
            self.underruns += 1
        self.was_underrun = underrun
        while capacity >= self.frame_bytes:
            if not self.pending:
                try:
                    self.pending = self.decoder.chunks.get_nowait()
                except queue.Empty:
                    break
            count = min(capacity, len(self.pending)) // self.frame_bytes * self.frame_bytes
            if not count:
                break
            data = self.pending[:count]
            accepted = self.output.write(data)
            if accepted <= 0:
                break
            self.pending = self.pending[accepted:]
            self.written += accepted
            pcm = np.frombuffer(data[:accepted], dtype=self.dtype).reshape(-1, CHANNELS)
            mono = pcm.astype(np.float32).mean(axis=1) / self.divisor
            n = min(len(mono), len(self.analysis_buffer))
            self.analysis_buffer = np.roll(self.analysis_buffer, -n)
            self.analysis_buffer[-n:] = mono[-n:]
            self.analysis_revision += 1
            capacity -= accepted
        # Latest-frame mailbox, not an ever-growing queue of visual updates.
        if self.mailbox[0] != self.analysis_revision:
            self.mailbox = (self.analysis_revision, self.analysis_buffer.copy())
        self._publish()
        if self.decoder.done.is_set() and self.decoder.chunks.empty() and not self.pending:
            # Wait until the sink has played the last buffered samples.
            if self.sink.state() == AudioEnums.State.IdleState:
                error = self.decoder.error
                self.stop()
                self._publish()
                if error:
                    self.failed.emit(error)
                else:
                    self.finished.emit()


class _SinkView:
    """UI-side audio state, with no cross-thread access to QAudioSink."""

    def __init__(self, state):
        self.token = state["sink_id"]
        self.update(state)

    def update(self, state):
        self._format = state["format"]
        self._volume = state["volume"]

    def format(self):
        return QAudioFormat(self._format)

    def volume(self):
        return self._volume


class AudioPlayer(QObject):
    """Player controls on the UI thread; all audio feeding stays independent."""

    samples = Signal(object)
    positionChanged = Signal(float)
    finished = Signal()
    failed = Signal(str)
    _command = Signal(str, object)

    def __init__(self, parent=None):
        super().__init__(parent)
        self._audio_thread = QThread(self)
        self._audio_thread.setObjectName("omaMusi audio")
        self._engine = _AudioEngine()
        self._engine.moveToThread(self._audio_thread)
        self._audio_thread.finished.connect(self._engine.deleteLater)
        self._command.connect(self._engine.dispatch, Qt.ConnectionType.BlockingQueuedConnection)
        self._engine.finished.connect(self._finished)
        self._engine.failed.connect(self._failed)
        self.sink = None
        self._position = 0
        self._revision = 0
        self._closed = False
        self._audio_thread.start(QThread.Priority.HighPriority)
        self._sync()
        self.timer = QTimer(self)
        self.timer.setInterval(33)
        self.timer.timeout.connect(self._poll)
        self.timer.start()

    @property
    def position(self):
        return self._position

    @property
    def worker_snapshot(self):
        return dict(self._engine.published)

    def _sync(self):
        state = self._engine.published
        self.path, self.offset = state["path"], state["offset"]
        self.paused, self.volume = state["paused"], state["volume"]
        self.source_rate, self.sample_rate = state["source_rate"], state["sample_rate"]
        self._position = state["position"]
        if state["sink_id"] is None:
            self.sink = None
        elif self.sink is None or self.sink.token != state["sink_id"]:
            self.sink = _SinkView(state)
        else:
            self.sink.update(state)

    def _call(self, method, *args, **kwargs):
        if not self._closed:
            self._command.emit(method, (args, kwargs))
            self._sync()

    def _poll(self):
        self._sync()
        revision, frame = self._engine.mailbox
        if revision != self._revision:
            self._revision = revision
            self.samples.emit(frame)
        self.positionChanged.emit(self.position)

    @Slot()
    def _finished(self):
        if self._closed:
            return
        self._sync()
        self.finished.emit()

    @Slot(str)
    def _failed(self, message):
        if self._closed:
            return
        self._sync()
        self.failed.emit(message)

    def play(self, path, offset=0.0, *, sample_rate=None):
        if sample_rate is None:
            # Probing must also never block the audio thread's feed timer.
            sample_rate = self.source_rate if path == self.path else probe(path)["sample_rate"]
        self._call("play", path, offset, sample_rate=sample_rate)

    def stop(self, clear=False):
        self._call("stop", clear=clear)

    def toggle_pause(self):
        self._call("toggle_pause")

    def seek(self, seconds):
        self._call("seek", seconds)

    def set_volume(self, value):
        self._call("set_volume", value)

    def shutdown(self):
        if self._closed:
            return
        self.timer.stop()
        self.stop()
        self._closed = True
        self._audio_thread.quit()
        self._audio_thread.wait()
