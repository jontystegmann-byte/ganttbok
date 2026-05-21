Gantt Bok v1.1.1 — drag, dependency, holiday & settings fixes

FIXED
- Right-edge resize now actually grows the bar to the right. Previously the right handle was rerouting to the left edge and shifting the start instead.
- The dependency-creation circle no longer disappears when your mouse crosses the gap from the bar end to the port. Hover zone now bridges across.
- Bars physically split around public holidays now. When the holiday toggle is on, the bar breaks visually at each public holiday and the task automatically extends one extra day to compensate.
- Phase bars are more robust to tasks whose start dates fall before the project's stated start (the viewport now expands leftward so they're visible).

CHANGED
- Job-level settings (public-holiday split toggle) moved out of the right-hand panel and into a small ⚙ gear icon at the bottom-left. The right-hand panel now goes back to showing only when you select a task or phase. Duration unit (weeks/days) also lives in the new settings popover.
