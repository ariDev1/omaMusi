# omaMusi Rust Conversion Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans or superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Replace the Python player, packaging and CI with a verified Rust player preserving every existing workflow and all nine visuals.

**Architecture:** A GUI-independent core owns library, playlists, navigation and analysis. GTK4 presents that state; CPAL consumes bounded FFmpeg PCM; OpenGL and Cairo render shared visual snapshots. Generation IDs reject stale worker and audio results.

**Tech Stack:** Rust, Cargo, gtk4-rs, Cairo, OpenGL 3.3, CPAL with PulseAudio support, FFmpeg/ffprobe, Serde JSON, RustFFT, image decoding, TOML theme parsing.

**Spec:** `docs/superpowers/specs/2026-10-14-rust-conversion-design.md`

## Global Constraints

- Linux remains the target platform; FFmpeg and ffprobe remain external runtime dependencies; the optional Omarchy QML launcher remains.
- Existing music and saved playlists must be preserved.
- Set the GTK runtime floor to 4.6 to support Ubuntu 22.04.
- Commit Cargo.lock for the binary.
- Keep the executable name `omaMusi` and application ID `io.github.aridev1.omamusi`.
- Audio callbacks must not allocate, perform filesystem access, spawn processes, or wait on a UI-held lock.
- Each category allows at most one running and one replaceable pending operation.
- Every visual must have a functional software fallback.
- Keep reference Python code and tests available during migration; remove them from the active tree only after parity verification.
- Do not publish a release or install over the user's active player as part of development verification.
- The minimum supported Rust version is selected from resolved dependency requirements and then verified in CI. Do not advertise an untested floor.

## Review Focus

- Files named with Unicode, spaces, shell metacharacters or invalid UTF-8: discovery and process arguments preserve paths and never invoke a shell. Task 1/3 tests own this.
- Symlink loops and unreadable nested directories: recursive discovery terminates and reports errors while retaining the prior queue. Task 1/2 tests own this.
- Output devices disappearing or refusing a source format: playback fails visibly or negotiates a supported format; stale events cannot advance a new track. Task 3 tests own this.
- GTK/OpenGL context loss and transparent Wayland windows: software fallback works, cleanup uses the owning context, and waveform alpha survives composition. Task 6/7 checks own this.
- Interrupted upgrade or unrelated launchers in the data directory: installation preserves the previous executable and playlists and refuses unmanaged replacement. Task 8 tests own this.

## File structure and shared types

Create `Cargo.toml`, `Cargo.lock`, `src/lib.rs`, `src/main.rs`, and focused modules
under `src/`: `library.rs`, `playlists.rs`, `player.rs`, `background.rs`,
`audio/{mod,decoder,output}.rs`, `artwork.rs`, `gallery.rs`,
`visual/{mod,analysis,director,horizon,waveform}.rs`,
`render/{mod,gl,cpu}.rs`, `ui/{mod,actions,browser,playlists}.rs`,
`cli.rs`, `desktop.rs`, and `theme.rs`. Extract shader text to `assets/shaders/`.
Place Rust integration tests under `tests/rust/`, registered explicitly in Cargo
to coexist with Python tests until retirement. Put GUI/GPU smoke binaries under
`src/bin/`; they must exercise production widgets/rendering rather than copies.

Use `anyhow::Result<T>` for operations that can fail. Core paths are `PathBuf`,
never lossy display strings. `Generation` is a `u64`. `PlaybackSnapshot` owns
position, duration, pause state, output rate, volume, generation and error state.
`PcmSnapshot` owns 4,096 mono `f32` samples, sample rate, revision and generation.
`VisualMode` enumerates the nine baseline modes in their current order.
`VisualFrame` carries renderer-independent analysis, theme and elapsed state.
`Artwork` owns bounded RGBA pixels and dimensions. GUI conversion happens on
the main thread. Tests may use temporary directories and generated WAV files.

Every task follows RED → GREEN → full task verification → commit. Run test
commands with approved desktop/audio access when the sandbox prevents them.
Log evidence and deviations in the execution ledger. No task may be marked
complete solely because its files exist or a narrower test passes.

### Task 1: Rust library and playlist core

**Files:** Cargo files, `src/lib.rs`, `src/library.rs`, `src/playlists.rs`, `tests/rust/core.rs`, `.gitignore`.

**Interfaces:** Produce `discover(paths: &[PathBuf], recursive: bool) -> Result<Vec<PathBuf>>`, `matches(path: &Path, query: &str) -> bool`, and `PlaylistStore::open(path: PathBuf) -> Result<Self>` with `names`, `tracks`, `create`, `add`, `rename`, `remove`, and `delete` operations matching the baseline.

- [x] Write core tests first: natural order `2.FLAC` before `10.mp3`; explicit-file order and deduplication; recursion excludes symlinked directories; parent-folder case-insensitive matches; Unicode and metacharacter paths survive; missing/unreadable paths return errors.
- [x] Write persistence tests first: reopen retains track order; duplicate add returns false; case-equivalent names collide; blank/control/over-120-character names fail; malformed JSON and failed save preserve existing bytes; playlist removal never removes music.
- [x] Run `cargo test --test core`; expect failure from missing core implementation before implementing it.
- [x] Implement the core with version-1 JSON and atomic same-directory write/fsync/rename. Retain the baseline extension list and explicit files beyond it.
- [x] Run `cargo test --test core`; expect all tests pass. Run `cargo fmt --check` and commit the task.

### Task 2: Navigation state and bounded background work

**Files:** `src/player.rs`, `src/background.rs`, `tests/rust/state.rs`, `tests/rust/background.rs`.

**Interfaces:** Consume Task 1. Produce `PlayerState` with queue/current/selection/history/shuffle/search/browser state; `dispatch(action: Action) -> Vec<Effect>` updates state without GTK. `BackgroundTasks::submit(category, operation)` returns a generation; `poll()` returns owned results; `invalidate` and `close` discard pending work/results.

- [x] Write state tests for next/previous and restart behavior from `app.py`, shuffled tracks hidden by search, matched-result wrap, no-match focus, queue preservation after failed scans, repeated additions retaining their recursion flags, missing playlist songs, and empty playlist preserving playback.
- [x] Write worker tests using controlled channels: one running/one replaceable pending task, latest result wins, invalidate suppresses completion, and close returns immediately while an operation remains blocked.
- [x] Run `cargo test --test state --test background`; expect missing-implementation failure.
- [x] Implement typed actions/effects and per-category bounded workers. Port baseline behavior, including metadata loading pause intent and browser cancellation, without GUI objects in workers.
- [x] Run `cargo test --test state --test background` and `cargo test`; expect all pass; commit.

### Task 3: FFmpeg decoder and native audio output

**Files:** `src/audio/{mod,decoder,output}.rs`, `tests/rust/audio.rs`.

**Interfaces:** Produce `probe(path: &Path) -> Result<Metadata>`, `AudioPlayer::new() -> Result<Self>`, `play(path, offset, source_rate)`, `pause`, `seek`, `stop`, `set_volume`, `snapshot() -> PlaybackSnapshot`, `samples() -> PcmSnapshot`, and generation-tagged finished/failed events. Task 2 effects call this API.

- [x] Write generated-tone tests: 1.2-second 440-Hz stereo decode; seek at 0.7 yields 0.5 seconds; 96-kHz float decode preserves rate; malformed media reports error; metacharacter filenames decode; full-queue cancellation reaps the child promptly.
- [x] Write virtual-audio integration tests: pause freezes position, resume advances, volume retains current events, stop/seek/replay discard old events and old PCM, rapid seeks terminate, unsupported rates negotiate an available format, missing/disconnected device yields an error, EOF advances exactly once.
- [x] Run `cargo test --test audio`; expect missing audio implementation failure.
- [x] Implement bounded producer queues and a preallocated audio consumer. Drain stderr concurrently; use child termination/reaping on cancellation; negotiate stereo/source rate where supported and convert via FFmpeg otherwise. Report consumed-frame position with available latency correction.
- [x] Run `cargo test --test audio` against virtual output and `cargo test`; expect all pass. Check callback code for allocation and blocking operations; commit.

### Task 4: Analysis, director and waveform state

**Files:** `src/visual/{mod,analysis,director,horizon,waveform}.rs`, `tests/rust/analysis.rs`, `tests/rust/fixtures/`.

**Interfaces:** Consume `PcmSnapshot`. Produce `VisualMode`, `VisualFrame`, and `VisualState::{feed,reset,tick,cycle}`; feed/tick never mutate audio playback. Numeric state includes 96 bands/peaks, FFT frequencies, history, onset metrics, director, horizon payload and waveform density.

- [x] Generate deterministic reference fixtures from the baseline Python analyzer/director/horizon/waveform tests and fixed PCM inputs; retain fixture provenance and tolerances.
- [x] Write tests first comparing quiet/tone/transient/max-input frames to fixtures, sample-rate resets, repeated sample revisions, forward/backward nine-mode cycling, stable particle seeds, finite/clamped metrics and pause persistence.
- [x] Run `cargo test --test analysis`; expect missing visual state implementation failure.
- [x] Port algorithms from `visualizer.py` and `omamusi/visual/` using RustFFT and fixed typed buffers; preserve smoothing, band mapping and director parameters rather than approximating them.
- [x] Run `cargo test --test analysis` and `cargo test`; expect all pass within explicit fixture tolerances; commit.

### Task 5: Artwork and interactive cover gallery

**Files:** `src/artwork.rs`, `src/gallery.rs`, `tests/rust/artwork.rs`, `tests/rust/gallery.rs`.

**Interfaces:** Produce `load_artwork(path: &Path) -> Result<Option<Artwork>>`; `CoverGallery::{add,layout,hover,hit,tick,clear}` accepts bounded artwork and paths, returns render rectangles and track groups. Consume Task 2 generations for asynchronous loading.

- [x] Write tests first for embedded MP3/FLAC images, named JPEG/PNG/WebP fallback, malformed/oversized images, bounded subprocess time, and stale track/folder results.
- [x] Write gallery tests first for shared-art grouping, at most 128 cached covers, 320-pixel thumbnails, responsive layout with 18-pixel gaps and 240-pixel maximum tiles, 5.0-second rotation, 1.2-second crossfade, hover freezing the dominant image, clicks avoiding the current track and fixed sizes under bass changes.
- [x] Run `cargo test --test artwork --test gallery`; expect missing implementation failure.
- [x] Implement bounded image decoding and subprocess extraction, grouping, shuffled source sampling and baseline transition/hit rules.
- [x] Run `cargo test --test artwork --test gallery` and `cargo test`; expect all pass; commit.

### Task 6: OpenGL and software renderers

**Files:** `assets/shaders/`, `src/render/{mod,gl,cpu}.rs`, `tests/rust/render.rs`, `src/bin/render-smoke.rs`.

**Interfaces:** Consume `VisualFrame` and artwork/gallery render inputs. Produce `CpuRenderer::draw(context, frame, width, height)` and `GlRenderer::{new,draw,cleanup}` operating only with the owning current context. No rendering call replaces audio output.

- [x] Write software image tests first for every visual, cover placement/empty black frame, gallery fades, waveform transparent pixels and persistent color, gold ring prominence, small particle dots and bounded color at maximum input. Test invalid dimensions and very small windows.
- [x] Write production GL smoke checks first: render all nine modes, compare stable Warp after 64 switches, clear cover texture to black, verify waveform alpha/color, isolate particle size, and test cleanup once with the correct context and failure fallback.
- [x] Run `cargo test --test render` and `cargo run --bin render-smoke`; expect missing render implementation failure.
- [x] Extract shader constants verbatim from the Python reference; port uniform/texture/buffer/transform-feedback management and all Cairo fallback drawing algorithms. Use GTK GLArea with required OpenGL 3.3 and a software DrawingArea on error.
- [x] Run software and real-GPU smoke checks; expect every named check passes. Run `cargo test`; commit. A headless software run cannot satisfy the GPU check.

### Task 7: GTK player and desktop behavior

**Files:** `src/main.rs`, `src/cli.rs`, `src/theme.rs`, `src/ui/{mod,actions,browser,playlists}.rs`, `src/desktop.rs`, `tests/rust/cli.rs`, `tests/rust/theme.rs`, `src/bin/player-smoke.rs`.

**Interfaces:** Consume Tasks 1–6. Produce `run(cli: Cli) -> Result<()>`, `Cli::parse_from`, production `PlayerWindow`, GTK actions, theme loader and single-instance activation. Add `--desktop install|remove|launch` as the Rust replacement for the Python desktop module.

- [x] Write CLI/theme tests first for baseline flags/view aliases/version, default recursive cwd, explicit path order, built-in palette, invalid colors, absent theme files and read-only config access.
- [x] Write GUI smoke checks first using production actions/widgets: every baseline README shortcut; search focus/no-match/wrap/Enter; folder controls; drag/drop; pending metadata pause; seek/volume; history/shuffle; playlist create/add/rename/remove/delete confirmation/cancel; missing songs; text fields suppress shortcuts; scroll/resize/fullscreen/Tab.
- [x] Add second-process activation checks: minimized restore, existing audio retained, missing Music starts empty, crash recovery, and Hyprland focus behavior. Add real desktop waveform-composition check; retain explicit GNOME environment limitation.
- [x] Run `cargo test --test cli --test theme` and `cargo run --bin player-smoke`; expect missing implementation failure.
- [x] Implement GTK composition resembling baseline, actions/effects integration, bounded asynchronous loaders, renderer fallback and error messages, GApplication single instance, XDG locations and safe desktop entry operations. Use GTK 4.6 APIs.
- [x] Run GUI/audio/activation smoke on this desktop and `cargo test`; expect all automated checks pass; commit. Record compositor checks and environment evidence individually.

### Task 8: Native installation, migration and CI

**Files:** scripts setup/remove/launch, `tests/rust/install.rs`, `.github/workflows/tests.yml`, `README.md`, `docs/compatibility.md`, `CHANGELOG.md`, `manifest.json`, Cargo metadata.

**Interfaces:** Installer consumes built `omaMusi` and `--desktop` operations; stores managed binary under XDG data home and launcher under `~/.local/bin`, preserving the existing ownership marker and playlist location.

- [x] Write isolated install/remove/upgrade tests first with temporary HOME/XDG roots: paths with spaces, idempotency, playlist preservation, unrelated launcher/icon refusal, recognized Python-managed upgrade, unmanaged install refusal, missing dependencies and interrupted copy preserving the prior executable. Validate generated desktop entries.
- [x] Run `cargo test --test install`; expect failure against Python-dependent installer behavior.
- [x] Implement native source build and atomic binary replacement, safe desktop entry migration/removal and launcher updates. Preserve user data and unrelated files; never upgrade the user's real installation during tests.
- [x] Replace CI with fmt, clippy with warnings denied, full core/integration suite, virtual PulseAudio, Xvfb software GL/GUI checks, minimum-GTK compatibility build and release packaging. Resolve dependencies and set the tested Rust floor. Update user instructions and version to 0.11.0 consistently.
- [x] Run `cargo test --test install`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo build --release --bin omaMusi`, and isolated native install tests; expect all pass; commit.

### Task 9: Parity audit and Python retirement

**Files:** remove active Python application/tests/pyproject only after audit; update `.gitignore`, documentation and `docs/rust-parity.md`; preserve icon under `assets/`.

**Interfaces:** Native release binary, updated installers and all Rust tests are the sole active player/build path. Git history retains the Python oracle.

- [x] Create `docs/rust-parity.md` mapping every spec paragraph and baseline README workflow to concrete test names, commands, artifacts and desktop observations. Include all nine visual/software/GPU checks, cancellation/teardown cases and installer safety. Record genuine limitations rather than marking missing evidence complete.
- [x] Run the full Python reference suite while still present and compare deterministic fixtures and workflow checks with Rust. Expect baseline pass and every parity requirement covered. Fix gaps using failing Rust tests before deleting the reference.
- [x] Remove Python runtime/build paths and obsolete tests from the active tree, move icon, update references, and verify tracked-file searches contain no Python dependency in install/build/run/CI paths. Preserve ordinary shell/QML/GLSL assets.
- [x] Run full Rust fmt/clippy/tests/release build, isolated install/upgrade checks, production GUI/audio smoke and real GPU checks from the final tree; expect all pass. Run supported-Rust and GTK-floor checks before claiming those versions.
- [x] Obtain whole-branch review of the spec, plan, ledger and implementation; fix important findings with RED→GREEN tests and full-suite verification. Commit final state; leave integration/publishing to the user unless separately authorized. Mark the full goal complete only when the parity audit proves the requested end state.

## Plan self-review

The shared core, audio snapshots, artwork and visual frame types above are the
interfaces consumed by later tasks. Task 9 owns final requirement-by-requirement
coverage; task-level tests cannot substitute for that audit. Tasks are ordered
because UI and render parity depend on the preceding state/data contracts.
No release publication or real installation is needed to prove the conversion.
Task completion requires evidence; hardware/environment limitations remain
visible and do not silently waive a feature.

## Execution choice

Recommended: native execution in this session, followed by one independent
whole-branch review. The tasks share substantial state and rendering contracts;
keeping implementation in one context avoids repeated handoffs. Alternatively,
subagent-driven execution adds independent review after each task with higher
context cost. Implementation starts after the user reviews this written plan
and selects the execution method.
