#!/usr/bin/env bash
# Generates the Tauri app icons with `tauri icon`.
#
# Source: contents/.assets/logo/logo.png (the synced content logo),
# falling back to fav.png. When no source exists, a fallback icon is drawn
# (requires python3 + Pillow) so the build never breaks.
# Output: app/src-tauri/icons/
#
# Requires node_modules in app/ (run `npm install` in app/ first).
#
# Usage:
#   ./scripts/generate-icons.sh [--source FILE]
set -euo pipefail

SOURCE=""

while [ $# -gt 0 ]; do
  case "$1" in
    --source) SOURCE="$2"; shift 2 ;;
    *) echo "Unknown argument: $1" >&2; exit 2 ;;
  esac
done

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if [ -z "$SOURCE" ]; then
  for candidate in "$REPO_ROOT/contents/.assets/logo/logo.png" "$REPO_ROOT/contents/.assets/logo/fav.png"; do
    if [ -f "$candidate" ]; then SOURCE="$candidate"; break; fi
  done
fi

if [ -z "$SOURCE" ] || [ ! -f "$SOURCE" ]; then
  echo "No logo found in contents/.assets/logo/ - drawing a fallback icon."
  SOURCE="$REPO_ROOT/app/src-tauri/fallback-logo.png"
  python3 - "$SOURCE" <<'PYEOF'
import sys
try:
    from PIL import Image, ImageDraw, ImageFont
except ImportError:
    print("Pillow is required for the fallback icon (pip install pillow).", file=sys.stderr)
    sys.exit(1)
out = sys.argv[1]
img = Image.new("RGBA", (512, 512), (0, 0, 0, 0))
draw = ImageDraw.Draw(img)
draw.rounded_rectangle([8, 8, 504, 504], radius=110, fill=(37, 99, 235, 255))
try:
    font = ImageFont.truetype("DejaVuSans-Bold.ttf", 300)
except OSError:
    font = ImageFont.load_default()
text = "T"
bbox = draw.textbbox((0, 0), text, font=font)
w, h = bbox[2] - bbox[0], bbox[3] - bbox[1]
draw.text(((512 - w) / 2 - bbox[0], (512 - h) / 2 - bbox[1]), text, font=font, fill=(255, 255, 255, 255))
img.save(out)
print("Fallback logo drawn:", out)
PYEOF
fi

echo "Icon source: $SOURCE"
# The content logo is usually wide (not square): --fit contain scales it
# inside the square canvas with transparent padding instead of cropping.
(cd "$REPO_ROOT/app" && npx tauri icon "$SOURCE" --fit contain)
echo "OK: icons generated in $REPO_ROOT/app/src-tauri/icons"
ls -la "$REPO_ROOT/app/src-tauri/icons"
