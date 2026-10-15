# Rust conversion parity audit

Reference: development `d00010374b4c0ef09f3be56472f96f095d70a948` (0.10.0).
Target: native Rust/GTK 0.11.0. The optional QML launcher and original GLSL
remain ordinary assets; the player does not import or execute Python.

Status: the active player, build, installation and CI paths are native Rust.
63 tracked Python/reference packaging and test files were retired in `0a66652`.
Final-tree verification has passed. The independent whole-branch review
identified three issues; all three have reproducing regressions and fixes.
This report does not claim a real GNOME session or an unrun hosted CI job
has passed.

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
| Native install/update/remove and path escaping | Eight isolated installer tests plus desktop ownership/escaping tests and actual setup/remove scripts in a temporary home |
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

Retirement commit `0a66652` passed the full checks. The final review fixes
were then verified again, with these results:

| Check | Result |
| --- | --- |
| Tracked Python application/build files | `git ls-files '*.py' pyproject.toml` is empty; no interpreter invocation in Cargo, native source, setup/launch/remove scripts or CI. Legacy names identify managed installations only. |
| Formatting and lint | `cargo fmt --all --check` and strict all-target Clippy pass. |
| Ordinary suite | 80 passed, 8 native cases deliberately ignored by the ordinary command. |
| Native audio | All 9 audio cases pass, including the 5 ignored output-device cases. |
| GPU | All nine views, 64 switches, cover clearing, waveform alpha/color, particle bounds, cleanup and painted Cairo fallback pass on AMD Radeon. |
| GUI | Production workflow case passes on GTK 4.22.4 and 4.6.9; each smoke driver checks registration before execution. |
| Compositor | Real Hyprland waveform transparency, opaque spectrum, focus, 640×420 footer/list bounds and 60-track scrolling pass. Green background channel in the final run: waveform 200, spectrum 38. |
| Minimum compiler/runtime API | Rust 1.92.0 all-target build and all 80 ordinary tests pass. Real GTK 4.6.9 player and GPU/fallback cases pass with the loader assertion enabled. |
| Optimized delivery | Native release build and `--version` pass; isolated installer suite includes execution of the installed binary, desktop-file validation, upgrade and removal. |

Together these run 88 distinct Rust cases (80 ordinary, 5 additional audio,
and 3 GTK/GPU/compositor cases). The Python reference's 175 cases are a
separate baseline, not added to the Rust count. Audio and GTK checks run in
separate processes so GTK remains on its initializing thread.

The independent reviewer inspected the baseline, spec, plan, parity reports,
production modules, relevant tests and CI. It found no Critical issues, two
Important issues and one sorting difference initially graded Minor. The first
review attempt failed before inspection because its model hit an account usage
limit; the available model performed the fresh-context review instead.

All findings were reproduced as failures before correction:

- EOF completion counted future silence and could discard scheduled final PCM.
  `audio::output::tests::eof_waits_for_scheduled_last_pcm_in_partial_and_ahead_buffers`
  now proves that completion waits for the final PCM playback timestamp, across
  0/5/40/100-ms latency and several buffers requested ahead of playback.
- A failed first installation could persist an executable before its ownership
  marker, blocking retries. `failed_first_install_and_interrupted_copy_remain_retryable`
  now proves safe retry after a desktop-icon write failure and a copy read failure,
  preserving playlists and unrelated files. Validated ownership is established
  before creating native installation artifacts; upgrades retain atomic copying.
- ASCII-only number recognition changed Unicode filename order. This was
  regraded Important because it affects required parity.
  `natural_order_matches_unicode_decimal_tokens_and_stable_ties` now covers
  Unicode decimal runs, mixed scripts, numeric/text boundaries and stable
  case/leading-zero ties using compiled Unicode 16 data.

The ordinary suite passes 80/80 after the fix pass; native audio passes 9/9.
Strict Clippy and formatting pass. No second review is substituted for the
regressions and full-suite checks. No review findings remain deferred.

## Implementation decisions

These decisions preserve the approved scope. Each lists the cost of changing
it if later evidence shows it was wrong.

- Work on the requested development checkout with incremental commits and retain the Python reference through retirement verification; moving the migration commits to another branch would be the correction.
- Share bounded subprocess capture between ffprobe and artwork; splitting that module would be the correction.
- Pass model generation and initial pause intent to `AudioPlayer::play`, sharing `TrackInfo` metadata; changing API call sites would be the correction.
- Request supported 20-ms callback buffers after observing two-second backend defaults; device-specific buffer retuning would be the correction.
- Pass frame, waveform, artwork, gallery and bounds in a shared rendering `Scene`; changing the two renderer signatures would be the correction.
- Separate GTK canvas lifecycle and private GL tests into their own modules; moving two modules would be the correction.
- Preserve the reference's text-only player with keyboard seeking/volume and transport displays; adding widgets would be the correction.
- Keep ownership and atomic installer operations in Rust behind `--install`/`--uninstall`; removing two switches and splitting the module would be the correction.
- Use dedicated folder scan/result/cancellation variants and an owned `FolderListing` so replacement and additions cannot interfere; changing those contracts at their consumers would be the correction.
- Use the available reviewer after the preferred review model failed before inspection; another independent review before integration would be the correction if that review proves insufficient.
- Regrade Unicode decimal ordering as Important and preserve reference behavior; the cost is modest sorting allocations on the filesystem worker.
- Retain reference failure behavior for dangling symlinks and non-UTF-8 saved paths; defining a new recovery policy or persistence format would be the correction.
- Retain the synchronous core playlist helper because production availability checks run on the bounded worker; removing the unused action would be the correction.
- Retain the explicit GNOME/full-Ubuntu/hosted-CI validation limits rather than infer results from other environments; validation and possible platform fixes before wider distribution would be the correction.

## Particle Dance visibility follow-up

The reported faint stream reproduced with production analyzer uniforms: only
1,007 blue/violet pixels exceeded the readable-color threshold at 640×420.
The shader's 0.10–0.24 opacity range suppressed most tiny sprite fragments.
Increasing source-over opacity makes 5,895 stream pixels readable in the same
GPU case, while retaining 2.6-pixel sprites, the palette and bounded blending.
The regression fails against the old shader and passes against the fix. It
also exercises realistic accelerated phase/velocity and checks the broad gold
ring separately. Existing maximum-input, no-white-out, isolated-dot-size and
all-nine-mode GPU/fallback checks continue to pass. The ordinary 80-test suite,
strict Clippy and release build pass. Optional GPU PNG artifacts use
`OMAMUSI_RENDER_ARTIFACTS`, shared with the software image checks.

## Compact now-playing layout follow-up

When navigation faded, the invisible footer still reserved its height, leaving
playback information 149 pixels above the bottom in the GUI regression.
The faded footer and status now release their layout space; title, optional
artist/album and playback time sit above the normal 24-pixel bottom inset.
Interaction restores the footer and the full layout. Stationary-pointer motion
replayed during GTK reallocation does not count as activity, preventing the
compact layout from immediately reopening itself. The production GUI regression
failed before the fix and passes afterward, including restoration. A real
Hyprland capture and geometry check verifies the compact layout at 640×420 with
60 tracks; ordinary tests and strict Clippy pass.
