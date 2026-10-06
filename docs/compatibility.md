# Compatibility validation for 0.10.0

omaMusi uses one Python/PySide6 application on Linux. The Omarchy plugin is a launcher for that application; GNOME uses its native window decorations.

## Requirements

- Python 3.11 or newer.
- PySide6/Qt 6.7 or newer, below 7.
- NumPy 1.26 or newer, below 3; PyOpenGL 3.1.7 or newer, below 4.
- FFmpeg and ffprobe, plus a desktop audio service.
- OpenGL 3.3 or newer for hardware visuals; a CPU fallback is available.
- Setuptools 77.0.3 or newer when building the package.

## Validation on this machine

| Environment | Result |
| --- | --- |
| Python 3.14.7, PySide6/Qt 6.11.2 | All 140 development unit/integration tests pass |
| Python 3.11.17, PySide6/Qt 6.7.0, NumPy 1.26.4, PyOpenGL 3.1.7 | All 140 development unit/integration tests pass |
| Setuptools 77.0.3 | Wheel builds with version 0.10.0, MIT license expression, license file, and all new modules |
| Hyprland, AMD Radeon GPU, Qt 6.11.2 and Qt 6.7.0 | GPU smoke tests pass on both runtimes, including waveform alpha, color accumulation, 56 view switches, and uninterrupted audio |
| GNOME configuration, offscreen renderer | Native decoration flags retained; Waveform has transparent background pixels and other visuals are opaque |

The isolated Qt 6.7 environment caught and verified the fix for a lazy audio-enum alias import failure. Current builds prefer `QtAudio` and only look up the legacy `QAudio` fallback when needed.

## GNOME desktop validation still required

GNOME Shell is not installed on the development machine. Offscreen checks verify application configuration and rendering, but do not verify GNOME's compositor or focus policy. Run these checks on a real GNOME session before describing 0.10.0 as fully validated there:

1. Start omaMusi and confirm native title-bar controls, moving, resizing, and fullscreen work.
2. Select Waveform with **V**. Confirm the desktop is visible through empty areas and text remains visible. Cycle through the other visuals and confirm their backgrounds are opaque.
3. Minimize omaMusi, then launch it again using its launcher. Confirm the existing window is restored and receives focus without restarting playback or creating another player. Repeat while it is on another workspace.
4. Use **A** to create and add to a playlist; use **B** to browse, rename, remove tracks, and load it. Confirm focus returns to the player when the dialog closes and text-entry shortcuts behave normally.
5. Close and reopen the player; confirm saved playlists persist and **R** still toggles random playback.

For the automated desktop GPU/audio check, run:

```bash
.venv/bin/python tests/smoke_gpu.py
```

The full suite runs GUI tests offscreen but needs access to the desktop audio service:

```bash
.venv/bin/python -m unittest discover -s tests
```

## Development hardening checks

The development branch also covers slow metadata reads, slow or unreadable folders, missing tracks, cancellation, shutdown during a pending read, repeated additions with different recursion settings, and pause requests during metadata loading. Worker queues retain at most one running and one pending operation per category. Cancellation discards pending results; it cannot interrupt a filesystem call already blocked in the operating system.

Audio teardown uses `QAudioSink.reset()` to discard buffered samples when seeking, switching tracks, or closing. Qt documents that `stop()` drains buffers synchronously on Linux; this produced a long stall when seeking a suspended Qt 6.7 sink during compatibility testing. See [Qt audio sink buffer behavior](https://doc.qt.io/qt-6/qaudiosink.html#stop).

The regression workflow in `.github/workflows/tests.yml` runs the complete suite and builds a wheel under Python 3.11 / Qt 6.7.0 / NumPy 1.26.4 / PyOpenGL 3.1.7, and Python 3.14 with current dependency versions. Its virtual PulseAudio output exercises playback without desktop speakers. A hosted CI pass does not replace the real GPU/audio smoke test or the GNOME checklist above.
