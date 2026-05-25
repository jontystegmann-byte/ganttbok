import { describe, it, expect } from 'vitest';
import { computeViewportDays, addCalendarDays, addWorkdays } from '../calendar';
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
