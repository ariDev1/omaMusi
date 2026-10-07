# Changelog

## Unreleased

- Added Cover Gallery, a separate visual with gently changing album-cover tiles. Matching artwork groups songs together; clicking a tile plays a random song from the group, and hovering keeps that tile still.
- Gallery artwork loads in the background with bounded thumbnails and protection against results from a replaced music collection. The layout adapts to window size and leaves room for controls.
- Added a user-local application-menu launcher and Event Horizon icon. It opens the configured Music folder recursively, or an empty player when that folder is unavailable.
- Omarchy setup and removal manage the menu entry automatically; standalone installations can add or remove it with `python -m omamusi.desktop install` or `remove`.

## 0.10.0

- Starting in a music folder includes all nested subfolders. Search matches filenames and folder names without regard to case; Up/Down moves from search to matching results, and Enter plays the selection.
- Track paths distinguish duplicate filenames, and the footer groups all keyboard shortcuts by function.
- Cover Art shows one sharp, music-reactive album cover on the right against a black background, using embedded artwork or local cover images.
- Folder discovery, metadata, and artwork load in the background, with cancellation and stale-result protection.
- Fixed playlist preservation after failed scans, stale audio events, GPU cleanup, and CPU rendering without an Omarchy theme.
- Added regression and wheel-build CI for minimum supported and current Python/Qt dependencies.
- Repeat launches restore and focus the running player instead of opening another instance.
- **R** toggles random playback while preserving previous-track history.
- **A** adds tracks to named saved playlists; **B** browses and manages playlists.
- Waveform uses a colored persistence map with a transparent background, with track changes and seeking clearing the map.
- Package license metadata uses the MIT SPDX identifier and declares the license file explicitly.
- Fixed audio module imports on PySide6 6.7 when Qt's legacy enum aliases have not been loaded.

## 0.9.1

- Added the version and Git revision footer.
