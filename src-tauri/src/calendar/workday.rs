use chrono::{Datelike, Duration, NaiveDate, Weekday};

/// Add `n` workdays (Mon–Fri only — no holiday awareness yet) to `from`.
/// `from` itself counts as day 0 of work iff it is a workday; otherwise advance to the next workday first.
pub fn add_workdays(from: NaiveDate, n: i64) -> NaiveDate {
    let mut cur = from;
    while !is_workday(cur) {
        cur += Duration::days(1);
    }
    if n <= 0 {
        return cur;
    }
    let mut remaining = n;
    while remaining > 0 {
        cur += Duration::days(1);
        if is_workday(cur) {
            remaining -= 1;
        }
    }
    cur
}

/// Inclusive workday count between `start` and `end` (both inclusive).
/// `end < start` returns 0. Sat/Sun are not counted.
pub fn count_workdays(start: NaiveDate, end: NaiveDate) -> i64 {
    if end < start {
        return 0;
    }
    let mut cur = start;
    let mut n: i64 = 0;
    while cur <= end {
        if is_workday(cur) {
            n += 1;
        }
        cur += Duration::days(1);
    }
    n
}

pub fn is_workday(d: NaiveDate) -> bool {
    !matches!(d.weekday(), Weekday::Sat | Weekday::Sun)
}

use std::collections::HashSet;

pub fn add_workdays_excluding(
    from: NaiveDate,
    n: i64,
    excluded: &HashSet<NaiveDate>,
) -> NaiveDate {
    let is_work = |d: NaiveDate| is_workday(d) && !excluded.contains(&d);
    let mut cur = from;
    while !is_work(cur) {
        cur += Duration::days(1);
    }
    if n <= 0 { return cur; }
    let mut remaining = n;
    while remaining > 0 {
        cur += Duration::days(1);
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

    #[test]
    fn add_zero_workdays_returns_same_day_if_workday() {
        assert_eq!(add_workdays(d(2026, 6, 8), 0), d(2026, 6, 8)); // Mon
    }

    #[test]
    fn add_zero_workdays_from_saturday_advances_to_monday() {
        assert_eq!(add_workdays(d(2026, 6, 6), 0), d(2026, 6, 8)); // Sat -> Mon
    }

    #[test]
    fn add_three_workdays_from_monday_lands_on_thursday() {
        assert_eq!(add_workdays(d(2026, 6, 8), 3), d(2026, 6, 11)); // Mon +3 -> Thu
    }

    #[test]
    fn add_five_workdays_from_monday_skips_weekend() {
        assert_eq!(add_workdays(d(2026, 6, 8), 5), d(2026, 6, 15)); // Mon -> next Mon
    }

    #[test]
    fn count_workdays_inclusive_week() {
        assert_eq!(count_workdays(d(2026, 6, 8), d(2026, 6, 12)), 5); // Mon-Fri
    }

    #[test]
    fn count_workdays_skips_weekend_in_middle() {
        assert_eq!(count_workdays(d(2026, 6, 8), d(2026, 6, 15)), 6); // Mon-Mon = 6 workdays
    }

    #[test]
    fn count_workdays_reverse_returns_zero() {
        assert_eq!(count_workdays(d(2026, 6, 15), d(2026, 6, 8)), 0);
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
        );
        assert_eq!(result, NaiveDate::from_ymd_opt(2026,6,19).unwrap());
    }
}
