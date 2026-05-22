#!/usr/bin/env bash
# copy-mcp-sidecar.sh
# Copies the compiled blikplan-mcp binary into the Tauri sidecar directory
# with the required target-triple suffix.
# Run after `cargo build --release -p blikplan-mcp`.
#
# Usage: ./scripts/copy-mcp-sidecar.sh [--debug]
#
# The Tauri app (Plan 4) adds "binaries/blikplan-mcp" to bundle.externalBin.
# Tauri resolves the correct platform binary at build time.

set -euo pipefail

PROFILE="${1:---release}"
PROFILE_DIR="release"
if [ "$PROFILE" = "--debug" ]; then
  PROFILE_DIR="debug"
fi

# Use cargo metadata to find the actual target directory,
# which may be redirected via .cargo/config.toml
TARGET_DIR="$(cargo metadata --format-version 1 --no-deps 2>/dev/null | grep -o '"target_directory":"[^"]*' | head -1 | sed 's/"target_directory":"//')"
BUILD_DIR="${TARGET_DIR}/${PROFILE_DIR}"

TRIPLE="$(rustc -vV | awk '/^host:/ { print $2 }')"
SRC="${BUILD_DIR}/blikplan-mcp"
DEST_DIR="src-tauri/binaries"
DEST="${DEST_DIR}/blikplan-mcp-${TRIPLE}"

if [ ! -f "$SRC" ]; then
  echo "ERROR: $SRC not found. Run: cargo build --release -p blikplan-mcp" >&2
  exit 1
fi

mkdir -p "$DEST_DIR"
cp "$SRC" "$DEST"
echo "Copied $SRC → $DEST"
