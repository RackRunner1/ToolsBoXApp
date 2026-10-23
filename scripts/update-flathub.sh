#!/usr/bin/env bash
# Updates the Flathub repository (flathub/io.github.RackRunner1.ToolsBoXApp)
# with the latest tag and commit for automated daily releases.
#
# Usage:
#   ./scripts/update-flathub.sh [--tag TAG] [--commit SHA] [--token GITHUB_TOKEN] [--push]
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP_ID="io.github.RackRunner1.ToolsBoXApp"
FLATHUB_REPO="flathub/$APP_ID"

TAG=""
COMMIT=""
TOKEN="${GITHUB_TOKEN:-}"
DO_PUSH=false

while [ $# -gt 0 ]; do
  case "$1" in
    --tag) TAG="$2"; shift 2 ;;
    --commit) COMMIT="$2"; shift 2 ;;
    --token) TOKEN="$2"; shift 2 ;;
    --push) DO_PUSH=true; shift ;;
    -h|--help)
      echo "Usage: $(basename "$0") [--tag TAG] [--commit SHA] [--token GITHUB_TOKEN] [--push]"
      exit 0
      ;;
    *) echo "Unknown argument: $1" >&2; exit 2 ;;
  esac
done

if [ -z "$TAG" ]; then
  TAG="$(curl -fsSL https://api.github.com/repos/RackRunner1/ToolsBoXApp/releases/latest | python3 -c "import json,sys; print(json.load(sys.stdin).get('tag_name',''))")"
fi

if [ -z "$COMMIT" ]; then
  COMMIT="$(git rev-parse HEAD)"
fi

echo "Flathub update target: App ID=$APP_ID, Tag=$TAG, Commit=$COMMIT"

AUTH_HEADER=()
if [ -n "$TOKEN" ]; then
  AUTH_HEADER=(-H "Authorization: Bearer $TOKEN")
fi

HTTP_STATUS="$(curl -s -o /dev/null -w "%{http_code}" "${AUTH_HEADER[@]}" "https://api.github.com/repos/$FLATHUB_REPO" || true)"

if [ "$HTTP_STATUS" != "200" ]; then
  echo "Repository $FLATHUB_REPO is not yet active on Flathub (HTTP $HTTP_STATUS). Initial submission PR may be pending review."
  echo "Skipping Flathub push."
  exit 0
fi

WORKDIR="$(mktemp -d)"
trap 'rm -rf "$WORKDIR"' EXIT

CLONE_URL="https://github.com/$FLATHUB_REPO.git"
if [ -n "$TOKEN" ]; then
  CLONE_URL="https://x-access-token:${TOKEN}@github.com/$FLATHUB_REPO.git"
fi

echo "Cloning $FLATHUB_REPO..."
git clone --depth 1 "$CLONE_URL" "$WORKDIR/flathub-repo"

MANIFEST="$WORKDIR/flathub-repo/$APP_ID.yml"
if [ ! -f "$MANIFEST" ]; then
  MANIFEST="$WORKDIR/flathub-repo/$APP_ID.json"
fi

[ -f "$MANIFEST" ] || { echo "Manifest not found in $FLATHUB_REPO" >&2; exit 1; }

python3 - "$MANIFEST" "$TAG" "$COMMIT" <<'PYEOF'
import sys, re
manifest_path, tag, commit = sys.argv[1], sys.argv[2], sys.argv[3]
with open(manifest_path, "r", encoding="utf-8") as f:
    content = f.read()

content = re.sub(r'tag:\s*["\']?[^"\'\n]+["\']?', f'tag: {tag}', content)
content = re.sub(r'commit:\s*["\']?[^"\'\n]+["\']?', f'commit: {commit}', content)

with open(manifest_path, "w", encoding="utf-8") as f:
    f.write(content)
print(f"Updated {manifest_path} to tag {tag} and commit {commit}")
PYEOF

if [ -f "$REPO_ROOT/flatpak/cargo-sources.json" ]; then
  cp "$REPO_ROOT/flatpak/cargo-sources.json" "$WORKDIR/flathub-repo/cargo-sources.json"
fi

if [ "$DO_PUSH" = true ]; then
  echo "Pushing update to $FLATHUB_REPO..."
  git -C "$WORKDIR/flathub-repo" config user.name "github-actions[bot]"
  git -C "$WORKDIR/flathub-repo" config user.email "41898282+github-actions[bot]@users.noreply.github.com"
  git -C "$WORKDIR/flathub-repo" add -A
  if git -C "$WORKDIR/flathub-repo" diff --cached --quiet; then
    echo "No changes needed in Flathub manifest."
    exit 0
  fi
  git -C "$WORKDIR/flathub-repo" commit -m "Update to version $TAG"
  git -C "$WORKDIR/flathub-repo" push origin master
  echo "Successfully pushed update to $FLATHUB_REPO!"
else
  echo "Dry run complete (pass --push to commit and push)."
fi
