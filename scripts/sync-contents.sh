#!/usr/bin/env bash
# Syncs the contents/ folder from ToolsBoXMachine.
#
# Fetches the source repository content (shallow clone, or a local clone
# through --source-path) then mirrors the known folders/files into contents/
# (files deleted upstream are removed locally).
# Web-only extras (SEO/crawler files, OG images, repo meta files) are purged
# after the sync so they never ship in the desktop app.
# Writes contents/.sync-info.json (source SHA + timestamp, only rewritten
# when the source changed, to avoid a daily commit with no changes).
#
# Usage:
#   ./scripts/sync-contents.sh [--source-repo URL] [--branch BRANCH]
#                              [--dest DIR] [--source-path DIR]
set -euo pipefail

SOURCE_REPO="https://github.com/RackRunner1/ToolsBoXMachine.git"
BRANCH="main"
DEST=""
SOURCE_PATH=""

while [ $# -gt 0 ]; do
  case "$1" in
    --source-repo) SOURCE_REPO="$2"; shift 2 ;;
    --branch)      BRANCH="$2"; shift 2 ;;
    --dest)        DEST="$2"; shift 2 ;;
    --source-path) SOURCE_PATH="$2"; shift 2 ;;
    *) echo "Unknown argument: $1" >&2; exit 2 ;;
  esac
done

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
[ -z "$DEST" ] && DEST="$REPO_ROOT/contents"

# Folders synced as mirrors
SYNC_DIRS=".assets pages public tools ui"
# Files synced at the root
SYNC_FILES="index.html style.css"
# Never shipped in contents/ (web-only or repo-meta extras): purged after
# the sync, including leftovers from previous syncs.
EXCLUDE_DIRS="og .github scripts"
EXCLUDE_FILES="sitemap.xml robots.txt llms.txt README.md LICENSE CONTRIBUTING.md .gitignore"

mkdir -p "$DEST"

# ---------------------------------------------------------------------------
# 1. Get the source
# ---------------------------------------------------------------------------
TMP=""
if [ -n "$SOURCE_PATH" ]; then
  [ -f "$SOURCE_PATH/index.html" ] || { echo "Invalid --source-path: index.html is missing from '$SOURCE_PATH'." >&2; exit 1; }
  SRC="$(cd "$SOURCE_PATH" && pwd)"
  echo "Local source: $SRC"
else
  TMP="$(mktemp -d "${TMPDIR:-/tmp}/tbxm-src-XXXXXX")"
  echo "Cloning $SOURCE_REPO ($BRANCH) -> $TMP"
  git clone --depth 1 --branch "$BRANCH" "$SOURCE_REPO" "$TMP" >/dev/null 2>&1 \
    || { echo "git clone failed." >&2; rm -rf "$TMP"; exit 1; }
  [ -f "$TMP/index.html" ] || { echo "The clone looks invalid: index.html is missing." >&2; rm -rf "$TMP"; exit 1; }
  SRC="$TMP"
fi

cleanup() { [ -n "$TMP" ] && rm -rf "$TMP"; }
trap cleanup EXIT

# ---------------------------------------------------------------------------
# 2. Source SHA
# ---------------------------------------------------------------------------
SHA="unknown"
if [ -d "$SRC/.git" ]; then
  SHA="$(git -C "$SRC" rev-parse HEAD 2>/dev/null | tr -d '[:space:]')" || SHA="unknown"
  [ -z "$SHA" ] && SHA="unknown"
fi
echo "Source commit: $SHA"

# ---------------------------------------------------------------------------
# 3. Mirror the folders
# ---------------------------------------------------------------------------
for d in $SYNC_DIRS; do
  if [ ! -d "$SRC/$d" ]; then
    echo "  (folder missing from source, skipped) $d"
    continue
  fi
  mkdir -p "$DEST/$d"
  rsync -a --delete --exclude='.gitkeep' "$SRC/$d/" "$DEST/$d/"
done

# ---------------------------------------------------------------------------
# 4. Root files
# ---------------------------------------------------------------------------
for f in $SYNC_FILES; do
  if [ -f "$SRC/$f" ]; then
    cp -f "$SRC/$f" "$DEST/$f"
  else
    echo "  (file missing from source, skipped) $f"
  fi
done

# ---------------------------------------------------------------------------
# 5. Purge web-only extras (SEO/crawler files, OG images, repo meta)
# ---------------------------------------------------------------------------
for d in $EXCLUDE_DIRS; do
  if [ -e "$DEST/$d" ]; then
    rm -rf "$DEST/$d"
    echo "  (excluded) $d"
  fi
done
for f in $EXCLUDE_FILES; do
  if [ -e "$DEST/$f" ]; then
    rm -f "$DEST/$f"
    echo "  (excluded) $f"
  fi
done

# ---------------------------------------------------------------------------
# 6. Sync metadata (idempotent)
# ---------------------------------------------------------------------------
INFO_PATH="$DEST/.sync-info.json"
UNCHANGED=0
if [ -f "$INFO_PATH" ]; then
  EXISTING_SHA="$(python3 -c "import json,sys; print(json.load(open('$INFO_PATH')).get('sourceSha',''))" 2>/dev/null || true)"
  EXISTING_BRANCH="$(python3 -c "import json,sys; print(json.load(open('$INFO_PATH')).get('branch',''))" 2>/dev/null || true)"
  EXISTING_REPO="$(python3 -c "import json,sys; print(json.load(open('$INFO_PATH')).get('sourceRepo',''))" 2>/dev/null || true)"
  if [ "$EXISTING_SHA" = "$SHA" ] && [ "$EXISTING_BRANCH" = "$BRANCH" ] && [ "$EXISTING_REPO" = "$SOURCE_REPO" ]; then
    UNCHANGED=1
  fi
fi

if [ "$UNCHANGED" -eq 1 ]; then
  echo ".sync-info.json unchanged (same source commit)."
else
  if [ -n "$SOURCE_PATH" ]; then
    RESOLVED_PATH="$(cd "$SOURCE_PATH" && pwd)"
    SOURCE_PATH_JSON="\"$RESOLVED_PATH\""
  else
    SOURCE_PATH_JSON="null"
  fi
  SYNCED_AT="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  cat > "$INFO_PATH" <<EOF
{"source": "RackRunner1/ToolsBoXMachine", "sourceRepo": "$SOURCE_REPO", "sourcePath": $SOURCE_PATH_JSON, "sourceSha": "$SHA", "branch": "$BRANCH", "syncedAtUtc": "$SYNCED_AT"}
EOF
fi

FILE_COUNT="$(find "$DEST" -type f | wc -l | tr -d ' ')"
echo "OK: contents/ synced ($FILE_COUNT files, source @ $SHA)"
