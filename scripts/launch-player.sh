#!/usr/bin/env bash
set -euo pipefail

source_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
player="$HOME/.local/bin/omaMusi"
if [[ ! -x "$player" ]]; then
  message="Install the player first: bash \"$source_dir/scripts/setup-player.sh\""
  printf 'omaMusi: %s\n' "$message" >&2
  if command -v notify-send >/dev/null; then
    notify-send -- "omaMusi setup required" "$message" || true
  fi
  exit 1
fi
music_dir="$HOME/Music"
if command -v xdg-user-dir >/dev/null; then
  configured_music=$(xdg-user-dir MUSIC 2>/dev/null || true)
  if [[ -n "$configured_music" && "$configured_music" != "$HOME" && -d "$configured_music" ]]; then
    music_dir="$configured_music"
  fi
fi
if [[ -d "$music_dir" ]]; then
  exec "$player" --view "event horizon" "$music_dir" --recursive
fi
# An explicit empty directory avoids the player's default cwd scan.
empty_dir=$(mktemp -d "${TMPDIR:-/tmp}/omamusi-empty-XXXXXXXX")
trap 'rmdir -- "$empty_dir"' EXIT
"$player" --view "event horizon" "$empty_dir"
