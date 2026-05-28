import type { Task } from './types';

export interface ViewportDay {
  date: string;                // YYYY-MM-DD
  weekday: 'M' | 'T' | 'W' | 'T' | 'F' | 'S';
  dayOfMonth: number;
  projectWeekNumber: number;   // 1-indexed from the Monday of the project's start week
  isWeekend: boolean;
}

const WEEKDAY_LETTERS = ['M', 'T', 'W', 'T', 'F'] as const;
const ALL_DAY_LETTERS = ['M', 'T', 'W', 'T', 'F', 'S', 'S'] as const;

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

/** Mon–Fri only. Used internally; weekend-inclusive mode bypasses this. */
function isMonFri(d: Date): boolean {
  const day = d.getUTCDay();
  return day >= 1 && day <= 5;
}

/** Is `d` a workable day, given the includeWeekends setting? */
function isWorkable(d: Date, includeWeekends: boolean): boolean {
  return includeWeekends || isMonFri(d);
}

export function addCalendarDays(iso: string, n: number): string {
  const d = parse(iso);
  d.setUTCDate(d.getUTCDate() + n);
  return fmt(d);
}

/**
 * Snap an ISO date to the nearest workable day, looking ±90 days symmetrically.
 * "Workable" means: weekend-skipped iff !includeWeekends, AND not in noWorkSet.
 * Returns the original date if no workable day is found in range (defensive — never throws).
 */
export function snapToNearestWorkable(
  iso: string,
  noWorkSet: Set<string>,
  includeWeekends: boolean,
): string {
  const isWorkable = (candidate: string): boolean => {
    const d = parse(candidate);
    if (!includeWeekends && !isMonFri(d)) return false;
    if (noWorkSet.has(candidate)) return false;
    return true;
  };
  if (isWorkable(iso)) return iso;
  for (let delta = 1; delta <= 90; delta++) {
    const forward = addCalendarDays(iso, delta);
    const backward = addCalendarDays(iso, -delta);
    const fwdOk = isWorkable(forward);
    const bwdOk = isWorkable(backward);
    if (fwdOk && bwdOk) return forward; // ties go forward (matches user "next workday" intuition)
    if (fwdOk) return forward;
    if (bwdOk) return backward;
  }
  return iso;
}

export function addWorkdays(iso: string, n: number, includeWeekends: boolean = false): string {
  const d = parse(iso);
  const snapDir = n >= 0 ? 1 : -1;
  while (!isWorkable(d, includeWeekends)) {
    d.setUTCDate(d.getUTCDate() + snapDir);
  }
  if (n === 0) return fmt(d);
  const step = n > 0 ? 1 : -1;
  let remaining = Math.abs(n);
  while (remaining > 0) {
    d.setUTCDate(d.getUTCDate() + step);
    if (isWorkable(d, includeWeekends)) remaining--;
  }
  return fmt(d);
}

/**
 * Compute the actual workdays a task occupies. Returns ISO dates of length durationWorkdays.
 * Weekends are workable iff includeWeekends. SA holidays / manual no-work days are skipped
 * iff skipNoWork.
 */
export function occupiedWorkdays(
  startDate: string,
  durationWorkdays: number,
  noWorkSet: Set<string>,
  skipNoWork: boolean,
  includeWeekends: boolean = false,
): string[] {
  const out: string[] = [];
  const d = parse(startDate);
  while (!isWorkable(d, includeWeekends) || (skipNoWork && noWorkSet.has(fmt(d)))) {
    d.setUTCDate(d.getUTCDate() + 1);
  }
  while (out.length < durationWorkdays) {
    const iso = fmt(d);
    if (isWorkable(d, includeWeekends) && (!skipNoWork || !noWorkSet.has(iso))) {
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

export function computeViewportDays(
  projectStart: string,
  tasks: Task[],
  includeWeekends: boolean = false,
): ViewportDay[] {
  // Start at the earliest of (project_start, any task's start_date) so tasks dragged
  // before the project_start_date are still visible in the viewport.
  let earliestStart = parse(projectStart);
  for (const t of tasks) {
    const ts = parse(t.start_date);
    if (ts < earliestStart) earliestStart = ts;
  }
  const start = mondayOfWeek(earliestStart);

  let latestEnd = parse(projectStart);
  for (const t of tasks) {
    const end = parse(addWorkdays(t.start_date, Math.max(0, t.duration_workdays - 1), includeWeekends));
    if (end > latestEnd) latestEnd = end;
  }
  // Pad ~4 weeks beyond the latest task end.
  latestEnd.setUTCDate(latestEnd.getUTCDate() + 28);
  // Round latestEnd forward to Sunday so viewport contains whole weeks.
  const dow = latestEnd.getUTCDay();
  if (dow !== 0) {
    latestEnd.setUTCDate(latestEnd.getUTCDate() + (7 - dow));
  }

  const days: ViewportDay[] = [];
  const cur = new Date(start);
  let weekNum = 1;
  let weekdayIdx = 0;
  while (cur <= latestEnd) {
    const isWknd = cur.getUTCDay() === 0 || cur.getUTCDay() === 6;
    const include = includeWeekends || !isWknd;
    if (include) {
      const letters = includeWeekends ? ALL_DAY_LETTERS : WEEKDAY_LETTERS;
      days.push({
        date: fmt(cur),
        weekday: letters[weekdayIdx] as ViewportDay['weekday'],
        dayOfMonth: cur.getUTCDate(),
        projectWeekNumber: weekNum,
        isWeekend: isWknd,
      });
      weekdayIdx++;
      const wkLen = includeWeekends ? 7 : 5;
      if (weekdayIdx === wkLen) {
        weekdayIdx = 0;
        weekNum++;
      }
    }
    cur.setUTCDate(cur.getUTCDate() + 1);
  }
  return days;
}
