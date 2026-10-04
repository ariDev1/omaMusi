# Omarchy marketplace implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox syntax for tracking.

**Goal:** Prepare a tested omaMusi launcher plugin for marketplace submission.

**Architecture:** A root Omarchy bar-widget manifest loads a small QML launcher. An explicit setup script installs the Python player into a managed user-local environment; the widget never installs software itself.

**Tech Stack:** Python/PySide6, Bash, QtQuick/Quickshell, installed Omarchy shell components.

**Spec:** `docs/superpowers/specs/2026-10-04-omarchy-marketplace-design.md`

## Global constraints

- Version `0.9.1`, license MIT, author `ariDev1`, plugin ID `io.github.aridev1.omamusi`.
- Use an actual Event Horizon rendering for the logo.
- Do not install on plugin load or overwrite unrelated launchers or desktop settings.
- Preserve the existing uncommitted camera movement.
- Prepare a submission draft only; submission requires owner approval of the completed checklist and issue.

## Review focus

- Paths containing spaces: setup and launch must preserve arguments.
- Existing unrelated launcher: setup must refuse to replace it.
- Missing player/dependencies: users must receive an actionable message.
- Missing Music folder: launch an empty playlist, not a plugin directory scan.
- Repeated setup/removal: update managed files safely and preserve unrelated files.

### Task 1: Managed player setup and launcher

**Files:** Create `scripts/setup-player.sh`, `scripts/remove-player.sh`, `scripts/launch-player.sh`, and `tests/test_plugin_install.py`.

**Interfaces:** Setup installs under `${XDG_DATA_HOME:-$HOME/.local/share}/omamusi`, with launcher `${HOME}/.local/bin/omaMusi`; launch script invokes that managed player with `--view "event horizon"` and Music directory when present. Scripts return nonzero on failure.

- [ ] Add behavioral tests using isolated temporary HOME/XDG directories for spaces, conflicting launcher, missing executable dependencies, missing Music directory, and repeated setup/removal. Substitute controlled executables for dependency installation; assert resulting files and launch arguments.
- [ ] Run `.venv/bin/python -m unittest discover -s tests -p test_plugin_install.py`; verify failures before implementation.
- [ ] Implement scripts with quoted paths, explicit dependency checks, managed-install ownership marker, and clear error messages. Missing player triggers a desktop notification with setup instructions when `notify-send` is available, plus stderr.
- [ ] Rerun the focused tests; require all pass. Run `bash -n` on all scripts.

### Task 2: Event Horizon icon and Omarchy bar widget

**Files:** Create `manifest.json`, `BarWidget.qml`, `assets/event-horizon.png`, and `preview.png`.

**Interfaces:** Root manifest declares `bar-widget` with `entryPoints.barWidget: "BarWidget.qml"`; widget calls the Task 1 launch script by absolute resolved local path via Quickshell, without shell-string interpolation.

- [ ] Capture a clean Event Horizon render and produce a small square logo; use the existing screenshot for the marketplace preview.
- [ ] Implement `BarWidget.qml` using `qs.Ui.BarWidget` and `WidgetButton`, with an Image child, `hasVisualContent: true`, a tooltip, and left-click launching. Size the icon to horizontal and vertical bars.
- [ ] Create the manifest with the approved identity, license, version, and default right section.
- [ ] Run `omarchy plugin validate "$PWD"`; require success. Run `qmllint -I /usr/share/omarchy/shell BarWidget.qml`; resolve actionable errors.
- [ ] Test with the installed shell: icon visibility, click, missing-player notification, disable/re-enable, restart, and removal. Record any checks requiring user interaction as outstanding rather than claiming success.

### Task 3: Licensing, documentation, and final verification

**Files:** Create `LICENSE` and `docs/marketplace-submission.md`; modify `README.md` and `pyproject.toml`.

**Interfaces:** Root README documents the scripts and plugin ID from Tasks 1–2. Submission draft uses category `Widgets`, tags `media, launcher`, and the marketplace's exact headings/checklist; owner-dependent checklist confirmations remain pending.

- [ ] Add the standard MIT license with `Copyright (c) 2026 ariDev1` and package license metadata compatible with the declared setuptools floor.
- [ ] Document plugin and player install, update, remove, external dependencies, download behavior, OpenGL requirement, and Event Horizon icon usage.
- [ ] Check plugin ID availability against the current marketplace catalog; document any collision and stop before publishing a conflicting identity.
- [ ] Prepare the completed submission title/body with all six required headings. Do not create the GitHub issue.
- [ ] Run `.venv/bin/python -m unittest discover -s tests`, `.venv/bin/python tests/smoke_gpu.py`, manifest validation, script syntax checks, and `git diff --check`; require success.
- [ ] Review final changes against the spec and report test evidence and any remaining real-shell checks. Preserve the separate camera change; do not publish it incidentally.
