#!/usr/bin/env bash
# Applies the forced app-only changes to the staged web copy:
#   1. Hide the "App" button in the navbar.
#   2. Remove the footer links (About, Philosophy, Privacy Policy,
#      Terms of Service), keeping only the copyright line.
#
# Runs against the staged web copy (app/web/), never against contents/
# (which stays a pristine mirror of ToolsBoXMachine). Called automatically
# at the end of scripts/prepare-web.sh, before `tauri build`.
# Idempotent: running it twice gives the same result.
#
# Usage:
#   ./scripts/apply-app-changes.sh [--web-dir DIR]
set -euo pipefail

WEB_DIR=""

while [ $# -gt 0 ]; do
  case "$1" in
    --web-dir) WEB_DIR="$2"; shift 2 ;;
    *) echo "Unknown argument: $1" >&2; exit 2 ;;
  esac
done

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
[ -z "$WEB_DIR" ] && WEB_DIR="$REPO_ROOT/app/web"

[ -d "$WEB_DIR" ] || { echo "Web dir is missing: '$WEB_DIR' - run first: ./scripts/prepare-web.sh" >&2; exit 1; }
command -v python3 >/dev/null 2>&1 || { echo "python3 is required to apply app changes." >&2; exit 1; }

# ---------------------------------------------------------------------------
# 1. Hide the "App" button in the navbar: inside the installed app, a button
#    advertising the app itself (installation instructions for a product the
#    user already runs) makes no sense. The pages/app.html page is kept.
# ---------------------------------------------------------------------------
WEB_DIR="$WEB_DIR" python3 - <<'PYEOF'
import os
import re

path = os.path.join(os.environ["WEB_DIR"], "ui", "navbar.html")
if not os.path.isfile(path):
    print("(skipped) ui/navbar.html not found in the web copy.")
    raise SystemExit(0)

with open(path, encoding="utf-8") as f:
    raw = f.read()

# Remove the <a ... href="/pages/app.html" ...>...</a> element (any attrs/order).
new = re.sub(
    r'<a\b[^>]*href="/pages/app\.html"[^>]*>.*?</a\s*>',
    "",
    raw,
    flags=re.IGNORECASE | re.DOTALL,
)
# Collapse the blank line left behind.
new = re.sub(r"\n[ \t]*\n([ \t]*</div>)", r"\n\1", new)

if new == raw:
    print("navbar: no App link found, nothing to do.")
else:
    with open(path, "w", encoding="utf-8", newline="") as f:
        f.write(new)
    print("navbar: App link removed.")
PYEOF

# ---------------------------------------------------------------------------
# 2. Remove the footer links, keeping only the copyright line: with a single
#    child left, center the footer content so the line does not stick to the
#    left edge on desktop.
# ---------------------------------------------------------------------------
WEB_DIR="$WEB_DIR" python3 - <<'PYEOF'
import os
import re

path = os.path.join(os.environ["WEB_DIR"], "ui", "footer.html")
if not os.path.isfile(path):
    print("(skipped) ui/footer.html not found in the web copy.")
    raise SystemExit(0)

with open(path, encoding="utf-8") as f:
    raw = f.read()

new = re.sub(
    r'[ \t]*<div\b[^>]*class="footer-links"[^>]*>.*?</div\s*>',
    "",
    raw,
    flags=re.IGNORECASE | re.DOTALL,
)
new = re.sub(r"\n[ \t]*\n([ \t]*</div>)", r"\n\1", new)
new = new.replace(
    '<div class="footer-content">',
    '<div class="footer-content" style="justify-content: center; text-align: center;">',
)

if new == raw:
    print("footer: no footer links found, nothing to do.")
else:
    with open(path, "w", encoding="utf-8", newline="") as f:
        f.write(new)
    print("footer: footer links removed, copyright centered.")
PYEOF

echo "OK: app changes applied"
