Gantt Bok v1.1.0 — features, polish, bug fixes

NEW
- Solid vertical lines at every week boundary, running the full height of the chart.
- Click any duration tag (e.g. "2.3w") to flip every tag between weeks and days. Setting is remembered next time you open the app.
- Each job now has a "SA public holidays count as no-work days" toggle in the right-hand panel. Switch it off and bars will run through holidays instead of stepping around them. New jobs default to whatever you used last.
- Dependency creation gesture redesigned. Resize handles at the bar's left and right edges are now wider and faintly tinted so you can see them. The dependency-creation port is now a separate filled circle that floats 10 px outside the right edge with a clear connector — no more accidental drags between resize and dependency.

FIXED
- Print now actually works. Cmd+P → Print → Print → the macOS print dialog opens (save-as-PDF or send to printer). Previous build silently failed.
- Row alignment between the task list and the chart canvas. Phase + task rows now line up perfectly all the way down, including when phases have many tasks.

UNDER THE HOOD
- Native Apple Silicon build (no more Rosetta).
- All future updates land in-app — version badge in the bottom-left becomes "Update to v1.1.X →" when a new release is published.
