#!/usr/bin/env bash
# Generates flatpak cargo-sources.json for ToolsBoXApp.
#
# Requires: python3
# Uses the vendored flatpak-cargo-generator.py or downloads it from flatpak-builder-tools.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUTPUT="${1:-$REPO_ROOT/flatpak/cargo-sources.json}"

mkdir -p "$(dirname "$OUTPUT")"

GENERATOR=""
if [ -f "/tmp/opencode/fbt/cargo/flatpak-cargo-generator.py" ]; then
  GENERATOR="/tmp/opencode/fbt/cargo/flatpak-cargo-generator.py"
else
  CACHE_DIR="${XDG_CACHE_HOME:-$HOME/.cache}/flatpak-cargo-generator"
  mkdir -p "$CACHE_DIR"
  GENERATOR="$CACHE_DIR/flatpak-cargo-generator.py"
  if [ ! -f "$GENERATOR" ]; then
    echo "Downloading flatpak-cargo-generator.py..."
    curl -fsSL "https://raw.githubusercontent.com/flatpak/flatpak-builder-tools/master/cargo/flatpak-cargo-generator.py" -o "$GENERATOR"
  fi
fi

# Ensure python environment has dependencies
VENV_DIR="${XDG_CACHE_HOME:-$HOME/.cache}/fbt-venv"
if [ ! -f "$VENV_DIR/bin/python3" ]; then
  python3 -m venv "$VENV_DIR"
  "$VENV_DIR/bin/pip" install --quiet tomlkit aiohttp pyyaml
fi

echo "Generating cargo-sources.json from app/src-tauri/Cargo.lock..."
"$VENV_DIR/bin/python3" "$GENERATOR" "$REPO_ROOT/app/src-tauri/Cargo.lock" -o "$OUTPUT"
echo "OK: $OUTPUT generated ($(wc -l < "$OUTPUT") lines)"
