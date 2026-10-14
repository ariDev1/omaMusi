#!/usr/bin/env bash
set -euo pipefail
data_dir="${XDG_DATA_HOME:-$HOME/.local/share}"
if [[ "$data_dir" != /* ]]; then data_dir="$HOME/.local/share"; fi
player="$data_dir/omamusi/bin/omaMusi"
install_dir="$data_dir/omamusi"
marker="$install_dir/.omamusi-managed"
if [[ -e "$install_dir" || -L "$install_dir" ]]; then
  if [[ -L "$install_dir" || -L "$install_dir/bin" || -L "$marker" || ! -f "$marker" ]] ||
     [[ $(cat -- "$marker") != io.github.aridev1.omamusi ]]; then
    printf 'omaMusi: refusing unmanaged installation: %s\n' "$install_dir" >&2
    exit 1
  fi
fi
if [[ -x "$player" && ! -L "$player" ]]; then
  exec "$player" --uninstall
fi
printf 'omaMusi native executable is absent; no user data was removed.\n'
