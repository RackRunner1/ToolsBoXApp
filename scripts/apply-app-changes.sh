#!/usr/bin/env bash
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

WEB_DIR="$WEB_DIR" python3 - <<'PYEOF'
import os
import re

path = os.path.join(os.environ["WEB_DIR"], "ui", "navbar.html")
if not os.path.isfile(path):
    print("(skipped) ui/navbar.html not found in the web copy.")
    raise SystemExit(0)

with open(path, encoding="utf-8") as f:
    raw = f.read()

new = re.sub(
    r'<a\b[^>]*href="/pages/app\.html"[^>]*>.*?</a\s*>',
    "",
    raw,
    flags=re.IGNORECASE | re.DOTALL,
)
new = re.sub(r"\n[ \t]*\n([ \t]*</div>)", r"\n\1", new)

if new == raw:
    print("navbar: no App link found, nothing to do.")
else:
    with open(path, "w", encoding="utf-8", newline="") as f:
        f.write(new)
    print("navbar: App link removed.")
PYEOF

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

if new == raw:
    print("footer: no footer links found, nothing to do.")
else:
    with open(path, "w", encoding="utf-8", newline="") as f:
        f.write(new)
    print("footer: footer links removed, copyright kept.")
PYEOF

WEB_DIR="$WEB_DIR" python3 - <<'PYEOF'
import os
import re

path = os.path.join(os.environ["WEB_DIR"], "ui", "navbar.html")
if not os.path.isfile(path):
    print("(skipped) ui/navbar.html not found in the web copy.")
    raise SystemExit(0)

with open(path, encoding="utf-8") as f:
    raw = f.read()

new = re.sub(
    r'<a\b[^>]*class="nav-edition"[^>]*>.*?</a\s*>',
    "",
    raw,
    flags=re.IGNORECASE | re.DOTALL,
)
new = re.sub(r"\n[ \t]*\n([ \t]*</div>)", r"\n\1", new)

if new == raw:
    print("navbar: no edition label found, nothing to do.")
else:
    with open(path, "w", encoding="utf-8", newline="") as f:
        f.write(new)
    print("navbar: edition label removed.")
PYEOF

WEB_DIR="$WEB_DIR" python3 - <<'PYEOF'
import os
import re

path = os.path.join(os.environ["WEB_DIR"], "index.html")
if not os.path.isfile(path):
    print("(skipped) index.html not found in the web copy.")
    raise SystemExit(0)

with open(path, encoding="utf-8") as f:
    raw = f.read()

new = re.sub(
    r'[ \t]*<div\b[^>]*class="maintenance-banner"[^>]*>.*?</div\s*>\n?',
    "",
    raw,
    flags=re.IGNORECASE | re.DOTALL,
)

if new == raw:
    print("index: no maintenance banner found, nothing to do.")
else:
    with open(path, "w", encoding="utf-8", newline="") as f:
        f.write(new)
    print("index: maintenance banner removed.")
PYEOF

WEB_DIR="$WEB_DIR" python3 - <<'PYEOF'
import os
import re

path = os.path.join(os.environ["WEB_DIR"], "tools", "new-year-countdown", "new-year-countdown.html")
if not os.path.isfile(path):
    print("(skipped) tools/new-year-countdown/new-year-countdown.html not found in the web copy.")
    raise SystemExit(0)

with open(path, encoding="utf-8") as f:
    raw = f.read()

new = re.sub(
    r'\s*<section class="ny-info">.*?</section>',
    "",
    raw,
    flags=re.IGNORECASE | re.DOTALL,
)

if new == raw:
    print("new-year: info section not found, nothing to do.")
else:
    with open(path, "w", encoding="utf-8", newline="") as f:
        f.write(new)
    print("new-year: 'How the New Year countdown works' section removed.")
PYEOF

echo "OK: app changes applied"
