import type { Task } from './types';

export interface ViewportDay {
  date: string;                // YYYY-MM-DD
  weekday: 'M' | 'T' | 'W' | 'T' | 'F';
  dayOfMonth: number;
  projectWeekNumber: number;   // 1-indexed from the Monday of the project's start week
}

const WEEKDAY_LETTERS = ['M', 'T', 'W', 'T', 'F'] as const;

function parse(iso: string): Date {
  // Parse as UTC to avoid timezone drift on the date math.
  const [y, m, d] = iso.split('-').map(Number);
  return new Date(Date.UTC(y, m - 1, d));
}

function fmt(d: Date): string {
  return d.toISOString().slice(0, 10);
}

function mondayOfWeek(d: Date): Date {
  // JS getUTCDay: Sun=0, Mon=1 ... Sat=6.
  const day = d.getUTCDay();
  const offset = day === 0 ? -6 : 1 - day; // shift back to Monday
  return new Date(d.getTime() + offset * 86400000);
}

function isWorkday(d: Date): boolean {
  const day = d.getUTCDay();
  return day >= 1 && day <= 5;
}

export function addCalendarDays(iso: string, n: number): string {
  const d = parse(iso);
  d.setUTCDate(d.getUTCDate() + n);
  return fmt(d);
}

export function addWorkdays(iso: string, n: number): string {
  let d = parse(iso);
  // Snap to a workday if we landed on a weekend. Direction follows the sign of n
  // (positive/zero → snap forward, negative → snap backward).
  const snapDir = n >= 0 ? 1 : -1;
  while (!isWorkday(d)) {
    d.setUTCDate(d.getUTCDate() + snapDir);
  }
  if (n === 0) return fmt(d);
  const step = n > 0 ? 1 : -1;
  let remaining = Math.abs(n);
  while (remaining > 0) {
    d.setUTCDate(d.getUTCDate() + step);
    if (isWorkday(d)) remaining--;
  }
  return fmt(d);
}

/**
 * Compute the actual workdays a task occupies, optionally skipping no-work days.
 * Returns an array of ISO date strings of length `durationWorkdays`.
 * If `skipNoWork=true`, holidays/manual no-work days are stepped over (and the task extends further into the future).
 */
export function occupiedWorkdays(
  startDate: string,
  durationWorkdays: number,
  noWorkSet: Set<string>,
  skipNoWork: boolean,
): string[] {
  const out: string[] = [];
  const d = parse(startDate);
  // Snap forward to the next valid workday if we landed on a weekend or no-work day.
  while (!isWorkday(d) || (skipNoWork && noWorkSet.has(fmt(d)))) {
    d.setUTCDate(d.getUTCDate() + 1);
  }
  while (out.length < durationWorkdays) {
    const iso = fmt(d);
    if (isWorkday(d) && (!skipNoWork || !noWorkSet.has(iso))) {
      out.push(iso);
    }
    d.setUTCDate(d.getUTCDate() + 1);
  }
  return out;
}

/** Group consecutive viewport indices into runs: e.g. [0,1,2,5,6] → [{start:0,len:3},{start:5,len:2}]. */
export function groupConsecutive(indices: number[]): { start: number; len: number }[] {
  if (indices.length === 0) return [];
  const sorted = [...indices].sort((a, b) => a - b);
  const runs: { start: number; len: number }[] = [];
  let runStart = sorted[0];
  let runLen = 1;
  for (let i = 1; i < sorted.length; i++) {
    if (sorted[i] === sorted[i - 1] + 1) {
      runLen++;
    } else {
      runs.push({ start: runStart, len: runLen });
      runStart = sorted[i];
      runLen = 1;
    }
  }
  runs.push({ start: runStart, len: runLen });
  return runs;
}

export function computeViewportDays(projectStart: string, tasks: Task[]): ViewportDay[] {
  // Start at the earliest of (project_start, any task's start_date) so tasks dragged
  // before the project_start_date are still visible in the viewport.
  let earliestStart = parse(projectStart);
  for (const t of tasks) {
    const ts = parse(t.start_date);
    if (ts < earliestStart) earliestStart = ts;
  }
  const start = mondayOfWeek(earliestStart);

  // Compute the latest end so the viewport is wide enough.
  let latestEnd = parse(projectStart);
  for (const t of tasks) {
    const end = parse(addWorkdays(t.start_date, Math.max(0, t.duration_workdays - 1)));
    if (end > latestEnd) latestEnd = end;
  }
  // Pad 4 weeks beyond the latest task end (28 days).
  latestEnd.setUTCDate(latestEnd.getUTCDate() + 28);
  // Round latestEnd forward to the Friday of its week so viewport contains whole weeks.
  // getUTCDay: Mon=1..Fri=5..Sun=0
  const dow = latestEnd.getUTCDay();
  if (dow === 0) {
    latestEnd.setUTCDate(latestEnd.getUTCDate() - 2); // Sun -> previous Fri
  } else if (dow === 6) {
    latestEnd.setUTCDate(latestEnd.getUTCDate() - 1); // Sat -> previous Fri
  } else if (dow < 5) {
    latestEnd.setUTCDate(latestEnd.getUTCDate() + (5 - dow)); // forward to Friday
  }

  const days: ViewportDay[] = [];
  let cur = new Date(start);
  let weekNum = 1;
  let weekdayIdx = 0;
  while (cur <= latestEnd) {
    if (isWorkday(cur)) {
      days.push({
        date: fmt(cur),
        weekday: WEEKDAY_LETTERS[weekdayIdx],
        dayOfMonth: cur.getUTCDate(),
        projectWeekNumber: weekNum,
      });
      weekdayIdx++;
      if (weekdayIdx === 5) {
        weekdayIdx = 0;
        weekNum++;
      }
    }
    cur.setUTCDate(cur.getUTCDate() + 1);
  }
  return days;
}
