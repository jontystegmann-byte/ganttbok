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
export TAURI_SIGNING_PRIVATE_KEY_PATH="$KEY_PATH"
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

declare -A PLATFORM_TAR
declare -A PLATFORM_SIG

collect() {
  local TRIPLE="$1"
  local PLATFORM_KEY="$2"
  local BUNDLE_DIR="src-tauri/target/$TRIPLE/release/bundle"
  local TAR=$(ls "$BUNDLE_DIR/macos/"*.app.tar.gz | head -1)
  local SIG="${TAR}.sig"
  local DMG=$(ls "$BUNDLE_DIR/dmg/"*.dmg | head -1)
  local DMG_OUT="$assets_dir/Gantt_Bok_${VERSION}_${PLATFORM_KEY}.dmg"
  local TAR_OUT="$assets_dir/Gantt_Bok_${VERSION}_${PLATFORM_KEY}.app.tar.gz"
  cp "$DMG" "$DMG_OUT"
  cp "$TAR" "$TAR_OUT"
  cp "$SIG" "$TAR_OUT.sig"
  PLATFORM_TAR[$PLATFORM_KEY]="$TAR_OUT"
  PLATFORM_SIG[$PLATFORM_KEY]="$(cat "$SIG")"
}

collect x86_64-apple-darwin   darwin-x86_64
collect aarch64-apple-darwin  darwin-aarch64

LATEST_JSON="$assets_dir/latest.json"
export GB_VERSION="$VERSION"
export GB_PUB_DATE="$PUB_DATE"
export GB_REPO="$REPO"
export GB_TAG="$TAG"
export GB_NOTES="$(cat "$NOTES_FILE")"
export GB_TAR_X86="$(basename "${PLATFORM_TAR[darwin-x86_64]}")"
export GB_TAR_ARM="$(basename "${PLATFORM_TAR[darwin-aarch64]}")"
export GB_SIG_X86="$(cat "${PLATFORM_TAR[darwin-x86_64]}.sig")"
export GB_SIG_ARM="$(cat "${PLATFORM_TAR[darwin-aarch64]}.sig")"
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
