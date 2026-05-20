# For Gray — Gantt Bok v1.0.0

A Gantt-chart desktop app for managing your apartment-renovation projects. Built around the way you actually think about a job: week-numbered, Monday-to-Friday, collapsible phases, drag-to-reorder, A3 landscape print.

---

## What to send

One file:

> **`Gantt_Bok_v1.0.0.dmg`** *(3.9 MB)*

Located on Jonty's Desktop. AirDrop it, email it, USB it — whatever works.

(If you want, copy this `FOR GRAY.md` file across too as a printed quick-start.)

---

## Install (~30 seconds)

1. **Double-click `Gantt_Bok_v1.0.0.dmg`.** A small window opens.
2. **Drag the `Gantt Bok` app icon into the `Applications` folder shortcut** in that window.
3. **Eject the DMG** (right-click the disk on the Desktop → Eject).
4. Open `Applications` and double-click `Gantt Bok`.

### First launch only

Because the app isn't notarized with Apple yet, macOS will warn you the first time:

> *"Gantt Bok cannot be opened because the developer cannot be verified."*

To get past this **once**:

1. Click `Cancel` on that warning.
2. Open `System Settings → Privacy & Security`.
3. Scroll to the *Security* section. You'll see a line: *"Gantt Bok was blocked from use because it is not from an identified developer."*
4. Click `Open Anyway`.
5. Re-open the app from Applications. It opens normally. **You'll never see the warning again.**

---

## First job

When the app opens you'll see an empty sidebar with a `+ New job` button. Click it. Fill in:

- **Name** — e.g. "Sea Point apartment reno"
- **Client** (optional)
- **Address** (optional)
- **Project start** — the Monday (or whichever weekday) work begins
- **Start from** — leave as "Blank" for your first job. Once you've built a template you'll see it here too.

Hit `Create`. The Gantt chart opens with an empty grid, week-numbered from Week 1, public holidays for the next 18 months auto-marked with diagonal stripes.

---

## Building out a job

The flow is roughly:

1. **Add a phase** (`+ Phase` button at the bottom of the left rail). Names like "Demolition", "Plumbing", "Electrical", "Tiling", "Finishes". Each phase gets an auto-colour you can change.
2. **Add tasks inside each phase** (expand the phase, hit `+ Task`). Each task starts as 3 workdays long. Rename and resize as you go.
3. **Link tasks** when one has to finish before another can start. Hover over a task bar — a small circle (`○`) appears on its right edge. Click and drag that circle onto the dependent task. Now they're chained — dragging the first one shifts the second one automatically.
4. **Tweak dates** by grabbing any bar and dragging it. The bar snaps to day-edges with a soft magnetic pull. Resize by grabbing either edge.

---

## Keyboard shortcuts

| Shortcut | What it does |
|---|---|
| `⌘ N` | (button only — there's no shortcut yet) New job |
| `⌘ Z` | Undo last change |
| `⌘ ⇧ Z` | Redo |
| `⌘ S` | Manually save (everything autosaves anyway — this is a peace-of-mind button) |
| `⌘ P` | Open the Print Options sheet |
| Double-click empty cell | Quick-create a 1-day task on that exact day in that phase row |
| Right-click day-column header | Mark a custom non-working day (rain day, site closed, body corporate AGM) |
| Right-click any job in sidebar | Save as template / Archive / Delete |

---

## Drag rules

- **Drag a task** in the middle = move it in time. Downstream linked tasks shift with it automatically.
- **Drag a task's left or right edge** = resize. Cursor changes to a horizontal-arrow when you're in the resize zone.
- **Drag a collapsed phase bar** = move the entire phase as one unit. All its tasks slide together.
- **Drag a phase or task row's left-rail label** vertically = reorder (changes the 1, 1.1, 1.2 numbering).
- Bars **always snap to whole working days** on release.

---

## Templates

Once you've built out a typical apartment-reno structure (phases + tasks), right-click that job in the sidebar → **Save as template**. It saves the *skeleton* — names only, no dates, no durations, no dependencies. For the next job, pick that template from the "Start from" dropdown in the New-Job modal; you get the skeleton ready to fill in for the new site.

Templates show in their own section in the sidebar.

---

## Public holidays

South African public holidays are auto-marked for every project as diagonal-striped grey columns with the holiday name running vertically. They're visual-only — bars draw straight through them (since you and your team are presumably making up the time, not actually pausing).

If you want a day marked as non-working that ISN'T a public holiday — a rain day, site closed for some reason — right-click that day's column header at the top of the chart and pick `Mark non-working day`. Right-click again to unmark.

---

## Printing

`⌘P` opens the Print Options sheet. Choose **A3 landscape** (default), **Fit to page** (default for jobs up to ~18 weeks) or **Multi-page** for longer ones, and whether to print task notes. Hit `Print` and the standard macOS print dialog opens — you can send to a printer or save as PDF.

The printed sheet shows:
- Job header (name, client, address, print date)
- The full chart in the same visual language as the screen
- Public holidays in the project's range, listed at the bottom

What's collapsed on-screen prints collapsed — so you control the resolution of the printout by expanding/collapsing phases before pressing `⌘P`.

---

## Your data

Everything lives on your Mac at:

```
~/Library/Application Support/Gantt Bok/ganttbok.db
```

One SQLite file. Time Machine backs it up automatically (no app-side backup logic). If the disk dies, restore from Time Machine; your jobs come with it.

---

## Anything weird?

If you hit a bug or something feels wrong, ping Jonty. He's the only developer and you're the only user — fast feedback loop.

---

*Gantt Bok v1.0.0 — built for Gray Robertson by Jonty Stegmann, May 2026.*
