# BLIK Plan

A small, fast macOS Gantt-chart app for builders, designers, and anyone who plans projects in weeks. Mon–Fri (or 7-day) workdays, public holidays auto-synced for **South Africa · USA · UK · India · China**, week-numbered timeline, magnetic-snap drag with hard-chain dependency ripple, per-job notes panel, today-line tracker. Prints A3 landscape (Gantt) and A4 portrait (notes). ~5 MB DMG, fully offline, no account, no cloud.

> Built by [Jonty Stegmann](https://github.com/jontystegmann-byte). Free.
>
> *Previously known as Gantt Bok — same app, new name, same auto-update channel.*

---

## Download

Always grab the latest version from the [**Releases page**](https://github.com/jontystegmann-byte/ganttbok/releases/latest).

### Which DMG?

1. Click the Apple menu  → **About This Mac** → look at the "Chip" row.
2. Pick the matching DMG below from the latest release:

| Your Mac | File to download |
|----------|------------------|
| **Apple Silicon** — M1, M2, M3, M4 (most Macs sold since late 2020) | `Blik_Plan_X.Y.Z_Apple-Silicon.dmg` |
| **Intel** — Macs with "Intel Core" listed under Chip | `Blik_Plan_X.Y.Z_Intel.dmg` |

Not sure? Try the **Apple-Silicon** one first. If macOS says "wrong architecture" when opening it, you're on Intel — grab the Intel DMG instead.

### Install

1. Open the DMG. Drag **Blik Plan** into your Applications folder.
2. First launch: macOS will say "developer cannot be verified" (the app is signed but not paid-Apple-Developer signed). One-time workaround:
   - **Right-click** the app in Applications → **Open** → confirm the warning dialog.
   - From here on every launch is normal.
3. Updates: from your first install onwards, every new version installs itself inside the app — you'll see a `Update to vX.Y.Z →` badge in the bottom-left corner. One click, restart, done. No more DMGs.

### Your data

All your jobs live in a single SQLite file at `~/Library/Application Support/Gantt Bok/ganttbok.db`. *(Path retained from the app's original name — Blik Plan and Gantt Bok share the same data directory so your jobs carried over the rebrand automatically.)* The file sits outside the app bundle, so it survives every reinstall and every auto-update. Time Machine backs it up automatically.

---

## What it does

- **Phases and tasks** — collapsible phases, drag-to-reorder, colour per phase.
- **Hard-chain dependencies** — link tasks; dragging a predecessor ripples successors automatically.
- **Mon–Fri workdays** — weekends are non-working by default.
- **SA public holidays** — auto-synced for the project's date range. Per-job toggle to either split bars around holidays (default) or let work run through them.
- **Manual no-work days** — right-click any day column to mark/unmark.
- **Templates** — save a job's structure as a template, instantiate new jobs from it.
- **A3 landscape print** — Cmd+P → opens macOS print dialog with fit-to-page or multi-page options. Save as PDF works.
- **Undo / redo** — Cmd+Z / Cmd+Shift+Z across every action.

---

## Reporting bugs / requesting features

Open an [issue](https://github.com/jontystegmann-byte/ganttbok/issues) on GitHub or message me directly.

---

## For developers

### Run from source

```bash
pnpm install
pnpm tauri dev
```

### Test

```bash
pnpm vitest run
cd src-tauri && cargo test
```

### Cut a release

See [`docs/RELEASE.md`](docs/RELEASE.md). Short version: bump version in `tauri.conf.json` + `package.json`, optionally edit `RELEASE_NOTES.md`, then `./scripts/release.sh`.
