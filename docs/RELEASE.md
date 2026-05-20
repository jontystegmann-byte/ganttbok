# Release & install

## Build

```bash
cd ~/Desktop/GanttBok && pnpm tauri build
```

Produces:
- `src-tauri/target/release/bundle/macos/Gantt Bok.app`  — drop in /Applications
- `src-tauri/target/release/bundle/dmg/Gantt Bok_X.Y.Z_*.dmg` — installer

First build takes 10–15 min (Rust release compile). Subsequent builds are 1–2 min.

## First install on Gray's Mac

1. Double-click the `.dmg`. Drag `Gantt Bok.app` into Applications.
2. First launch: Gatekeeper will block with "developer cannot be verified".
3. Workaround: right-click the app in Applications → **Open** → confirm the dialog. Only needed once.
4. The app now launches normally on every subsequent open.

## Why ad-hoc signing?

`tauri.conf.json` uses `"signingIdentity": "-"` which signs the binary with an ad-hoc certificate. This means:
- No paid Apple Developer account required ($0/yr instead of $99/yr).
- The binary is still cryptographically signed, just not by Apple.
- Gatekeeper warns once on first open, then trusts it forever.

## Future: full notarisation (zero-warning install)

1. Get an Apple Developer account ($99/yr at developer.apple.com).
2. Generate a "Developer ID Application" certificate via Xcode → Settings → Accounts.
3. Set `signingIdentity` in `tauri.conf.json` to your team's identity string (e.g. `"Developer ID Application: Jonty Stegmann (TEAMID)"`).
4. Configure notarisation env vars (`APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID`) per Tauri docs.
5. `pnpm tauri build` will sign and notarise automatically.

## Updating

There is no auto-update channel in v1.0.0. To roll out v1.1+:
1. Bump the version in `src-tauri/tauri.conf.json`.
2. Rebuild (`pnpm tauri build`).
3. Hand the new `.dmg` to Gray; he drags the new `.app` into Applications, replacing the old one.

The SQLite database lives in `~/Library/Application Support/com.jontystegmann.ganttbok/` and is preserved across reinstalls.
