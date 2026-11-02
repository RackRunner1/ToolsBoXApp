#!/usr/bin/env bash
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

rm -rf "$WEB_DIR"
mkdir -p "$WEB_DIR"
cp -a "$CONTENTS_DIR/." "$WEB_DIR/"

for f in index.html style.css; do
  [ -f "$WEB_DIR/$f" ] || { echo "$f is missing from app/web/ - incomplete content?" >&2; exit 1; }
done

FILE_COUNT="$(find "$WEB_DIR" -type f | wc -l | tr -d ' ')"
SIZE_MB="$(du -sm "$WEB_DIR" | cut -f1)"
echo "OK: app/web/ ready ($FILE_COUNT files, ${SIZE_MB} MB) - embedded into the binary at build time"

bash "$REPO_ROOT/scripts/apply-app-changes.sh" --web-dir "$WEB_DIR"
