#!/usr/bin/env bash
# Updates the AUR PKGBUILD and .SRCINFO for toolsboxapp-bin.
#
# Usage:
#   ./scripts/update-aur.sh [--version X.Y.Z] [--tag YYYYMMDD] [--pkg-path PATH] [--push]
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
AUR_DIR="$REPO_ROOT/aur/toolsboxapp-bin"
PKGBUILD="$AUR_DIR/PKGBUILD"
SRCINFO="$AUR_DIR/.SRCINFO"

VERSION=""
TAG=""
PKG_PATH=""
DO_PUSH=false

while [ $# -gt 0 ]; do
  case "$1" in
    --version) VERSION="$2"; shift 2 ;;
    --tag) TAG="$2"; shift 2 ;;
    --pkg-path) PKG_PATH="$2"; shift 2 ;;
    --push) DO_PUSH=true; shift ;;
    -h|--help)
      echo "Usage: $(basename "$0") [--version VERSION] [--tag TAG] [--pkg-path FILE] [--push]"
      exit 0
      ;;
    *) echo "Unknown argument: $1" >&2; exit 2 ;;
  esac
done

# If version is not provided, fetch the latest GitHub release
if [ -z "$VERSION" ] || [ -z "$TAG" ]; then
  LATEST_JSON="$(curl -fsSL https://api.github.com/repos/RackRunner1/ToolsBoXApp/releases/latest)"
  TAG="$(echo "$LATEST_JSON" | grep -m1 '"tag_name":' | cut -d '"' -f 4)"
  # Extract version from pkg filename
  PKG_NAME="$(echo "$LATEST_JSON" | grep -o 'toolsboxapp-[0-9.]\+-[0-9]\+-x86_64\.pkg\.tar\.zst' | head -n1)"
  VERSION="$(echo "$PKG_NAME" | sed -E 's/toolsboxapp-([0-9.]+)-[0-9]+-x86_64\.pkg\.tar\.zst/\1/')"
fi

echo "Updating AUR package toolsboxapp-bin to version $VERSION (tag $TAG)..."

# Download or calculate SHA256 of the release pkg.tar.zst
if [ -n "$PKG_PATH" ] && [ -f "$PKG_PATH" ]; then
  TAR_SHA256="$(sha256sum "$PKG_PATH" | cut -d' ' -f1)"
else
  CHECKSUMS_URL="https://github.com/RackRunner1/ToolsBoXApp/releases/download/${TAG}/checksums.txt"
  CHECKSUMS="$(curl -fsSL "$CHECKSUMS_URL")"
  TAR_SHA256="$(echo "$CHECKSUMS" | grep "toolsboxapp-${VERSION}-.*-x86_64\.pkg\.tar\.zst" | head -n1 | cut -d' ' -f1)"
fi

LICENSE_SHA256="$(sha256sum "$REPO_ROOT/LICENSE" | cut -d' ' -f1)"

[ -n "$TAR_SHA256" ] || { echo "Could not determine SHA256 for pkg.tar.zst" >&2; exit 1; }
[ -n "$LICENSE_SHA256" ] || { echo "Could not determine SHA256 for LICENSE" >&2; exit 1; }

# Update PKGBUILD
cat > "$PKGBUILD" <<EOF
# Maintainer: RackRunner1 <https://github.com/RackRunner1>
pkgname=toolsboxapp-bin
pkgver=${VERSION}
pkgrel=1
pkgdesc="Desktop toolbox (ToolsBoXMachine content)"
arch=('x86_64')
url="https://github.com/RackRunner1/ToolsBoXApp"
license=('Unlicense')
depends=('glibc' 'gcc-libs' 'gtk3' 'libsoup3' 'webkit2gtk-4.1' 'curl' 'hicolor-icon-theme')
provides=('toolsboxapp')
conflicts=('toolsboxapp')
options=('!debug')

_tag="${TAG}"
source_x86_64=("https://github.com/RackRunner1/ToolsBoXApp/releases/download/\${_tag}/toolsboxapp-\${pkgver}-\${pkgrel}-\${CARCH}.pkg.tar.zst"
               "LICENSE::https://raw.githubusercontent.com/RackRunner1/ToolsBoXApp/main/LICENSE")
sha256sums_x86_64=('${TAR_SHA256}'
                  '${LICENSE_SHA256}')

package() {
    cp -dr --no-preserve=ownership "\${srcdir}/usr" "\${pkgdir}/"
    install -Dm644 "\${srcdir}/LICENSE" "\${pkgdir}/usr/share/licenses/\${pkgname}/LICENSE"
}
EOF

# Update .SRCINFO
cat > "$SRCINFO" <<EOF
pkgbase = toolsboxapp-bin
	pkgdesc = Desktop toolbox (ToolsBoXMachine content)
	pkgver = ${VERSION}
	pkgrel = 1
	url = https://github.com/RackRunner1/ToolsBoXApp
	arch = x86_64
	license = Unlicense
	depends = glibc
	depends = gcc-libs
	depends = gtk3
	depends = libsoup3
	depends = webkit2gtk-4.1
	depends = curl
	depends = hicolor-icon-theme
	provides = toolsboxapp
	conflicts = toolsboxapp
	options = !debug
	source_x86_64 = https://github.com/RackRunner1/ToolsBoXApp/releases/download/${TAG}/toolsboxapp-${VERSION}-1-x86_64.pkg.tar.zst
	source_x86_64 = LICENSE::https://raw.githubusercontent.com/RackRunner1/ToolsBoXApp/main/LICENSE
	sha256sums_x86_64 = ${TAR_SHA256}
	sha256sums_x86_64 = ${LICENSE_SHA256}

pkgname = toolsboxapp-bin
EOF

echo "PKGBUILD and .SRCINFO updated successfully."

if [ "$DO_PUSH" = "true" ]; then
  AUR_REPO_URL="ssh://aur@aur.archlinux.org/toolsboxapp-bin.git"
  TMP_AUR="$(mktemp -d)"
  trap 'rm -rf "$TMP_AUR"' EXIT
  git clone "$AUR_REPO_URL" "$TMP_AUR"
  cp "$PKGBUILD" "$SRCINFO" "$TMP_AUR/"
  (
    cd "$TMP_AUR"
    git config user.name "RackRunner1"
    git config user.email "rackrunner1@users.noreply.github.com"
    git add PKGBUILD .SRCINFO
    if git diff --cached --quiet; then
      echo "AUR is already up-to-date."
    else
      git commit -m "Update to $VERSION"
      git push origin master
      echo "Pushed update to AUR."
    fi
  )
fi
