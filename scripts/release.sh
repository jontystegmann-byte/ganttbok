#!/usr/bin/env bash
# Cut a GanttBok release: build for x86_64 + aarch64, sign update artifacts,
# generate latest.json, publish a GitHub release with DMG + tarballs + manifest.
#
# Prereqs:
#   - gh auth login (run once)
#   - TAURI_SIGNING_PRIVATE_KEY_PATH or TAURI_SIGNING_PRIVATE_KEY set
#   - Rust targets installed: rustup target add x86_64-apple-darwin aarch64-apple-darwin
#   - Run from the repo root: ./scripts/release.sh

set -euo pipefail
cd "$(dirname "$0")/.."

REPO="jontystegmann-byte/ganttbok"
KEY_PATH="${TAURI_SIGNING_PRIVATE_KEY_PATH:-$HOME/.tauri/ganttbok.key}"
if [ ! -f "$KEY_PATH" ]; then
  echo "❌ Signing key not found at $KEY_PATH"
  exit 1
fi
export TAURI_SIGNING_PRIVATE_KEY="$(cat "$KEY_PATH")"
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD="${TAURI_SIGNING_PRIVATE_KEY_PASSWORD:-}"

VERSION=$(node -p "require('./src-tauri/tauri.conf.json').version")
TAG="v$VERSION"
echo "==> Releasing GanttBok $TAG"

if gh release view "$TAG" --repo "$REPO" >/dev/null 2>&1; then
  echo "❌ Release $TAG already exists on $REPO. Bump version in tauri.conf.json + package.json first."
  exit 1
fi

# Ensure cargo is on PATH (Tauri can't find it under Claude's shell otherwise)
[ -f "$HOME/.cargo/env" ] && source "$HOME/.cargo/env"

PUB_DATE="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
NOTES_FILE="$(mktemp)"
if [ -f "RELEASE_NOTES.md" ]; then
  cp RELEASE_NOTES.md "$NOTES_FILE"
else
  echo "Bug fixes and improvements." > "$NOTES_FILE"
fi
NOTES_ESCAPED=$(node -e "console.log(JSON.stringify(require('fs').readFileSync('$NOTES_FILE','utf8').trim()))")

build_target() {
  local TRIPLE="$1"
  echo "==> Building for $TRIPLE"
  pnpm tauri build --target "$TRIPLE"
}

build_target x86_64-apple-darwin
build_target aarch64-apple-darwin

assets_dir="$(mktemp -d)"
echo "==> Collecting artifacts into $assets_dir"

# Resolve the actual cargo target dir (may be redirected by .cargo/config.toml).
TARGET_DIR="$(cargo metadata --format-version 1 --no-deps 2>/dev/null \
  | grep -o '"target_directory":"[^"]*' | head -1 | sed 's/"target_directory":"//')"
if [ -z "$TARGET_DIR" ]; then
  echo "❌ Could not resolve cargo target directory via 'cargo metadata'"
  exit 1
fi
echo "==> cargo target_directory: $TARGET_DIR"

# collect() takes triple + a human-friendly slug used in the output filenames.
# The Tauri updater's platform keys (darwin-x86_64 / darwin-aarch64) are hard-coded
# in latest.json below — those must stay exact.
collect() {
  local TRIPLE="$1"
  local SLUG="$2"
  local BUNDLE_DIR="$TARGET_DIR/$TRIPLE/release/bundle"
  shopt -s nullglob
  local TARS=("$BUNDLE_DIR/macos/"*.app.tar.gz)
  local DMGS=("$BUNDLE_DIR/dmg/"*.dmg)
  shopt -u nullglob
  if [ ${#TARS[@]} -eq 0 ]; then
    echo "❌ No .app.tar.gz found in $BUNDLE_DIR/macos/"
    exit 1
  fi
  if [ ${#DMGS[@]} -eq 0 ]; then
    echo "❌ No .dmg found in $BUNDLE_DIR/dmg/"
    exit 1
  fi
  local TAR_SRC="${TARS[0]}"
  local DMG_SRC="${DMGS[0]}"
  local SIG_SRC="${TAR_SRC}.sig"
  if [ ! -f "$SIG_SRC" ]; then
    echo "❌ Missing signature: $SIG_SRC (is TAURI_SIGNING_PRIVATE_KEY set during the build?)"
    exit 1
  fi
  local DMG_OUT="$assets_dir/Blik_Plan_${VERSION}_${SLUG}.dmg"
  local TAR_OUT="$assets_dir/Blik_Plan_${VERSION}_${SLUG}.app.tar.gz"
  cp "$DMG_SRC" "$DMG_OUT"
  cp "$TAR_SRC" "$TAR_OUT"
  cp "$SIG_SRC" "$TAR_OUT.sig"
  echo "$TAR_OUT"
}

TAR_X86="$(collect x86_64-apple-darwin   Intel)"
TAR_ARM="$(collect aarch64-apple-darwin  Apple-Silicon)"

LATEST_JSON="$assets_dir/latest.json"
export GB_VERSION="$VERSION"
export GB_PUB_DATE="$PUB_DATE"
export GB_REPO="$REPO"
export GB_TAG="$TAG"
export GB_NOTES="$(cat "$NOTES_FILE")"
export GB_TAR_X86="$(basename "$TAR_X86")"
export GB_TAR_ARM="$(basename "$TAR_ARM")"
export GB_SIG_X86="$(cat "${TAR_X86}.sig")"
export GB_SIG_ARM="$(cat "${TAR_ARM}.sig")"
node -e '
const fs = require("fs");
const base = `https://github.com/${process.env.GB_REPO}/releases/download/${process.env.GB_TAG}`;
const manifest = {
  version: process.env.GB_VERSION,
  notes: process.env.GB_NOTES,
  pub_date: process.env.GB_PUB_DATE,
  platforms: {
    "darwin-x86_64":  { signature: process.env.GB_SIG_X86, url: `${base}/${process.env.GB_TAR_X86}` },
    "darwin-aarch64": { signature: process.env.GB_SIG_ARM, url: `${base}/${process.env.GB_TAR_ARM}` }
  }
};
fs.writeFileSync(process.argv[1], JSON.stringify(manifest, null, 2));
' "$LATEST_JSON"
echo "Wrote $LATEST_JSON"

echo "==> Creating GitHub release $TAG"
gh release create "$TAG" \
  --repo "$REPO" \
  --title "Gantt Bok $TAG" \
  --notes-file "$NOTES_FILE" \
  "$assets_dir"/*

echo ""
echo "✅ Released $TAG"
echo "   Manifest: https://github.com/$REPO/releases/latest/download/latest.json"
echo "   Existing installs will see the update on next launch (silent check) or via the version badge."
