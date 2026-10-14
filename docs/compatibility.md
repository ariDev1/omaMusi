# Compatibility validation for 0.11.0

omaMusi uses one Rust/GTK player on Linux. The optional Omarchy QML widget
launches that executable. Python is not needed to build or run the native player.

## Requirements

- Rust 1.92 or newer for source builds; the installed executable needs no Rust toolchain.
- GTK 4.6 or newer, Cairo, GLib and a Linux audio service (PipeWire/PulseAudio or ALSA).
- FFmpeg and ffprobe for decoding and metadata.
- OpenGL 3.3 for GPU rendering; Cairo fallback handles every visual.
- Source builds also need pkg-config, GTK/ALSA/PulseAudio development libraries and libepoxy.

## Recorded local checks

| Environment | Evidence |
| --- | --- |
| Rust 1.99, GTK 4.22.4 | Core and isolated-install tests, strict Clippy and formatting pass; production GTK workflows pass |
| Rust 1.92.0 | All targets compile; core and isolated-install tests pass |
| GTK 4.6.9 from Ubuntu Jammy, Rust 1.92 | All targets build against its pkg-config files; player workflows pass with an assertion confirming the loaded GTK version |
| Hyprland / AMD Radeon (radeonsi, renoir) | Real GPU tests cover nine modes, 64 switches, alpha/color, cover clearing, sustained particle input, context-safe cleanup and forced Cairo fallback |
| Temporary HOME/XDG directories | Native installation, recognized Python-managed upgrade, idempotency, removal, missing dependencies, failed-copy preservation and desktop-file validation pass |

The isolated GTK 4.6 test loads Ubuntu GTK alongside this host's newer GLib
and other libraries. Its workflows pass, but that mixed stack emits GTK
focus and GObject finalization diagnostics. This does not establish a clean
full Ubuntu 22.04 runtime. CI is configured to test the complete Ubuntu
22.04 stack with Rust 1.92, and Ubuntu 24.04 with stable Rust; those hosted
jobs have not been run from this unpushed checkout.

## Running verification

```bash
cargo fmt --all --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo test --locked --test audio -- --include-ignored --test-threads=1
cargo run --locked --bin render-smoke
cargo run --locked --bin player-smoke
cargo build --locked --release --bin omaMusi
```

The audio tests need a native or virtual output. Renderer and player smoke
checks need a desktop; each uses a separate process because GTK initialization
belongs to one thread. The drivers reject missing test registration instead
of treating an empty selection as a pass. For CI, `scripts/test-desktop.sh`
runs under Xvfb, a private D-Bus session, Openbox and virtual PulseAudio.
Software GL checks in CI do not substitute for hardware GPU evidence.

Filesystem jobs retain one running and one replaceable pending operation per
category. Cancellation rejects late results and shutdown does not wait for a
filesystem call already blocked in the kernel. FFmpeg children have bounded
capture or cancellation/reaping, and audio callbacks use preallocated rings.

## Real GNOME checklist

GNOME Shell is unavailable on this machine. Actual GNOME focus policy and
compositor transparency remain unverified; the previous application's user
reports do not establish Rust desktop parity. Check on a GNOME session:

1. Verify native decorations, move/resize, fullscreen and return from fullscreen.
2. In Waveform, verify the desktop is visible through empty pixels and text remains readable; other visuals have opaque backgrounds.
3. Minimize and launch again, including across workspaces. Verify one restored, focused window and uninterrupted audio.
4. Create/add/rename/remove/load saved playlists, cancel deletion, and verify text fields suppress playback shortcuts.
5. Close/reopen and verify playlist persistence and shuffle behavior.

The final migration audit and any outstanding observations are recorded in
`docs/rust-parity.md` when that audit is completed.
