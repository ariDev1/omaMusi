# omaMusi

A quiet local music player with **GPU-rendered, music-reactive visuals**. Launch it from the folder containing your music; playback opens in its own desktop window.

Less text, more music. A single song gets one title and a small time display. Short playlists take only the space they need; long playlists fill the available height above the playback text, without an eight-row cap. There are no visible buttons, panels, borders, sliders, or scrollbars. Volume appears briefly when you adjust it. Colors and monospace text follow your current Omarchy theme and terminal font.

```bash
cd ~/Music
omaMusi -all
```

## Features

- A GPU warp tunnel: 4,096 colored star-streams — teal, amber, green, and magenta drawn from your Omarchy theme — rushing toward you through swirling tunnel walls into a dark, subtly breathing vanishing point. The tunnel flies an extreme rollercoaster path — the vanishing point whips along wide two-frequency curves while the whole view banks hard into every turn, distant streaks swing wide through the bends, and dive surges stretch and zoom the field with the music. Each stream's speed, width, brightness, and wobble follows its own slice of the live spectrum and waveform, so the whole field ripples with the music. Also called with the legacy name `--view sprites`.
- Spectrum, waveform, and scrolling spectrogram views. The spectrogram fills the entire window behind the text. Press **v** to cycle through all four visuals; **Shift+V** cycles backward. The selected view's name appears briefly.
- Text-only playlists sized to show as many files as the window allows; select with the arrow keys and Enter, or click a file.
- Simple playback, track selection, seeking, and volume.
- Automatic advancement through the playlist; playback starts immediately.
- Metadata titles, artist, and album for the currently playing track.
- Press `o` to open more files, or drag files/folders onto the window.
- Press `c` to browse folders and their audio files inside the player using a text-only list.
- FFmpeg format support, including MP3, FLAC, WAV, Ogg, Opus, M4A, and AAC.
- Bounded streaming decode rather than loading entire tracks into memory.

### Rendering and playback isolation

All four visualizations use OpenGL 3.3 shaders on a hardware GPU by default. Warp streak positions, perspective, tunnel walls, and glow are computed on the GPU; the whole warp field is drawn in one instanced draw call. Spectrum, waveform, and spectrogram rendering also use GPU shaders. Lightweight audio analysis remains on the CPU.

If hardware OpenGL is unavailable, or shader initialization/rendering fails, the player falls back to a smaller CPU-rendered view at 30 fps and reports the reason in the launching terminal. Software OpenGL implementations such as llvmpipe are treated as CPU fallback, not hardware GPU rendering. Headless tests use the CPU fallback.

Audio decoding uses a bounded producer queue. The audio sink and its feed timer run in a **dedicated Qt audio thread**, separate from rendering and UI timers. Visualization samples are delivered through a bounded latest-frame mailbox, so a stalled UI does not build up a queue of analysis work. Switching views reuses the same GPU resources and fixed star seeds; it never restarts audio or rebuilds the tunnel.

### Audio quality

Playback prefers the track's native sample rate and 32-bit floating-point stereo output. It avoids a fixed 48 kHz / 16-bit conversion when the audio device supports the source format. If needed, it negotiates a supported rate/format with the output device. FFmpeg performs decoding; there is no equalizer, normalization, compressor, or audio effect. The animation only analyzes copies of the samples and never alters the audio. System output processing and volume still belong to PipeWire and Omarchy.

## Installation

Requires **Python 3.11+**, **FFmpeg** (`ffmpeg` and `ffprobe`), and a working desktop audio output. The Python dependencies are PySide6, NumPy, and PyOpenGL. Hardware GPU rendering requires desktop OpenGL 3.3 or newer.

From this project folder:

```bash
python -m venv .venv
.venv/bin/python -m pip install -e .
mkdir -p ~/.local/bin
ln -s "$PWD/.venv/bin/omaMusi" ~/.local/bin/omaMusi
```

Ensure `~/.local/bin` is on your `PATH`. You can also launch directly with `.venv/bin/omaMusi`.

## Usage

```bash
omaMusi -all                       # All audio files in the current folder
omaMusi                           # Same default; opens an empty player if no music is found
omaMusi ~/Music                   # A particular folder
omaMusi -all --recursive           # Include subfolders
omaMusi song.flac another.mp3      # Specific files, in the supplied order
omaMusi -all --view spectrum       # Start with a classic spectrum
omaMusi -all --view warp         # Start with the warp tunnel (default)
omaMusi -all --view waveform       # Start with the waveform
omaMusi -all --view spectrogram    # Start with the scrolling spectrogram
omaMusi --help
```

Directory scans use natural filename ordering (`2` before `10`). Scans are nonrecursive unless you add `--recursive`; duplicate files are omitted. If paths accompany `-all`, the current folder is included first, followed by those paths. Dropped folders add their directly contained audio files.

### Keyboard controls

| Key | Action |
| --- | --- |
| Space | Play / pause |
| Down / Up, or J / K | Select next / previous list item |
| Enter | Play the selected file, or open the selected folder while browsing |
| N / P | Next / previous (previous restarts the song after 3 seconds) |
| Left / Right | Seek backward / forward 5 seconds; while browsing, go to parent / open selected folder |
| + / − (or =) | Player volume ±5%; shifted `+` and numeric keypad keys work |
| V / Shift+V | Next / previous visualization |
| Tab | Hide / show playlist |
| F | Toggle fullscreen |
| Escape | Cancel the folder prompt, leave the filter, or leave fullscreen |
| / | Open the text filter |
| C | Browse music folders (both `c` and Shift+C work) |
| Backspace | Go to the parent folder while browsing |
| L | Play the browsed folder and replace the playlist |
| Ctrl+L | Type a folder path directly |
| O / Ctrl+O | Add audio files (opens a system file chooser) |
| Q | Quit |

When the filter field is focused, typing edits the filter. Press Escape to return to player shortcuts.

### Omarchy integration

All player commands are local to the focused player window. The player does not install or change Hyprland keybindings. Your media-volume keys and Omarchy volume shortcuts continue to control the system output. `+`, `−`, and `=` adjust only omaMusi's audio stream; the displayed `vol` value is the player volume, independent of the system volume or system mute state. While entering a filter or folder path, these keys type text normally.

### Changing folders

Press **c** to open the directory browser inside the player. Use **Up/Down** (or **j/k**) to select a folder and **Right** or **Enter** to enter it. **Left**, **Backspace**, or the `../` entry takes you to the parent folder. When you reach your music folder, choose `./ play this folder` and press **Enter**, or press **l**. **Escape** returns to the playlist. Browsing leaves the current music playing until you load a folder.

Each directory shows its subfolders followed by its audio files, in natural filename order. Select an audio file and press **Enter** (or **Right**) to load that folder's playlist and start with the selected song.

You can also press **Ctrl+L** to type a path directly. Paths can be absolute (`/home/rene/Music`), home-relative (`~/Music`), or relative to the folder displayed in the browser (`../another-album`). Spaces in folder names work without quotes.

The new folder replaces the playlist and starts its first track. Folder scans include directly contained audio files. An empty folder shows an empty playlist; invalid paths leave the existing playlist and playback intact. Press **Escape** to cancel the prompt.

## Verification

```bash
.venv/bin/python -m unittest discover -s tests -v
```

Tests cover discovery, real FFmpeg decoding/seeking/cancellation, native-rate floating-point playback, pause/resume, automatic track advancement, folder navigation and song selection, volume-key isolation, compact layout, visualization cycling, and rendering of all four audio-reactive views. GUI tests use offscreen rendering and your audio output at zero volume; they need an available audio device.

For the real desktop GPU path:

```bash
.venv/bin/python tests/smoke_gpu.py
```

This generates a muted test tone, validates all four hardware GPU views, cycles visuals 32 times, blocks UI processing for 650 ms while checking audio progress and underruns, and compares frozen warp frames before/after cycling to verify tunnel continuity. It requires a desktop session with a hardware OpenGL GPU.
