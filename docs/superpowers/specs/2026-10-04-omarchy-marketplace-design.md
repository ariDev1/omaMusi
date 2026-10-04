# omaMusi Omarchy marketplace preparation

## Purpose

Make omaMusi discoverable and launchable through the Omarchy plugin marketplace while preserving the standalone Python player. Use an Event Horizon logo and MIT licensing, as requested by the owner.

## Plugin

- One root `manifest.json`, version `0.9.1`, MIT license, author `ariDev1`, permanent ID `io.github.aridev1.omamusi` (check marketplace availability before publication).
- Kind `bar-widget`, with a QML entry point using the installed Omarchy shell widget contract.
- A small Event Horizon icon and tooltip identifying omaMusi. Left click launches the player with Event Horizon selected and the user's Music directory when available; otherwise launch with an empty playlist, rather than scanning the plugin checkout.
- Use the actual Event Horizon rendering for the logo and the existing screenshot for a root marketplace preview. No new image-generation dependency.
- The plugin launches a separately installed player. Loading or enabling the widget must not install dependencies, run pip, or change desktop settings.
- If the player is missing, show an actionable setup message rather than silently failing.

## Installation and removal

- Provide an explicit user-run setup script for a dedicated virtual environment under the user's local data directory, installing from the checked-out plugin source, and a launcher under the user's local bin directory.
- Check Python, FFmpeg/ffprobe, and source availability. Document dependency downloads, Python dependencies, OpenGL requirements, and the need to rerun setup after plugin updates.
- Do not overwrite an unrelated existing launcher. Repeated setup should update the managed installation safely.
- Provide removal instructions for both the plugin and its managed player installation. Preserve music and other user files.
- Add a root MIT license with copyright attributed to ariDev1, and package license metadata.

## Documentation and publication

Update README with install, use, update, removal, licensing, and dependency details. Prepare a marketplace submission draft with category `Widgets` and tags `media, launcher`.

Preparation does not include submitting an issue to the marketplace. Before submission, the owner must approve the completed title/body and confirm the submission checklist, including ownership of code and preview assets, as required by the marketplace submission guide.

## Validation

Validate the root manifest with `omarchy plugin validate`, check QML against installed shell imports, exercise setup/removal in isolated temporary directories, and verify launcher arguments and missing-player behavior. Run the existing Python tests. A real shell test must check icon visibility, clicking, disable/re-enable, restart, and removal before declaring marketplace readiness.

## Existing work

Preserve the uncommitted subtle Event Horizon camera movement. Do not silently discard or publish it during marketplace preparation.
