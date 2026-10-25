#!/usr/bin/env bash
# Install the uv version uv.toml pins (required-version = "==X.Y.Z") into DEST (default: this
# repository's .tools/uv-X.Y.Z/), after checking the release archive's sha256 against
# scripts/uv-checksums.txt. Does nothing when DEST already holds that version.
# Usage: scripts/install-uv.sh [DEST]
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="$(sed -n 's/^required-version *= *"==\([^"]*\)"$/\1/p' "$ROOT/uv.toml")"
[ -n "$VERSION" ] || { echo "uv.toml pins no exact uv (required-version = \"==X.Y.Z\")" >&2; exit 1; }
DEST="${1:-$ROOT/.tools/uv-$VERSION}"
if [ -x "$DEST/uv" ] && [ "$("$DEST/uv" --version | cut -d' ' -f2)" = "$VERSION" ]; then exit 0; fi
case "$(uname -s)/$(uname -m)" in
  Darwin/arm64) TARGET=aarch64-apple-darwin ;;
  Darwin/x86_64) TARGET=x86_64-apple-darwin ;;
  Linux/aarch64|Linux/arm64) TARGET=aarch64-unknown-linux-gnu ;;
  Linux/x86_64|Linux/amd64) TARGET=x86_64-unknown-linux-gnu ;;
  *) echo "Unsupported platform: $(uname -s)/$(uname -m)" >&2; exit 1 ;;
esac
ARCHIVE="uv-$TARGET.tar.gz"
EXPECTED="$(awk -v key="$VERSION/$ARCHIVE" '$2 == key { print $1 }' "$ROOT/scripts/uv-checksums.txt")"
[ -n "$EXPECTED" ] || { echo "No sha256 for $VERSION/$ARCHIVE in scripts/uv-checksums.txt; add it from the release's $ARCHIVE.sha256" >&2; exit 1; }
WORK="$(mktemp -d)"
trap 'rm -rf "${WORK:?}"' EXIT
curl -sSfL "https://github.com/astral-sh/uv/releases/download/$VERSION/$ARCHIVE" -o "$WORK/$ARCHIVE"
if command -v sha256sum >/dev/null; then ACTUAL="$(sha256sum "$WORK/$ARCHIVE")"; else ACTUAL="$(shasum -a 256 "$WORK/$ARCHIVE")"; fi
ACTUAL="${ACTUAL%% *}"
[ "$ACTUAL" = "$EXPECTED" ] || { echo "$ARCHIVE has sha256 $ACTUAL, not $EXPECTED (scripts/uv-checksums.txt)" >&2; exit 1; }
tar -C "$WORK" -xzf "$WORK/$ARCHIVE" "uv-$TARGET/uv"
mkdir -p "$DEST"
mv "$WORK/uv-$TARGET/uv" "$DEST/uv"
chmod 0755 "$DEST/uv"
"$DEST/uv" --version
