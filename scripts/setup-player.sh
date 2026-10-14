#!/usr/bin/env bash
set -euo pipefail
source_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
fail() { printf 'omaMusi: %s\n' "$*" >&2; exit 1; }
for dependency in cargo ffmpeg ffprobe; do
  command -v "$dependency" >/dev/null || fail "Missing $dependency. Install Rust, GTK 4 development libraries and FFmpeg first."
done
exec cargo run --locked --release --manifest-path "$source_dir/Cargo.toml" --bin omaMusi -- --install
