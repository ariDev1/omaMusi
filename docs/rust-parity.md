# Rust conversion parity audit

Reference: development `d00010374b4c0ef09f3be56472f96f095d70a948` (0.10.0).
Target: native Rust/GTK 0.11.0. The optional QML launcher and original GLSL
remain ordinary assets; the player does not import or execute Python.

Status: 63 tracked reference/packaging/test files have been retired from the active tree. Final-tree verification and independent
whole-branch review are being completed. This report does not
claim a real GNOME session or an unrun hosted CI job has passed.

## Reference evidence

The original tree was extracted with `git archive` into
`/tmp/omamusi-python-oracle-d000103`. Its original regression suite passed all
175 tests in 22.537 seconds, using the existing isolated Python environment,
offscreen Qt and desktop audio. The log is
`/tmp/omamusi-python-oracle-final.log`. Native installation tests use temporary
HOME/XDG directories; no active user installation was replaced.

`tests/rust/fixtures/visual-reference.json` and `beat-reference.json` retain
numeric oracle inputs, expected outputs and provenance. Their README records
analysis/FFT/density tolerances and one-byte pixel interpolation tolerance.
Shader and star/swarm assets were extracted from the same reference. Unicode
16 full case folding is compiled as 1,557 mappings from the reference runtime;
its data license is in `assets/UNICODE-LICENSE.txt`.

## Requirements and evidence

The rows below map the approved design's acceptance requirements to production
code and concrete verification. Test names refer to `tests/rust/*.rs` unless
they include a `ui::` or `render::gl::` prefix.

| Requirement | Implementation and evidence |
| --- | --- |
| Native application, no Python build/runtime | `Cargo.toml`, `src/main.rs`, native installer and Rust workflow; final tracked-file/dependency search and release build |
| Existing executable name/application ID | `omaMusi`, `io.github.aridev1.omamusi`; CLI, desktop-entry and real activation checks |
| GTK 4.6 API floor, pinned dependencies | gtk4 `v4_6`, Cargo.lock; Rust 1.92 all-target build and real GTK 4.6.9 loader assertion in `desktop_workflows` |
| Version/revision display | CLI version tests, `build.rs`, `src/version.rs`; visible footer in compositor captures |
| Module/thread ownership | Owned Work/TaskResult values, GTK Rc state on main thread, preallocated atomic audio snapshots/rtrb rings; compiler and source review |
| File discovery, natural ordering, dedup and explicit order | `discovery_natural_order_dedup_recursion_and_explicit_order` |
| Unicode, metacharacters and non-UTF-8 discovery | `discovery_preserves_unicode_metacharacters_and_non_utf8_paths`, `casefold_matches_unicode_16_reference_including_newer_scripts` |
| Symlinks, parent components, missing saved paths and unreadable directories | `paths_resolve_symlinks_before_parent_components_even_for_missing_tracks`, `discovery_does_not_follow_recursive_symlink_loops_and_reports_permissions` |
| Filename and parent-folder search | `search_matches_parent_folders_case_insensitively_and_empty_query`, state search tests and production GUI searches |
| Version-1 JSON/location compatibility | `baseline_version_one_json_is_read_without_migration`, shared absolute XDG data root, installer preservation tests |
| Playlist name limits, case collisions, malformed JSON and failed writes | `playlist_invalid_names_and_collisions_preserve_bytes`, `malformed_playlist_and_failed_save_preserve_data` |
| Ordered playlist persistence and music safety | `playlists_persist_order_deduplicate_and_leave_music_untouched`; GUI/installer checks leave music and saved data intact |
| Playback/pause/seek/volume and source-rate output | `native_audio_pause_seek_volume_and_analysis`, `native_audio_rapid_seeks_and_unsupported_rate_fallback`, `floating_decoder_retains_96khz_source` |
| Transport hour formatting | `ui::tests::transport_clock_preserves_hour_format` preserves h:mm:ss for long recordings |
| Stereo, duration and tags | `decoder_preserves_duration_stereo_seek_and_metadata`; GUI title and transport checks |
| Bounded decoder queue and cancellation/reaping | `malformed_media_reports_error_and_full_queue_cancellation_reaps_child`; fixed 65,536-float ring, concurrent bounded stderr drain |
| Output latency and end position | Supported 20-ms callback buffers and consumed-frame position; native audio pause/seek/completion tests |
| Completion once, stale audio events and replay | `native_audio_finished_once_and_old_events_discarded_on_replay`, `space_replays_current_track_after_end_or_metadata_failure` |
| Corrupt media and output-device loss | `native_audio_corrupt_media_emits_failure_once_and_can_recover`, `native_audio_removed_device_is_reported_without_starting_decoder` |
| One running/one pending job per category | `one_running_one_replaceable_pending_and_only_latest_result_delivered` |
| Stale work, failures, cancellation and close | Background invalidation/error/panic/close tests; GUI stale-metadata-batch regression verifies a newer playback request survives |
| No blocked theme read on GUI thread | GUI FIFO fixture: construction returns before the delayed writer, and closing does not wait for the read |
| Failed folder scans preserve queue/audio | State failure/cancel tests; GUI file-as-folder rejection retains audio generation and the editable prompt |
| Canonical/nonrecursive folder navigation | Dedicated folder effect validates/resolves on the worker; GUI parent scan excludes nested album songs and returns canonical folder |
| Cancel typed path without leaving browse | `cancel_folder_prompt_keeps_browser_and_rejects_late_folder_results`; GUI browse/prompt/Escape checks |
| History, shuffle, hidden songs and end behavior | State next/previous/shuffle tests; GUI R/N/P; automatic end stops or advances as appropriate |
| Metadata pause request and errors | `metadata_generation_rejects_stale_tracks_and_preserves_pause_intent`; GUI toggles pause before metadata is applied |
| Seek-error recovery/startup failure | GUI failed-seek/replay check and deleted-cwd child check; startup errors propagate through `run_app` |
| Main composition, metadata, list, controls and hints | `ui::tests::desktop_workflows`; real compositor captures show title, hints and version |
| Search focus/no-match/wrap/Enter | State search navigation test; GUI no-match keeps focus, suppresses player text keys and does not activate a hidden row |
| Add files and dropped files/folders | CLI ordering/discovery tests; GTK native chooser and FileList drop target; GUI exercises the production path ingestion handler |
| Recursive default startup and view aliases | CLI default/order/alias tests, including legacy `-all` and `--` separator behavior |
| Saved-playlist create/add/rename/delete/remove | GUI production modal operations, delete cancellation/confirmation, missing-file inspection/removal/loading and file preservation |
| Text input and focus return | GUI search/folder/playlist entry checks; modal closure restores player focus |
| Resize, scroll, fullscreen, Tab and quit | GUI workflow plus real compositor 60-track/long-filename check at 640×420, selection scrolling and footer bounds |
| All nine accelerated visuals | `render::gl::tests::hardware_rendering`: Warp, Spectrum, Waveform, Spectrogram, Phi Cathedral, Event Horizon, Particle Dance, Cover Art and Cover Gallery |
| All nine software visuals | `all_nine_software_visuals_render_distinct_frames_without_mutating_audio` and dedicated software image tests |
| Original analysis/uniform generation | Eight analysis tests compare FFT bands/peaks/history, onsets, director, beats, horizon payloads and waveform against the fixed oracle |
| Stable seeds and visual switching | Analysis seed/persistence checks; 64 GPU switches return the same Warp frame; GUI checks unchanged audio generation across nine mode changes |
| Small particles, gold ring, loud-input color | Software geometry/ring tests and GPU isolated-point/sustained-maximum-input checks; no white-out |
| Waveform transparent pixels and retained color | Software persistence/pixel tests, GPU alpha/color readback and real compositor check |
| Separate compositor proof | `ui::compositor_tests::waveform_composes_with_desktop` captures an owned green background through Waveform (green about 199–204), versus opaque Spectrum (38) |
| GPU failure and context-safe teardown | Hardware test verifies forced failure selects a painted Cairo surface; owning-context cleanup is idempotent |
| Cover extraction/local fallback, size/orientation bounds | Seven artwork tests cover MP3/FLAC embedded art, JPEG/PNG/WebP sidecars, EXIF orientation, downscaling, malformed/oversized files and time/output limits |
| Cover right placement, pulse and black empty stage | Software cover test plus GPU cover rendering/clearing |
| Gallery bounded cache/shared-art groups/sample | Five gallery tests cover 128 thumbnails, grouping, sampling and current-song avoidance |
| Gallery layout/rotation/fade/hover/click/tooltips | Gallery layout/transition/hit tests and real GTK registered-click/tooltip case; five-second rotation, 1.2-second fades, fixed cover size |
| One instance and crash recovery | GUI driver uses a second process, verifies existing playback generation/focus, kills a separate registration fixture and reacquires it |
| Hyprland assistance | Bounded worker supports legacy and Lua dispatchers, validates window addresses and selects main/modal title; fake IPC regression plus real second-launch focus check |
| GNOME decorations | Native HeaderBar/decorated branch implemented; real GNOME compositor/focus checks remain explicitly unverified |
| Localized/missing Music location | `configured_localized_music_is_recursive_and_missing_or_home_opens_empty`, plus empty native window assertion |
| Read-only themes, defaults and live reload | Three theme tests plus GUI palette reload without audio-generation change; startup and periodic reads run on a bounded worker |
| Native install/update/remove and path escaping | Seven isolated installer tests plus desktop ownership/escaping tests and actual setup/remove scripts in a temporary home |
| Safe recognized Python-managed upgrade | `recognized_python_installation_upgrades_without_running_python`; old venv removed only after successful native install |
| Unmanaged data/launchers/icons retained | Negative desktop/installer tests; removal script rejects an unmanaged executable before running it; README explains manual migration |
| Interrupted copy and dependency failure | procfs EIO copy fixture preserves prior executable; missing FFmpeg creates no installation |
| Absolute XDG fallback | `relative_xdg_data_home_uses_standard_absolute_user_directory`; player, menu and installer share the same resolver |
| Rust build/lint/tests/release/CI | Native workflow with Ubuntu 22.04/Rust 1.92 and Ubuntu 24.04/stable, virtual PulseAudio, Xvfb/Openbox/D-Bus and native release archive |
| Consistent delivery documentation/version | Cargo/README/manifest/changelog/compatibility and marketplace draft updated; no publication or real-user installation performed |

## Reproduction commands

```bash
cargo fmt --all --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo test --locked --test audio -- --include-ignored --test-threads=1
cargo run --locked --bin render-smoke
cargo run --locked --bin player-smoke
cargo test --locked --lib ui::compositor_tests::waveform_composes_with_desktop -- --ignored --exact --test-threads=1 --nocapture
cargo build --locked --release --bin omaMusi
```

Run GTK cases in separate processes; initializing GTK on different test-harness
threads in one process is unsupported. The compositor case requires real
Hyprland and grim; it adjusts only its own temporary windows. Its PNGs are in
`/tmp/omamusi-rust-compositor/`. CPU render artifacts are in
`/tmp/omamusi-rust-render/`. The smoke drivers reject missing registrations.

The isolated Rust toolchains are under `/tmp/omamusi-cargo` and
`/tmp/omamusi-rustup`; normal commands above use the equivalent installed
Cargo. GTK 4.6.9 packages and the loader wrapper are under
`/tmp/omamusi-gtk46`. The wrapper asserts the loaded GTK minor version;
setting LD_LIBRARY_PATH only through Cargo configuration is insufficient,
since Cargo constructs the runtime search path for child binaries.

## Environment limits

Real GNOME Shell is unavailable. Its decoration/focus/transparency checklist
remains in `docs/compatibility.md`; neither a software image nor a GTK flag
assertion is represented as real GNOME compositor evidence.

GTK 4.6.9 runs here alongside newer host GLib/Pango libraries. Workflows and
GPU/fallback checks pass, but the mixed stack emits finalization diagnostics;
a few rapid synthetic focus transitions also emitted GtkText diagnostics on
the current stack. The complete Ubuntu 22.04 runtime and hosted matrix jobs
have not been run from this unpushed checkout. Those limits are retained in
compatibility documentation rather than silently waived.

## Review and final tree

Pending: final tracked-source retirement verification, complete Rust checks
from that tree and the independent whole-branch review. The goal remains
active until that evidence is recorded.
