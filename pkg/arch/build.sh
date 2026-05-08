#!/usr/bin/env bash
# build.sh — Create an Arch Linux package from the local ezkvm source tree.
#
# Usage:
#   cd pkg/arch
#   ./build.sh
#
# The resulting .pkg.tar.zst can be installed with:
#   sudo pacman -U ezkvm-<version>-1-<arch>.pkg.tar.zst
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

PKGVER=$(grep '^version' "$REPO_ROOT/Cargo.toml" | head -1 | cut -d'"' -f2)
PKGNAME="ezkvm"
TARBALL="$SCRIPT_DIR/${PKGNAME}-${PKGVER}.tar.gz"

echo "Packaging $PKGNAME-$PKGVER for Arch Linux..."

# Create reproducible source tarball from git HEAD
git -C "$REPO_ROOT" archive --prefix="${PKGNAME}-${PKGVER}/" HEAD \
    -o "$TARBALL"

# Sync pkgver in PKGBUILD to match Cargo.toml
sed -i "s/^pkgver=.*/pkgver=${PKGVER}/" "$SCRIPT_DIR/PKGBUILD"

cd "$SCRIPT_DIR"
makepkg -f --cleanbuild

RESULT=$(ls "${PKGNAME}"-*.pkg.tar.zst 2>/dev/null | head -1)
if [ -n "$RESULT" ]; then
    echo "Package ready: $SCRIPT_DIR/$RESULT"
    echo ""
    echo "Install with:"
    echo "  sudo pacman -U $SCRIPT_DIR/$RESULT"
else
    echo "ERROR: no package file found after makepkg" >&2
    exit 1
fi
