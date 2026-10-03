# omaMusi

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
