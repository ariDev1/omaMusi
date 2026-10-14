# omaMusi Rust conversion design

## Intent and baseline

Convert the development player into a Rust application with feature parity,
without requiring Python, PySide6, NumPy, or PyOpenGL at runtime or during
normal builds. The user approved the migration order proposed in the branch
audit and asked to carry out the conversion.

The reference is development commit `d00010374b4c0ef09f3be56472f96f095d70a948`,
version 0.10.0: 21 Python application modules, 5,378 source lines including
embedded GLSL, nine visuals, saved playlists, and Linux desktop integration.
Feature parity is required; merely compiling a reduced Rust player does not
complete this conversion.

Assumptions: Linux remains the target platform; FFmpeg and ffprobe remain
external runtime dependencies; the optional Omarchy QML launcher remains.
Existing music and saved playlists must be preserved.

## Architecture and alternatives

Use GTK4 through gtk4-rs for native widgets, keyboard focus, drag and drop,
dialogs, window decorations, and application activation. Use GTK GLArea with
OpenGL 3.3 for accelerated visuals, and a Cairo DrawingArea for the software
fallback. Port the existing GLSL sources into shader assets and the rendering
host into Rust. Use CPAL for device selection and audio output; FFmpeg supplies
decoded PCM through bounded worker queues. Use Rust FFT and typed arrays for
analysis, and Serde JSON for playlists and ffprobe responses.

GTK4 is recommended because this is an existing Linux desktop player with
native focus and dialog behavior, rather than a new interface design.
Egui with an OpenGL callback is an alternative with custom drawing advantages,
but requires more recreation of widget and focus behavior. Qt Rust bindings
are another alternative that retains toolkit behavior, but introduce binding
and build complexity. Neither alternative is selected in this design.

Set the GTK runtime floor to 4.6 to support Ubuntu 22.04. Avoid newer GTK APIs
unless guarded with a working alternative. Commit Cargo.lock for the binary.
Select and pin the minimum Rust version after resolving the chosen dependency
versions; validate it in CI before claiming support. Keep the executable name
`omaMusi` and application ID `io.github.aridev1.omamusi`.

## Modules and ownership

- `library`: file discovery, natural sorting, deduplication, path handling,
  directory errors, and filename/folder search. No GTK dependency.
- `playlists`: version-1 JSON validation and atomic persistence. No GTK
  dependency. Preserve `~/.local/share/omamusi/playlists.json` and XDG override.
- `player`: selected track, playing track, playback history, shuffle, repeat
  navigation, search selection, and folder/playlist navigation state.
- `audio`: device negotiation, FFmpeg lifetime, bounded PCM delivery,
  pause/seek/volume, playback position, and generation-tagged events.
- `background`: bounded work by operation category with stale-result rejection.
- `artwork` and `gallery`: embedded/local images, bounded decoding and cache,
  grouping tracks by shared covers, transitions, hover targets, and hit testing.
- `visual`: FFT, music metrics, visual director, waveform persistence, horizon
  state, and shared deterministic render inputs.
- `render`: OpenGL resource ownership, shaders, particles, textures, and Cairo
  fallback. Rendering consumes audio snapshots; it never controls playback.
- `ui`: GTK widgets, actions, focus rules, overlays, folder and playlist dialogs.
- `desktop`, `theme`, and `cli`: application activation, launchers, read-only
  theme loading, command-line parsing, and version/revision reporting.

Keep GUI objects on the main thread. Audio callbacks must not allocate,
perform filesystem access, spawn processes, or wait on a UI-held lock. Worker
results contain owned data and generation IDs, never GUI object references.

## Audio and background behavior

Preserve stereo decoding, seeking, pause/resume, volume, natural end-of-track
advance, and supported source-rate floating output where the device allows it.
Otherwise negotiate an available output format and resample through FFmpeg.
Report the actual analysis/output sample rate correctly.

Bound PCM buffering to a fixed documented capacity. Drain stderr concurrently
with stdout so malformed media cannot deadlock decoding. Decoder cancellation
must kill and reap FFmpeg and release pipe readers even with a full queue.
Switching, seeking, or stopping invalidates prior completion/error events and
discards old buffered samples. A visual switch must not replace an audio stream.
Position tracks consumed audio and accounts for output latency where available.

Filesystem and metadata work stays off the main thread. Each category allows
at most one running and one replaceable pending operation. Ignore results from
superseded requests. Closing the window must not join a filesystem operation
that can be blocked indefinitely. Missing devices, unreadable directories,
missing media, and corrupt files produce actionable errors without corrupting
state or saved data.

## Interface parity

Preserve the existing window composition, scrollable song list, current-track
markers, metadata, seek/volume controls, footer shortcuts, and visual notice.
The toolkit change allows ordinary differences in native widget appearance;
it does not authorize removing workflows or changing shortcut meanings.

Preserve every control documented in the baseline README: playback, selection,
next/previous, shuffle and listening history, seeking, volume, search, adding
files, browsing folders, typed folder paths, adding/browsing saved playlists,
visual cycling in both directions, fullscreen, hiding the song list, and quit.
Text entry receives text keys rather than player shortcuts. Search uses file
and folder names case-insensitively; Up/Down exits search into matching results,
Enter plays selection, and no-match navigation keeps search focus.

Preserve folder navigation, drag-and-drop files/folders, recursive default
startup, explicit-file ordering, CLI view aliases and version output. Preserve
playlist create/add/rename/delete and song removal controls, deletion
confirmation, missing-file marking, and music-file safety. Random playback
includes tracks hidden by search and previous navigation follows history.

## Visual parity

Preserve Warp, Spectrum, Waveform, Spectrogram, Phi Cathedral, Event Horizon,
Particle Dance, Cover Art, and Cover Gallery. Extract existing shaders rather
than replacing effects with simplified placeholders. Port the analysis and
uniform generation that drive them, including FFT bands, peaks, history,
onsets, director state, and horizon wave payloads.

Preserve stable particle seeds when cycling views, bounded small particle dots,
the prominent gold Particle Dance ring, and color under maximum audio input.
Waveform empty regions remain transparent and retain accumulated trace color;
other visuals remain opaque. Verify compositor behavior separately from image
alpha checks. GPU failures select a working software renderer and identify the
reason. Every visual must have a functional software fallback.

Cover Art retains embedded-image extraction and named local-image fallback,
bounded image size, black empty background, right-side placement, and pulse.
Gallery retains the maximum 128 distinct-cover sample, asynchronous discovery,
fixed cover sizes, roughly five-second changes, crossfades, stable hover targets,
tooltips and click-to-play, avoiding the current song when alternatives exist.

## Desktop and delivery

Use one application instance per user session; subsequent launches restore and
activate the existing window without resetting playback. Preserve Hyprland
focus assistance where necessary and ordinary GNOME decorations. Respect XDG
Music location; a missing Music folder opens an empty player rather than scanning
the launcher's working directory. Theme reads do not modify desktop config.

Provide Cargo build/run commands and a source installer for the native binary.
Update standalone and Omarchy setup/remove scripts with ownership checks,
paths-with-spaces support, and preservation of playlists, music and unrelated
launchers. Upgrade only a recognized managed Python installation; retain an
unmanaged installation and explain how to migrate it. Desktop menu install,
remove and launch operations become Rust CLI operations.

Replace Python packaging and CI with Rust formatting, lint, regression, release
build and installation checks. Keep reference Python code and tests available
during migration; remove them from the active tree only after parity verification.
Git history remains the reference after removal. Update README, manifest version,
changelog and compatibility notes together, reporting only verified environments.
Do not publish a release or install over the user's active player as part of
development verification.

## Migration sequence and acceptance

1. Establish Rust core tests for discovery, search, player state and version-1
   playlists, using the Python behavior as the oracle.
2. Implement FFmpeg decoding and native audio with generated-media integration
   tests for duration, sample rate, seeking, cancellation, pause and completion.
3. Port analysis and rendering, with deterministic numeric comparisons and
   software/GPU image checks for all nine views and resource teardown.
4. Connect GTK interface, shortcuts, focus, browser/playlist dialogs, artwork,
   background operations, activation and desktop launch behavior.
5. Replace installers, docs and CI, verify an isolated install and upgrade, and
   retire the Python implementation only after the feature checklist is satisfied.

Each stage remains part of the full conversion objective. A stage passing its
tests is not a claim of complete migration. Track requirements and evidence in
the implementation plan and progress ledger. Run Rust tests, formatting, lint
and release build; run GUI/audio tests with a virtual audio output and real
desktop GPU checks on this machine. Retain the separate real-GNOME validation
limitation until that environment is tested.

Test malformed playlists without overwriting them, duplicate/case-equivalent
playlist names, missing songs, inaccessible or slow folders, Unicode names,
symlinks, empty libraries, stale background results, rapid seeks/view switches,
no output device, renderer failure, and close during decode/load. Installer
tests must confirm user data and unrelated files survive install/remove/upgrade.

## Official API references checked

- GTK GLArea: https://gtk-rs.org/gtk4-rs/stable/latest/docs/gtk4/struct.GLArea.html
- GTK DrawingArea: https://docs.gtk.org/gtk4/class.DrawingArea.html
- CPAL: https://docs.rs/crate/cpal/0.18.2

## Design status

Prepared for user review. Product implementation has not begun. An isolated
Rust 1.99.0 toolchain was installed under `/tmp` with user approval to support
subsequent builds; no shell setup or active player installation was changed.
