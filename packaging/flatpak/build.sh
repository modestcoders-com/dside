#!/usr/bin/env bash
# Builds the Flatpak for DS·IDE.
#
# Build the program first, package it second: the binary is compiled here with the same flags a
# release uses (`--features tauri/custom-protocol`, without which it loads the dev URL), and the
# sandbox only copies it into place. A Flatpak built from a binary nobody ran is a packaging
# claim, not a test.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
VERSION="$(node -p "require('${ROOT}/package.json').version")"
BUILD="$ROOT/target/flatpak/build"
REPO="$ROOT/target/flatpak/repo"
STATE="$ROOT/target/flatpak/state"
OUT="$ROOT/target/flatpak/DS-IDE-${VERSION}-x86_64.flatpak"

cd "$ROOT"
pnpm build
(
  cd src-tauri
  cargo build --release --features tauri/custom-protocol
)

# The manifest reads this file; it is gitignored, like every build product here.
cp "$ROOT/src-tauri/target/release/DSIDE" "$ROOT/packaging/flatpak/DSIDE"

rm -rf "$BUILD" "$REPO" "$STATE"
mkdir -p "$ROOT/target/flatpak"

flatpak remote-add --if-not-exists --user flathub \
  https://flathub.org/repo/flathub.flatpakrepo

flatpak-builder --user --force-clean --disable-rofiles-fuse \
  --install-deps-from=flathub \
  --state-dir="$STATE" \
  --repo="$REPO" \
  "$BUILD" "$ROOT/packaging/flatpak/com.deepvibe.dside.yml"

flatpak build-bundle "$REPO" "$OUT" com.deepvibe.dside
echo "$OUT"
