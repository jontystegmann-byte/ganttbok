use chrono::{Datelike, Duration, NaiveDate, Weekday};

/// Returns true if `d` counts as a workday.
/// When `include_weekends` is true, Saturday and Sunday are treated as workdays.
pub fn is_workday(d: NaiveDate, include_weekends: bool) -> bool {
    if include_weekends {
        true
    } else {
        !matches!(d.weekday(), Weekday::Sat | Weekday::Sun)
    }
}

/// Add `n` workdays to `from`.
/// When `include_weekends` is true, Sat and Sun count as workdays (only
/// no-work-day entries cause skips downstream in `add_workdays_excluding`).
/// `from` itself counts as day 0; if it is not a workday, advance first.
pub fn add_workdays(from: NaiveDate, n: i64, include_weekends: bool) -> NaiveDate {
    let mut cur = from;
    // Snap to a workday if we landed on a non-workday.
    let snap_dir: i64 = if n >= 0 { 1 } else { -1 };
    while !is_workday(cur, include_weekends) {
        cur += Duration::days(snap_dir);
    }
    if n == 0 {
        return cur;
    }
    let step: i64 = if n > 0 { 1 } else { -1 };
    let mut remaining = n.abs();
    while remaining > 0 {
        cur += Duration::days(step);
        if is_workday(cur, include_weekends) {
            remaining -= 1;
        }
    }
    cur
}

/// Inclusive workday count between `start` and `end` (both inclusive).
/// `end < start` returns 0.
/// When `include_weekends` is true, every calendar day counts as a workday.
pub fn count_workdays(start: NaiveDate, end: NaiveDate, include_weekends: bool) -> i64 {
    if end < start {
        return 0;
    }
    let mut cur = start;
    let mut n: i64 = 0;
    while cur <= end {
        if is_workday(cur, include_weekends) {
            n += 1;
        }
        cur += Duration::days(1);
    }
    n
}

use std::collections::HashSet;

pub fn add_workdays_excluding(
    from: NaiveDate,
    n: i64,
    excluded: &HashSet<NaiveDate>,
    include_weekends: bool,
) -> NaiveDate {
    let is_work = |d: NaiveDate| is_workday(d, include_weekends) && !excluded.contains(&d);
    let mut cur = from;
    let snap_dir: i64 = if n >= 0 { 1 } else { -1 };
    while !is_work(cur) {
        cur += Duration::days(snap_dir);
    }
    if n == 0 { return cur; }
    let step: i64 = if n > 0 { 1 } else { -1 };
    let mut remaining = n.abs();
    while remaining > 0 {
        cur += Duration::days(step);
        if is_work(cur) { remaining -= 1; }
    }
    cur
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    // ── existing tests (weekends-off / include_weekends=false) ────────────────

    #[test]
    fn add_zero_workdays_returns_same_day_if_workday() {
        assert_eq!(add_workdays(d(2026, 6, 8), 0, false), d(2026, 6, 8)); // Mon
    }

    #[test]
    fn add_zero_workdays_from_saturday_advances_to_monday() {
        assert_eq!(add_workdays(d(2026, 6, 6), 0, false), d(2026, 6, 8)); // Sat -> Mon
    }

    #[test]
    fn add_three_workdays_from_monday_lands_on_thursday() {
        assert_eq!(add_workdays(d(2026, 6, 8), 3, false), d(2026, 6, 11)); // Mon +3 -> Thu
    }

    #[test]
    fn add_five_workdays_from_monday_skips_weekend() {
        assert_eq!(add_workdays(d(2026, 6, 8), 5, false), d(2026, 6, 15)); // Mon -> next Mon
    }

    #[test]
    fn add_negative_workdays_goes_backwards() {
        // Mon 15 Jun - 5 workdays = previous Mon 8 Jun
        assert_eq!(add_workdays(d(2026, 6, 15), -5, false), d(2026, 6, 8));
        // Mon 15 Jun - 1 workday = previous Fri 12 Jun
        assert_eq!(add_workdays(d(2026, 6, 15), -1, false), d(2026, 6, 12));
        // Sat - 0 workdays snaps backward to previous Friday (was previously forward to Mon)
        assert_eq!(add_workdays(d(2026, 6, 6), -1, false), d(2026, 6, 4)); // Sat → Fri → Thu (1 step back)
    }

    #[test]
    fn count_workdays_inclusive_week() {
        assert_eq!(count_workdays(d(2026, 6, 8), d(2026, 6, 12), false), 5); // Mon-Fri
    }

    #[test]
    fn count_workdays_skips_weekend_in_middle() {
        assert_eq!(count_workdays(d(2026, 6, 8), d(2026, 6, 15), false), 6); // Mon-Mon = 6 workdays
    }

    #[test]
    fn count_workdays_reverse_returns_zero() {
        assert_eq!(count_workdays(d(2026, 6, 15), d(2026, 6, 8), false), 0);
    }

    #[test]
    fn add_workdays_excluding_skips_holiday_in_path() {
        use std::collections::HashSet;
        let mut hol = HashSet::new();
        hol.insert(NaiveDate::from_ymd_opt(2026,6,16).unwrap()); // Youth Day, Tue
        // Mon 15 Jun + 3 workdays = Thu 18 Jun normally; with Tue blocked → Fri 19 Jun
        let result = add_workdays_excluding(
            NaiveDate::from_ymd_opt(2026,6,15).unwrap(),
            3,
            &hol,
            false,
        );
        assert_eq!(result, NaiveDate::from_ymd_opt(2026,6,19).unwrap());
    }

    // ── new tests: include_weekends=true ─────────────────────────────────────

    #[test]
    fn is_workday_respects_flag() {
        assert!(is_workday(d(2026, 6, 13), true));   // Sat workable when flag on
        assert!(!is_workday(d(2026, 6, 13), false)); // Sat not workable when flag off
        assert!(is_workday(d(2026, 6, 14), true));   // Sun workable when flag on
        assert!(!is_workday(d(2026, 6, 14), false)); // Sun not workable when flag off
        assert!(is_workday(d(2026, 6, 8), true));    // Mon always workable
        assert!(is_workday(d(2026, 6, 8), false));   // Mon always workable
    }

    #[test]
    fn add_workdays_with_weekends_treats_sat_sun_as_workdays() {
        // Mon +1 with include_weekends=true → Tue (unchanged)
        assert_eq!(add_workdays(d(2026, 6, 8), 1, true), d(2026, 6, 9));
        // Fri +1 with include_weekends=true → Sat (was Mon under false)
        assert_eq!(add_workdays(d(2026, 6, 12), 1, true), d(2026, 6, 13));
        // Fri +2 with include_weekends=true → Sun
        assert_eq!(add_workdays(d(2026, 6, 12), 2, true), d(2026, 6, 14));
    }

    #[test]
    fn count_workdays_with_weekends_counts_all_days() {
        // Mon-Sun with weekends: 7 days inclusive
        assert_eq!(count_workdays(d(2026, 6, 8), d(2026, 6, 14), true), 7);
        // Mon-Sun without weekends: 5 days (Mon-Fri only)
        assert_eq!(count_workdays(d(2026, 6, 8), d(2026, 6, 14), false), 5);
    }

    #[test]
    fn add_workdays_excluding_treats_weekend_in_set_as_skipped_when_flag_on() {
        // include_weekends=true, but the specific Sat is in excluded → skip it
        let mut excluded = std::collections::HashSet::new();
        excluded.insert(d(2026, 6, 13)); // Sat in excluded
        // Fri +1 with flag on should land on Sun (skipping the excluded Sat).
        assert_eq!(add_workdays_excluding(d(2026, 6, 12), 1, &excluded, true), d(2026, 6, 14));
    }
}
