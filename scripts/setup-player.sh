#!/usr/bin/env bash
set -euo pipefail

source_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
install_dir="${XDG_DATA_HOME:-$HOME/.local/share}/omamusi"
launcher="$HOME/.local/bin/omaMusi"
marker="$install_dir/.omamusi-managed"
fail() { printf 'omaMusi: %s\n' "$*" >&2; exit 1; }

[[ -f "$source_dir/pyproject.toml" ]] || fail "Player source is missing."
for dependency in python ffmpeg ffprobe; do
  command -v "$dependency" >/dev/null || fail "Missing $dependency. Install Python and FFmpeg first: omarchy pkg add python ffmpeg"
done
if [[ -e "$launcher" || -L "$launcher" ]]; then
  [[ -L "$launcher" && $(readlink -- "$launcher") == "$install_dir/venv/bin/omaMusi" && -f "$marker" ]] || fail "$launcher already exists and is not managed by this installer."
fi
if [[ -e "$install_dir" || -L "$install_dir" ]]; then
  [[ ! -L "$install_dir" && -d "$install_dir" && -f "$marker" && $(cat -- "$marker") == io.github.aridev1.omamusi ]] || fail "Refusing to modify an unmanaged installation at $install_dir."
fi
mkdir -p -- "$install_dir" "$(dirname -- "$launcher")"
printf 'io.github.aridev1.omamusi\n' > "$marker"
python -m venv "$install_dir/venv"
"$install_dir/venv/bin/python" -m pip install --upgrade "$source_dir"
[[ -x "$install_dir/venv/bin/omaMusi" ]] || fail "Installation did not produce the player executable."
ln -sfn -- "$install_dir/venv/bin/omaMusi" "$launcher"
printf 'Installed omaMusi. Launch with %s\n' "$launcher"
