#!/usr/bin/env bash
# Prepares the web assets embedded in the Tauri app.
#
# Copies contents/ to app/web/ (the frontendDist configured in
# app/src-tauri/tauri.conf.json). Every file found there is embedded into
# the binary at build time: the produced binary is standalone.
# At the end, scripts/apply-app-changes.sh enforces the forced app-only
# changes (e.g. hiding the "App" button).
#
# Run after scripts/sync-contents.sh and before `tauri build`.
#
# Usage:
#   ./scripts/prepare-web.sh [--contents-dir DIR] [--web-dir DIR]
set -euo pipefail

CONTENTS_DIR=""
WEB_DIR=""

while [ $# -gt 0 ]; do
  case "$1" in
    --contents-dir) CONTENTS_DIR="$2"; shift 2 ;;
    --web-dir)      WEB_DIR="$2"; shift 2 ;;
    *) echo "Unknown argument: $1" >&2; exit 2 ;;
  esac
done

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
[ -z "$CONTENTS_DIR" ] && CONTENTS_DIR="$REPO_ROOT/contents"
[ -z "$WEB_DIR" ] && WEB_DIR="$REPO_ROOT/app/web"

[ -f "$CONTENTS_DIR/index.html" ] || { echo "index.html is missing from '$CONTENTS_DIR' - run first: ./scripts/sync-contents.sh" >&2; exit 1; }

# ---------------------------------------------------------------------------
# Full rebuild of app/web/
# ---------------------------------------------------------------------------
rm -rf "$WEB_DIR"
mkdir -p "$WEB_DIR"
cp -a "$CONTENTS_DIR/." "$WEB_DIR/"

# ---------------------------------------------------------------------------
# Minimal checks
# ---------------------------------------------------------------------------
for f in index.html style.css; do
  [ -f "$WEB_DIR/$f" ] || { echo "$f is missing from app/web/ - incomplete content?" >&2; exit 1; }
done

FILE_COUNT="$(find "$WEB_DIR" -type f | wc -l | tr -d ' ')"
SIZE_MB="$(du -sm "$WEB_DIR" | cut -f1)"
echo "OK: app/web/ ready ($FILE_COUNT files, ${SIZE_MB} MB) - embedded into the binary at build time"

# ---------------------------------------------------------------------------
# Forced app-only changes
# ---------------------------------------------------------------------------
bash "$REPO_ROOT/scripts/apply-app-changes.sh" --web-dir "$WEB_DIR"
