# omaMusi

Current version: **v0.9.1**.

Licensed under the [MIT license](LICENSE).

**Your music. A little text. A whole universe of visuals.**

omaMusi is a local music player for Linux with animated visuals that react to your music. Open it in your music folder, choose a song, and enjoy. The interface stays out of the way: just your playlist, track information, and playback time.

![omaMusi playing music with the Event Horizon visualization](event-horizon.png)

## What it does

- Plays your local music, including MP3, FLAC, WAV, Ogg, Opus, M4A, and AAC.
- Shows song titles, artists, and albums, and advances to the next track automatically.
- Lets you browse folders, filter your playlist, or drag music into the window.
- Follows your Omarchy theme and terminal font.
- Runs music-reactive visuals on your GPU, with a simpler fallback when hardware rendering is unavailable.

The visuals analyze the music without changing its sound.

The subtle footer shows the version and short Git commit hash. A `+dirty` suffix means the checkout has uncommitted changes; the hash reads `unknown` when Git information is unavailable. You can also check the version with `omaMusi --version`.

Tested on Omarchy (Hyprland) and GNOME. Omarchy keeps the frameless window; GNOME uses native window decorations.

## Seven visuals

- **Event Horizon** — a cinematic black hole with a glowing disk, bass-driven waves, orbiting hot spots, and delayed light echoes.
- **Particle Dance** — thousands of colorful particles dancing to the music, with a faster torus drifting through the swarm.
- **Phi Cathedral** — evolving golden-ratio patterns and spirals.
- **Warp** — a swirling flight through colored star streams.
- **Spectrum**, **Waveform**, and **Spectrogram** — three classic ways to see your music.

Press **V** to switch visuals, or **Shift+V** to go back.

## Install

You need **Python 3.11+**, **FFmpeg** (including `ffprobe`), and a desktop audio output. The full visuals use **OpenGL 3.3+**.

From this project folder:

```bash
python -m venv .venv
.venv/bin/python -m pip install -e .
mkdir -p ~/.local/bin
ln -s "$PWD/.venv/bin/omaMusi" ~/.local/bin/omaMusi
```

Make sure `~/.local/bin` is on your `PATH`. You can also run `.venv/bin/omaMusi` directly.

## Omarchy plugin

The optional Omarchy Quattro bar widget uses an Event Horizon logo. Click it to open omaMusi with Event Horizon selected. It opens your Music folder recursively, or an empty playlist if that folder is missing. The player continues to work as a standalone application on GNOME and other desktops.

Once this plugin branch has been published and merged, install from the repository:

```bash
omarchy plugin add https://github.com/ariDev1/omaMusi.git --enable
bash ~/.config/omarchy/plugins/io.github.aridev1.omamusi/scripts/setup-player.sh
```

Installing or enabling the widget does not install the player automatically. The setup script creates a dedicated Python virtual environment in `${XDG_DATA_HOME:-~/.local/share}/omamusi` and a launcher at `~/.local/bin/omaMusi`. It downloads Python packages and build dependencies using pip and installs the local plugin checkout. It needs Python 3.11+, FFmpeg/ffprobe, network access for those downloads, and desktop audio. On Omarchy, install missing system dependencies with `omarchy pkg add python ffmpeg`. The Python dependencies are PySide6, NumPy, and PyOpenGL; see [pyproject.toml](pyproject.toml) for supported versions. Full visuals require OpenGL 3.3+.

If `~/.local/bin/omaMusi` already belongs to a development install, the widget can use it. Setup refuses to replace that launcher. To use the managed installation instead, move the old launcher yourself before running setup. The scripts do not change your desktop configuration or music files.

Move the widget with:

```bash
omarchy bar move io.github.aridev1.omamusi --section right
```

After updating the plugin, close the player and rerun setup to update its installed copy:

```bash
omarchy plugin update io.github.aridev1.omamusi
bash ~/.config/omarchy/plugins/io.github.aridev1.omamusi/scripts/setup-player.sh
```

Remove the managed player before removing the plugin checkout:

```bash
bash ~/.config/omarchy/plugins/io.github.aridev1.omamusi/scripts/remove-player.sh
omarchy plugin remove io.github.aridev1.omamusi
```

Removal affects only the managed player and its launcher. A separately installed development player is preserved. Git information may be unavailable in the pip-installed copy, in which case the footer shows `unknown` for the commit hash.

## Play

```bash
cd ~/Music
omaMusi
```

Playback starts automatically. You can also choose a folder, files, or a starting visual:

```bash
omaMusi ~/Music --recursive
omaMusi song.flac another.mp3
omaMusi --view "event horizon"
omaMusi --view "particle dance"
```

## Main controls

| Key | Action |
| --- | --- |
| Space | Play / pause |
| Up / Down, Enter | Select and play a song |
| N / P | Next / previous track |
| Left / Right | Seek backward / forward 5 seconds |
| + / − | Adjust player volume |
| V / Shift+V | Next / previous visual |
| F | Fullscreen |
| Tab | Hide / show playlist |
| C | Browse music folders |
| O | Add music files |
| / | Filter the playlist |
| Escape | Leave text input or fullscreen |
| Q | Quit |

While browsing folders, use the arrow keys and Enter to navigate; press **L** to play the current folder. Press **Escape** to return from text input to player shortcuts.

## Development checks

```bash
.venv/bin/python -m unittest discover -s tests
.venv/bin/python tests/smoke_gpu.py
```

The GPU smoke test needs a desktop session with a hardware GPU.
