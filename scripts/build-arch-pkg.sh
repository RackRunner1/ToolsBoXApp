#!/usr/bin/env bash
# Builds the Arch Linux pacman package (.pkg.tar.zst) from the .deb payload.
#
# The Tauri bundler only produces .deb and .rpm, so the Arch package reuses
# the exact file tree shipped in the .deb (app binary, tbxa/tbxm, desktop
# file, icons) and wraps it in the pacman format (.PKGINFO + zstd tarball).
# The Tauri binary is the same across formats; only the packaging differs.
#
# Usage:
#   ./scripts/build-arch-pkg.sh <version> [--deb PATH] [--out-dir DIR] [--pkgrel N]
set -euo pipefail

VERSION=""
DEB=""
OUT_DIR=""
PKGREL="1"

while [ $# -gt 0 ]; do
  case "$1" in
    --deb) DEB="$2"; shift 2 ;;
    --out-dir) OUT_DIR="$2"; shift 2 ;;
    --pkgrel) PKGREL="$2"; shift 2 ;;
    -h|--help)
      echo "Usage: $(basename "$0") <version> [--deb PATH] [--out-dir DIR] [--pkgrel N]"
      exit 0
      ;;
    -*)
      echo "Unknown argument: $1" >&2
      echo "Usage: $(basename "$0") <version> [--deb PATH] [--out-dir DIR] [--pkgrel N]" >&2
      exit 2
      ;;
    *)
      if [ -n "$VERSION" ]; then
        echo "Unexpected argument: $1" >&2
        exit 2
      fi
      VERSION="$1"
      shift
      ;;
  esac
done

[ -n "$VERSION" ] || { echo "Missing <version> (e.g. 2026.10.16)." >&2; exit 2; }

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
[ -z "$DEB" ] && DEB="$(ls -t "$REPO_ROOT"/app/src-tauri/target/release/bundle/deb/*.deb 2>/dev/null | head -n 1)"
[ -z "$OUT_DIR" ] && OUT_DIR="$REPO_ROOT/artifacts"

[ -n "$DEB" ] && [ -f "$DEB" ] || { echo "No .deb found ('$DEB'). Build it first: npm run build in app/." >&2; exit 1; }
command -v dpkg-deb >/dev/null 2>&1 || { echo "dpkg-deb is required but not installed." >&2; exit 1; }
command -v zstd >/dev/null 2>&1 || { echo "zstd is required but not installed (sudo apt install zstd / sudo pacman -S zstd)." >&2; exit 1; }

PKGNAME="toolsboxapp"
ARCH="x86_64"
PKGFILE="${PKGNAME}-${VERSION}-${PKGREL}-${ARCH}.pkg.tar.zst"

STAGING="$(mktemp -d "${TMPDIR:-/tmp}/arch-pkg-XXXXXX")"
trap 'rm -rf "$STAGING"' EXIT

# Same files as the .deb: app binary, tbxa/tbxm, desktop file, icons.
dpkg-deb --fsys-tarfile "$DEB" | tar -x -C "$STAGING"
[ -x "$STAGING/usr/bin/toolsboxapp" ] || { echo "toolsboxapp binary missing from the .deb payload." >&2; exit 1; }
[ -x "$STAGING/usr/bin/tbxa" ] || { echo "tbxa missing from the .deb payload." >&2; exit 1; }
[ -x "$STAGING/usr/bin/tbxm" ] || { echo "tbxm missing from the .deb payload." >&2; exit 1; }

# Install license in the standard Arch location
mkdir -p "$STAGING/usr/share/licenses/$PKGNAME"
cp "$REPO_ROOT/LICENSE" "$STAGING/usr/share/licenses/$PKGNAME/LICENSE"

SIZE="$(du -sb "$STAGING/usr" | cut -f1)"
BUILD_DATE="$(date +%s)"

cat > "$STAGING/.PKGINFO" <<EOF
pkgname = $PKGNAME
pkgver = $VERSION-$PKGREL
pkgdesc = Desktop toolbox (ToolsBoXMachine content)
url = https://github.com/RackRunner1/ToolsBoXApp
builddate = $BUILD_DATE
packager = ToolsBoXApp CI
size = $SIZE
arch = $ARCH
license = Unlicense
depend = curl
depend = gtk3
depend = libsoup3
depend = webkit2gtk-4.1
EOF

mkdir -p "$OUT_DIR"

# Top-level payload entries (usr, ...), with .PKGINFO first.
mapfile -t TOPS < <(cd "$STAGING" && ls -A | grep -v '^\.PKGINFO$')
tar --owner=0 --group=0 --numeric-owner \
  -I 'zstd -19 -T0 -q' \
  -cf "$OUT_DIR/$PKGFILE" -C "$STAGING" .PKGINFO "${TOPS[@]}"

echo "Built: $OUT_DIR/$PKGFILE"
tar -I unzstd -tf "$OUT_DIR/$PKGFILE" | tee /tmp/arch-pkg-list.txt
grep -q '^\.PKGINFO$' /tmp/arch-pkg-list.txt || { echo ".PKGINFO missing from the Arch package!" >&2; exit 1; }
grep -q 'usr/bin/tbxa$' /tmp/arch-pkg-list.txt || { echo "tbxa missing from the Arch package!" >&2; exit 1; }
grep -q 'usr/bin/tbxm$' /tmp/arch-pkg-list.txt || { echo "tbxm missing from the Arch package!" >&2; exit 1; }
grep -q 'usr/bin/toolsboxapp$' /tmp/arch-pkg-list.txt || { echo "app binary missing from the Arch package!" >&2; exit 1; }
grep -q '\.desktop$' /tmp/arch-pkg-list.txt || { echo "desktop file missing from the Arch package!" >&2; exit 1; }
echo "Arch package verified: app binary, tbxa, tbxm and desktop file present."
