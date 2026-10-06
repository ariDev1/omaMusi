#!/usr/bin/env bash
set -euo pipefail

install_dir="${XDG_DATA_HOME:-$HOME/.local/share}/omamusi"
launcher="$HOME/.local/bin/omaMusi"
if [[ ! -e "$install_dir" && ! -L "$install_dir" ]]; then
  if [[ -L "$launcher" && $(readlink -- "$launcher") == "$install_dir/venv/bin/omaMusi" ]]; then
    rm -- "$launcher"
  fi
  printf 'omaMusi managed installation is already removed.\n'
  exit 0
fi
if [[ -L "$install_dir" || ! -f "$install_dir/.omamusi-managed" ]] ||
   [[ $(cat -- "$install_dir/.omamusi-managed") != io.github.aridev1.omamusi ]]; then
  printf 'omaMusi: refusing to remove unmanaged installation: %s\n' "$install_dir" >&2
  exit 1
fi
if [[ -L "$launcher" && $(readlink -- "$launcher") == "$install_dir/venv/bin/omaMusi" ]]; then
  rm -- "$launcher"
fi
# The directory also holds persistent user data. The marker remains so
# subsequent setup/removal can recognize ownership without adopting other files.
rm -rf -- "$install_dir/venv"
printf 'Removed managed player installation. Saved playlists and music are preserved.\n'
