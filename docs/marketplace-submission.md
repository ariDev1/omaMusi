# Marketplace submission draft

Status: preparation only. Do not submit until the owner confirms every checklist statement and explicitly approves the completed issue. The plugin files must first be published on the repository's default branch; record that full commit SHA for the review.

Plugin ID: `io.github.aridev1.omamusi` (no exact match in the marketplace registry checked on 2026-10-04).

Title: `[Plugin]: omaMusi`

## Issue body

### Repository URL

https://github.com/ariDev1/omaMusi

### Category

Widgets

### Tags

media, launcher

### Suggest a missing tag

_No response_

### Maintainer notes

An Event Horizon bar icon launches the standalone omaMusi local music player. The widget does not install or download software on load. Explicit manual setup uses Python venv and pip to install the player and documented dependencies from the plugin checkout. Python 3.11+, FFmpeg/ffprobe, and desktop audio are required; full visuals require OpenGL 3.3+. The root README includes installation, update, and removal instructions. Code and the GPU-rendered Event Horizon logo are licensed under MIT. This listing requires manual player setup.

### Submission checklist

- [ ] The repository is public and contains installation and removal instructions.
- [ ] I have documented the plugin license and any external dependencies.
- [ ] I confirm that I own or have permission to submit this plugin and its preview assets.
- [ ] The plugin does not overwrite user configuration without explicit consent.
- [ ] I understand that approval is for listing and is not a security review.

## References

- https://plugins.omarchy.org/publish.html
- https://github.com/omacom/omarchy-plugin-marketplace/blob/main/SUBMISSION.md

Before submission, check all five boxes only after owner confirmation. Marketplace submission requires the exact headings and checklist above. A real shell check must cover click, missing-player notification, disable/re-enable, restart, and removal.
