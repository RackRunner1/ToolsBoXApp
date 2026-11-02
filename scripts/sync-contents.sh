#!/usr/bin/env bash
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

SYNC_DIRS=(.assets public tools ui)
SYNC_FILES=(index.html style.css)
EXCLUDE_DIRS=(og .github scripts pages)
EXCLUDE_FILES=(sitemap.xml robots.txt llms.txt README.md LICENSE CONTRIBUTING.md .gitignore edition.txt '**/apple-touch-icon.png')

exclude_files() {
  local pattern file
  for pattern in "$@"; do
    if [[ "$pattern" == '**/'* ]]; then
      while IFS= read -r -d '' file; do
        rm -f "$file"
        echo "  (excluded) ${file#"$DEST"/}"
      done < <(find "$DEST" -type f -name "${pattern##*/}" -print0)
      continue
    fi
    for file in "$DEST"/$pattern; do
      [ -e "$file" ] || continue
      rm -rf "$file"
      echo "  (excluded) ${file#"$DEST"/}"
    done
  done
}

mkdir -p "$DEST"

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

cleanup() { [ -n "$TMP" ] && rm -rf "$TMP"; return 0; }
trap cleanup EXIT

SHA="unknown"
if [ -d "$SRC/.git" ]; then
  SHA="$(git -C "$SRC" rev-parse HEAD 2>/dev/null | tr -d '[:space:]')" || SHA="unknown"
  [ -z "$SHA" ] && SHA="unknown"
fi
echo "Source commit: $SHA"

for d in "${SYNC_DIRS[@]}"; do
  if [ ! -d "$SRC/$d" ]; then
    echo "  (folder missing from source, skipped) $d"
    continue
  fi
  mkdir -p "$DEST/$d"
  rsync -a --delete --exclude='.gitkeep' "$SRC/$d/" "$DEST/$d/"
done

for f in "${SYNC_FILES[@]}"; do
  if [ -f "$SRC/$f" ]; then
    cp -f "$SRC/$f" "$DEST/$f"
  else
    echo "  (file missing from source, skipped) $f"
  fi
done

for d in "${EXCLUDE_DIRS[@]}"; do
  if [ -e "$DEST/$d" ]; then
    rm -rf "$DEST/$d"
    echo "  (excluded) $d"
  fi
done
exclude_files "${EXCLUDE_FILES[@]}"

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
