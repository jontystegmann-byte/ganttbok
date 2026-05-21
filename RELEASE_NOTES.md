Blik Plan v1.3.0 — rebrand, 5-region holidays, today line

🎉 GANTT BOK IS NOW BLIK PLAN.
Same app, same auto-update channel, same data (your jobs carry over untouched). New name, new red-and-white BP logo, and a stack of new features below.

ON FIRST LAUNCH after this update, you'll see a small "Rebranded" banner offering to rename `Gantt Bok.app` → `Blik Plan.app` in your Applications folder. One click does it. Skip it if you prefer; the dock and menu already show the new name regardless.

NEW
- Public-holiday regions: pick from **South Africa · United States · United Kingdom · India · China** in Settings. Changing region re-syncs the holidays for that job. New jobs default to your last-used region.
- **Today line**: a red vertical line tracks the current date across the whole chart, with a date flag at the top.
- **Past-task fade**: tasks that finished before today render dimmed so you can see at a glance what's still in flight.
- **Auto-scroll to current week** when you open a job.
- **Inline edit job name + project start date** in the Settings popover.

CHANGED
- Brand palette switched from blue to **Blik Red** (#E11D2A) across selections, hovers, the dependency port, the today line, and the new logo.
- Type system: Inter for UI, JetBrains Mono for meta and dates.
- New sidebar header with the BP block monogram + BLIK Plan wordmark.

UNDER THE HOOD
- DB schema bumped to v5: added `job.region` column; `no_work_day.source` CHECK constraint broadened to include the 4 new region tags. Existing SA holidays were auto-migrated to the new `za_holiday` source.
- India + China use hard-coded annual lookup tables for lunar/lunisolar holidays (Diwali, Holi, Spring Festival, etc.), covering 2026–2030 in this release. Years beyond fall back to fixed Gregorian dates until tables are extended.
