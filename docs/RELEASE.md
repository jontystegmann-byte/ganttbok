# Release & install

## Cutting a new release

```bash
# 1. Bump version in both files (must match):
#    src-tauri/tauri.conf.json  →  "version": "X.Y.Z"
#    package.json               →  "version": "X.Y.Z"
#
# 2. Optionally write release notes to RELEASE_NOTES.md (root). If absent, a
#    generic note is published.
#
# 3. Run the release script. It builds x86_64 + aarch64, signs the updater
#    artifacts, generates latest.json, and creates a GitHub release.
./scripts/release.sh
```

What gets published to `github.com/jontystegmann-byte/ganttbok/releases/tag/vX.Y.Z`:

- `Gantt_Bok_X.Y.Z_darwin-x86_64.dmg`           ← drag-install for Intel Macs
- `Gantt_Bok_X.Y.Z_darwin-aarch64.dmg`          ← drag-install for Apple Silicon
- `Gantt_Bok_X.Y.Z_darwin-x86_64.app.tar.gz`    ← used by in-app updater
- `Gantt_Bok_X.Y.Z_darwin-aarch64.app.tar.gz`   ← used by in-app updater
- `*.app.tar.gz.sig` files                       ← Ed25519 signatures
- `latest.json`                                  ← the update manifest the app polls

The in-app updater hits
`https://github.com/jontystegmann-byte/ganttbok/releases/latest/download/latest.json`
on each launch (silent, 3 s after open) and when the user clicks the version
badge in the bottom-left.

## Signing keys

Tauri's updater uses an Ed25519 keypair (separate from Apple code-signing).

- **Private key**: `~/.tauri/ganttbok.key` (no password)
- **Public key**: `~/.tauri/ganttbok.key.pub` (baked into `tauri.conf.json`)

The script reads the private key from `$TAURI_SIGNING_PRIVATE_KEY_PATH` (defaults
to `~/.tauri/ganttbok.key`). If the key has a password, also set
`$TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.

### ⚠️ Key custody

**If you lose this key, you cannot ship updates to existing installs.** Existing
apps will reject any update signed by a different key. Recovery requires
re-onboarding every user with a fresh manual install carrying the new pubkey.

Back up `~/.tauri/ganttbok.key` to:

- 1Password (paste contents as a Secure Note)
- iCloud Drive (encrypted at rest)

## Prereqs (one-time)

```bash
brew install gh
gh auth login                           # web-browser flow
rustup target add aarch64-apple-darwin  # for arm64 cross-build
rustup target add x86_64-apple-darwin   # for x86_64 cross-build
```

## First install on a new Mac

1. Download the matching DMG from the latest GitHub release
   (`darwin-aarch64` for Apple Silicon, `darwin-x86_64` for Intel).
2. Open the DMG. Drag **Gantt Bok.app** into Applications.
3. First launch: Gatekeeper will block with "developer cannot be verified".
4. Workaround: right-click the app in Applications → **Open** → confirm the
   dialog. Only needed once.
5. From here on, every update arrives in-app — no more DMG drag-drop.

## Why ad-hoc signing?

`tauri.conf.json` uses `"signingIdentity": "-"`, signing the binary with an
ad-hoc certificate. No paid Apple Developer account required ($0/yr instead of
$99/yr). Gatekeeper warns once on first install; updates delivered via the
in-app updater inherit the original trust and do not re-trigger Gatekeeper.

## Future: full notarisation (zero-warning first install)

1. Get an Apple Developer account ($99/yr).
2. Generate a "Developer ID Application" certificate via Xcode → Settings →
   Accounts.
3. Set `signingIdentity` in `tauri.conf.json` to your team's identity string.
4. Configure notarisation env vars (`APPLE_ID`, `APPLE_PASSWORD`,
   `APPLE_TEAM_ID`) per Tauri docs.
5. `pnpm tauri build` will sign and notarise automatically.

## Data location

The SQLite database lives at
`~/Library/Application Support/Gantt Bok/ganttbok.db`. It sits **outside** the
`.app` bundle, so it survives every reinstall and auto-update. Time Machine
backs it up automatically.
