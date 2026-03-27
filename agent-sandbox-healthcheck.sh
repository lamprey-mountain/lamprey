#!/usr/bin/env bash
set -euo pipefail

WARN=0

check_mount() {
  local path="$1"
  local name="$2"

  if ! mountpoint -q "$path"; then
    echo "WARNING: $path is not a virtiofs mount ($name cache) - falling back to local disk inside VM."
    echo "         Set \$$name on the host and re-run spawn-sandbox for persistent caching."
    WARN=1
  fi
}

check_mount /cache/sccache    "SCCACHE_DIR"
check_mount /cache/cargo-home "CARGO_HOME"
check_mount /cache/pnpm-store "PNPM_STORE"

if [ $WARN -eq 1 ]; then
  echo "--- Cache warning: builds will work but artifacts won't persist across VM restarts ---"
  sleep 2  # make sure it's visible before setup output floods the console
fi
