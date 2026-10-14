#!/usr/bin/env bash
# Run inside xvfb-run + dbus-run-session on CI; desktop checks use muted audio.
set -euo pipefail
export XDG_RUNTIME_DIR="${RUNNER_TEMP:-/tmp}/omamusi-ci-runtime"
mkdir -p "$XDG_RUNTIME_DIR"
chmod 700 "$XDG_RUNTIME_DIR"
export OMAMUSI_ALLOW_SOFTWARE_GL=1
export LIBGL_ALWAYS_SOFTWARE=1
export GDK_BACKEND=x11
pulseaudio --start --exit-idle-time=-1
pactl load-module module-null-sink sink_name=omamusi_test rate=48000
pactl set-default-sink omamusi_test
openbox >/dev/null 2>&1 &
window_manager_pid=$!
trap 'kill "$window_manager_pid" 2>/dev/null || true; pulseaudio --kill || true' EXIT
cargo test --locked --test audio -- --include-ignored --test-threads=1
cargo run --locked --bin render-smoke
cargo run --locked --bin player-smoke
