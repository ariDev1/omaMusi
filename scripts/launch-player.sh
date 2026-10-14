#!/usr/bin/env bash
set -euo pipefail
source_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
player="$HOME/.local/bin/omaMusi"
if [[ ! -x "$player" ]]; then
  message="Install the player first: bash \"$source_dir/scripts/setup-player.sh\""
  printf 'omaMusi: %s\n' "$message" >&2
  if command -v notify-send >/dev/null; then notify-send -- "omaMusi setup required" "$message" || true; fi
  exit 1
fi
exec "$player" --desktop launch
