# omaMusi

Current version: **v0.10.0**.

Licensed under the [MIT license](LICENSE).

**Your music. A little text. A whole universe of visuals.**

omaMusi is a local music player for Linux with animated visuals that react to your music. Open it in your music folder, choose a song, and enjoy. The interface stays out of the way: just your playlist, track information, and playback time.

![omaMusi playing music with the Event Horizon visualization](event-horizon.png)

## What it does

- Plays your local music, including MP3, FLAC, WAV, Ogg, Opus, M4A, and AAC.
- Shows song titles, artists, and albums, and advances to the next track automatically.
- Finds music by filename or folder name across nested folders, with keyboard navigation through results.
- Lets you browse folders, save playlists, or drag music into the window.
- Follows your Omarchy theme and terminal font.
- Runs music-reactive visuals on your GPU, with a simpler fallback when hardware rendering is unavailable.

The visuals analyze the music without changing its sound.

The muted footer groups keyboard shortcuts by function and shows the version and short Git commit hash. A `+dirty` suffix means the checkout has uncommitted changes; the hash reads `unknown` when Git information is unavailable. You can also check the version with `omaMusi --version`.

Basic player and search use has been tested on Omarchy and Ubuntu. Omarchy keeps the frameless window; GNOME uses native window decorations. See [compatibility validation](docs/compatibility.md) for automated checks and the remaining desktop checks.

## Eight visuals

- **Event Horizon** — a cinematic black hole with a glowing disk, gentle camera drift, bass-driven waves, orbiting hot spots, and delayed light echoes.
- **Particle Dance** — thousands of colorful particles dancing to the music, with a faster torus drifting through the swarm.
- **Phi Cathedral** — evolving golden-ratio patterns and spirals.
- **Warp** — a swirling flight through colored star streams.
- **Spectrum**, **Waveform**, and **Spectrogram** — three classic ways to see your music.
- **Cover Art** — a single album cover on the right side of the window, with a gentle bass-driven pulse against a plain black background. The image stays sharp and keeps its proportions. Without readable artwork, the stage is black.

Press **V** to switch visuals, or **Shift+V** to go back.

Cover Art reads embedded artwork first, then `cover`, `folder`, `front`, or `album` images in the track's folder (`.jpg`, `.jpeg`, `.png`, or `.webp`, without regard to case). Artwork loads in the background when this visual is selected and never changes the music files. Start directly with `omaMusi --view "cover art"`.

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

The optional Omarchy Quattro bar widget uses an Event Horizon symbol with Omarchy's native icon sizing, a theme-colored ring, and a gold light band. Click it to open omaMusi with Event Horizon selected. It opens your Music folder recursively, or an empty playlist if that folder is missing. The player continues to work as a standalone application on GNOME and other desktops.

Install from the repository:

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

Removal affects only the managed virtual environment and its launcher. Saved playlists and other user data remain in the data directory, together with the ownership marker for later reinstallation. Setup also accepts a data directory created by standalone playlist use. A separately installed development player is preserved. Git information may be unavailable in the pip-installed copy, in which case the footer shows `unknown` for the commit hash.

## Play

```bash
cd ~/Music
omaMusi
```

Playback starts automatically.

Starting in a folder with `omaMusi` (or `omaMusi -all`) includes music in
all nested subfolders. Press **/** to search filenames and folder names,
without regard to case. A matching folder shows all loaded songs beneath
it, including songs in deeper subfolders. Tracks display their relative
paths so files with the same name can be distinguished. Clear the search
to show the whole queue again. While typing a search, press **Up/Down** to
select a matching result and leave the search field, then **Enter** to play
it. The search stays active; press **/** to edit it again. With no matches,
the arrow keys keep focus in the search field. Explicit directory arguments
still require `--recursive` to include their subfolders.

You can also choose a folder, files, or a starting visual:

```bash
omaMusi ~/Music --recursive
omaMusi song.flac another.mp3
omaMusi --view "event horizon"
omaMusi --view "particle dance"
```

Folder scans and track metadata load in the background, including the initial scan. The window stays responsive while waiting for storage. Failed scans leave the existing queue and playback intact and show an error. Escape cancels a pending folder change or folder browse; an outdated result cannot reopen it. Pressing Space while a track loads preserves the requested pause when playback starts.

## Main controls

| Key | Action |
| --- | --- |
| Space | Play / pause |
| Up / Down (or K / J), Enter | Select and play a song |
| N / P | Next / previous track |
| R | Toggle random playback (off by default) |
| A | Add highlighted track to a saved playlist (falls back to playing track) |
| B | Browse saved playlists |
| Left / Right | Seek backward / forward 5 seconds |
| + / = / − | Adjust player volume |
| V / Shift+V | Next / previous visual |
| F | Fullscreen |
| Tab | Hide / show playlist |
| C | Browse music folders |
| O / Ctrl+O | Add music files |
| Ctrl+L | Enter a folder path |
| / | Search loaded filenames and folder names |
| Escape | Leave text input or fullscreen |
| Q | Quit |

While browsing folders, use the arrow keys (or **K/J**) and **Enter** to
navigate; **Right/Enter** opens the selection, **Left/Backspace** goes to
the parent folder, and **L** plays the current folder. Press **Escape** to
return from text input to player shortcuts. The footer groups all player
shortcuts by function and shows folder navigation while browsing; saved
playlist dialogs show their own controls at the bottom.

Random playback uses the entire loaded playlist, including tracks hidden by a search filter. When enabled, **N** and automatic advancement choose a random track, avoiding the current track when more than one is loaded. **P** keeps its usual restart/history behavior. Pressing **R** leaves the current song playing; the footer shows `random on` or `random off`.

Waveform uses an oscilloscope-style colored persistence map. Traces fade over roughly two seconds; repeated traces build from purple/blue through cyan and green to yellow/red, with a thin bright line for the latest waveform. Empty areas reveal the desktop in Waveform mode, while the other visuals paint their usual opaque backgrounds. Track changes and seeking clear the persistence map. Use **V** to cycle to Waveform, or start with `omaMusi --view waveform`.

## Saved playlists

Select a track with **↑/↓**, then press **A**. Choose a playlist with **↑/↓** and press **Enter** to add it, or choose **Create new playlist…**, type a name, and press **Enter**. Adding leaves playback running and prevents duplicate entries. With no highlighted track, **A** uses the playing track. It also works on audio files highlighted in the folder browser.

Press **B** to browse saved playlists:

| Key | Action in the playlist browser |
| --- | --- |
| ↑ / ↓ | Select a playlist or track |
| Enter | Load the selected playlist as the play queue and start playback |
| → / ← | Inspect playlist tracks / return to playlist names |
| F2 | Rename the selected playlist |
| Delete | Remove a track, or request deletion of a playlist |
| Escape | Cancel a name/deletion prompt, or close the browser |

Playlist deletion asks for **Enter** to confirm; **Escape** cancels. Removing tracks or deleting playlists leaves the music files untouched. Missing files are marked `[missing]` when inspecting tracks and skipped when loading a playlist. An empty playlist, or one with no available files, leaves the current queue and playback intact. Random playback also works with loaded playlists.

Playlists save automatically to `${XDG_DATA_HOME:-~/.local/share}/omamusi/playlists.json` as ordered references to absolute file paths. Names are unique without regard to case. **A** and **B** keep their normal text-entry behavior while typing in a field.

## Development checks

```bash
.venv/bin/python -m unittest discover -s tests
.venv/bin/python tests/smoke_gpu.py
```

The GPU smoke test needs a desktop session with a hardware GPU.

GitHub Actions runs the full regression suite and a wheel build on pushes to `development` and `main`, and on pull requests. It tests Python 3.11 with the minimum supported dependencies and Python 3.14 with current dependencies, using a virtual audio output. Hardware GPU/audio checks and a real GNOME session remain release checks; hosted CI does not validate either desktop compositor.

See [compatibility validation](docs/compatibility.md) for tested Python/Qt versions and the GNOME desktop checks.
