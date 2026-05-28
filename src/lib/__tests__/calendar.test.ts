import { describe, it, expect } from 'vitest';
import { computeViewportDays, addCalendarDays, addWorkdays, snapToNearestWorkable } from '../calendar';
import type { Task } from '../types';

describe('calendar', () => {
  it('viewport starts on the Monday of project start week', () => {
    // Wed 2026-06-10. Monday of that week is 2026-06-08.
    const days = computeViewportDays('2026-06-10', []);
    expect(days[0].date).toBe('2026-06-08');
    expect(days[0].weekday).toBe('M');
    expect(days[0].projectWeekNumber).toBe(1);
  });

  it('viewport excludes weekends', () => {
    const days = computeViewportDays('2026-06-08', []);
    expect(days.every(d => ['M', 'T', 'W', 'F'].includes(d.weekday) || d.weekday === 'T')).toBe(true);
    expect(days.length % 5).toBe(0);
  });

  it('Monday of week N has projectWeekNumber N', () => {
    const days = computeViewportDays('2026-06-08', []);
    // Week 1 starts Mon 8 Jun. Week 2 Mon = 15 Jun.
    const mon2 = days.find(d => d.date === '2026-06-15');
    expect(mon2?.projectWeekNumber).toBe(2);
  });

  it('viewport extends past latest task end', () => {
    const tasks: Task[] = [
      { id: 1, phase_id: 1, name: 'T', start_date: '2026-08-15', duration_workdays: 5, order_index: 0, notes: null, contact_id: null, last_chaser_sent_at: null, status: 'on_track', completion_date: null },
    ];
    const days = computeViewportDays('2026-06-08', tasks);
    // Last day must be >= the task end (which is ~22 Aug).
    expect(days[days.length - 1].date >= '2026-08-22').toBe(true);
  });

  it('addWorkdays skips weekends', () => {
    expect(addWorkdays('2026-06-08', 5)).toBe('2026-06-15'); // Mon + 5 wd = next Mon
    expect(addWorkdays('2026-06-12', 1)).toBe('2026-06-15'); // Fri + 1 wd = Mon
  });

  it('addWorkdays handles negative shifts (drag-backwards bugfix)', () => {
    expect(addWorkdays('2026-06-15', -5)).toBe('2026-06-08'); // Mon - 5 wd = previous Mon
    expect(addWorkdays('2026-06-15', -1)).toBe('2026-06-12'); // Mon - 1 wd = previous Fri
    expect(addWorkdays('2026-06-15', 0)).toBe('2026-06-15');  // no shift
  });

  it('addCalendarDays advances literally', () => {
    expect(addCalendarDays('2026-06-08', 1)).toBe('2026-06-09');
    expect(addCalendarDays('2026-06-08', 7)).toBe('2026-06-15');
  });
});

describe('snapToNearestWorkable', () => {
  it('returns the date itself when already workable', () => {
    // Wed 2026-06-10 is a workday; not in noWorkSet.
    expect(snapToNearestWorkable('2026-06-10', new Set(), false)).toBe('2026-06-10');
  });

  it('snaps Saturday to the nearest Friday (weekends excluded)', () => {
    // Sat 2026-06-13 — nearest workable is Fri 2026-06-12 (1 day back, vs Mon = 2 forward).
    expect(snapToNearestWorkable('2026-06-13', new Set(), false)).toBe('2026-06-12');
  });

  it('snaps Sunday to the nearest Monday (weekends excluded)', () => {
    // Sun 2026-06-14 — Mon 2026-06-15 is 1 forward, Fri 2026-06-12 is 2 back. Mon wins.
    expect(snapToNearestWorkable('2026-06-14', new Set(), false)).toBe('2026-06-15');
  });

  it('skips a no-work day to the nearest workable date', () => {
    // ZA Freedom Day 2026-04-27 is a Monday.
    // Nearest workable: Tue 2026-04-28 (1 forward) vs Fri 2026-04-24 (3 back). Tue wins.
    const noWork = new Set(['2026-04-27']);
    expect(snapToNearestWorkable('2026-04-27', noWork, false)).toBe('2026-04-28');
  });

  it('treats Saturday as workable when includeWeekends is true', () => {
    expect(snapToNearestWorkable('2026-06-13', new Set(), true)).toBe('2026-06-13');
  });

  it('returns the date unchanged for dates outside ±90 days walk', () => {
    // No workable day within 90 days — defensive fallback. Construct by marking everything.
    const noWork = new Set<string>();
    const start = '2026-06-10';
    // Mark 200 days around start as no-work; weekends already excluded.
    for (let i = -200; i <= 200; i++) {
      const [y, m, d] = start.split('-').map(Number);
      const dt = new Date(Date.UTC(y, m - 1, d + i));
      noWork.add(dt.toISOString().slice(0, 10));
    }
    expect(snapToNearestWorkable(start, noWork, false)).toBe(start);
  });
});
