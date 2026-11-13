#!/usr/bin/env bash
# Writes the `.zsync` control file that lets an installed AppImage update itself.
#
# A Tauri AppImage carries the update *information* (we hand `LDAI_UPDATE_INFORMATION` to the
# bundler), but not the control file it points at — that is what `zsyncmake` does, and Tauri has
# no step for it. Without it the embedded pointer leads nowhere: Shelly says "no zsync update
# URL", AppImageUpdate says the same, and the file stays at whatever version was downloaded.
#
# The URL header inside the control file is what a client follows to fetch the new version, so it
# has to name the release asset exactly — hence the tag, not `latest`.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="$(node -p "require('${ROOT}/package.json').version")"
TAG="${GITHUB_REF_NAME:-v$VERSION}"
REPO="modestcoders-com/dside"
DIR="$ROOT/src-tauri/target/release/bundle/appimage"

shopt -s nullglob
appimages=("$DIR"/*.AppImage)
shopt -u nullglob
if [ ${#appimages[@]} -eq 0 ]; then
  echo "no AppImage in $DIR — build it first (pnpm tauri build --bundles appimage)" >&2
  exit 1
fi
if [ ${#appimages[@]} -gt 1 ]; then
  echo "more than one AppImage in $DIR; refusing to guess which one to describe:" >&2
  printf '  %s\n' "${appimages[@]}" >&2
  exit 1
fi

FILE="${appimages[0]}"
NAME="$(basename "$FILE")"
URL="https://github.com/${REPO}/releases/download/${TAG}/${NAME}"

# The client resolves `<filename>.zsync` next to the AppImage, so the name must be predictable.
zsyncmake -u "$URL" -o "${FILE}.zsync" "$FILE"

# A control file without these headers is useless; check rather than assume.
for header in "Filename:" "MTime:" "URL:" "SHA-1:"; do
  if ! grep -q "^${header}" "${FILE}.zsync"; then
    echo "the control file has no ${header} header" >&2
    exit 1
  fi
done

echo "update file: ${FILE}.zsync → ${URL}"
