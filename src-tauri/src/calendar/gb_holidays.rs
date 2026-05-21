use chrono::{Datelike, Duration, NaiveDate, Weekday};
use super::easter::easter_sunday;
use super::sa_holidays::Holiday;

/// England & Wales bank holidays. 8 per year.
///
/// **Observation rule**: if Christmas (Dec 25), Boxing Day (Dec 26) or New Year's Day (Jan 1)
/// falls on a weekend, a "substitute" bank holiday is granted on the next available weekday.
/// Substitutes cascade — if Dec 25 is Saturday, Boxing Day substitute is the Monday Dec 28
/// (Sunday Dec 27 already took Monday Dec 27 for Christmas → cascade to Tuesday Dec 29 for
/// Boxing Day actually... wait that's wrong).
///
/// Correct UK rule per gov.uk: when Dec 25 is Saturday and Dec 26 is Sunday:
///   - Christmas substitute = Monday Dec 27
///   - Boxing Day substitute = Tuesday Dec 28
/// So substitutes can take two consecutive weekdays after Christmas weekend.
pub fn gb_holidays(year: i32) -> Vec<Holiday> {
    let mut out: Vec<Holiday> = Vec::with_capacity(8);

    let easter = easter_sunday(year);
    out.push(Holiday { date: easter - Duration::days(2), name: "Good Friday" });
    out.push(Holiday { date: easter + Duration::days(1), name: "Easter Monday" });

    // Floating Mondays
    out.push(Holiday { date: nth_weekday(year, 5,  Weekday::Mon, 1),  name: "Early May Bank Holiday" });
    out.push(Holiday { date: last_weekday(year, 5, Weekday::Mon),     name: "Spring Bank Holiday" });
    out.push(Holiday { date: last_weekday(year, 8, Weekday::Mon),     name: "Summer Bank Holiday" });

    // Fixed dates with weekend cascade
    let ny = NaiveDate::from_ymd_opt(year, 1, 1).unwrap();
    let xmas = NaiveDate::from_ymd_opt(year, 12, 25).unwrap();
    let boxing = NaiveDate::from_ymd_opt(year, 12, 26).unwrap();

    out.push(Holiday { date: weekend_forward(ny), name: "New Year's Day" });

    // Christmas + Boxing Day cascade
    let xmas_obs = weekend_forward(xmas);
    let mut boxing_obs = weekend_forward(boxing);
    if boxing_obs == xmas_obs {
        // Both shifted to the same weekday → push Boxing Day one more day forward
        boxing_obs += Duration::days(1);
    }
    out.push(Holiday { date: xmas_obs,   name: "Christmas Day" });
    out.push(Holiday { date: boxing_obs, name: "Boxing Day" });

    out.sort_by_key(|h| h.date);
    out
}

pub fn gb_holidays_for_range(from: NaiveDate, to: NaiveDate) -> Vec<Holiday> {
    if to < from { return vec![]; }
    let mut out = Vec::new();
    for y in from.year()..=to.year() {
        for h in gb_holidays(y) {
            if h.date >= from && h.date <= to { out.push(h); }
        }
    }
    out
}

/// Roll a weekend date forward to the next Monday.
fn weekend_forward(d: NaiveDate) -> NaiveDate {
    match d.weekday() {
        Weekday::Sat => d + Duration::days(2),
        Weekday::Sun => d + Duration::days(1),
        _ => d,
    }
}

fn nth_weekday(year: i32, month: u32, weekday: Weekday, n: u32) -> NaiveDate {
    let first = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
    let offset = (7 + weekday.num_days_from_monday() as i32 - first.weekday().num_days_from_monday() as i32) % 7;
    first + Duration::days(offset as i64 + 7 * (n as i64 - 1))
}

fn last_weekday(year: i32, month: u32, weekday: Weekday) -> NaiveDate {
    let next_month_first = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap()
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1).unwrap()
    };
    let last_of_month = next_month_first - Duration::days(1);
    let offset = (7 + last_of_month.weekday().num_days_from_monday() as i32 - weekday.num_days_from_monday() as i32) % 7;
    last_of_month - Duration::days(offset as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eight_holidays_per_year() {
        assert_eq!(gb_holidays(2026).len(), 8);
    }

    #[test]
    fn holidays_in_2026() {
        let h: Vec<NaiveDate> = gb_holidays(2026).into_iter().map(|x| x.date).collect();
        assert!(h.contains(&NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()));   // New Year (Thu)
        assert!(h.contains(&NaiveDate::from_ymd_opt(2026, 4, 3).unwrap()));   // Good Friday
        assert!(h.contains(&NaiveDate::from_ymd_opt(2026, 4, 6).unwrap()));   // Easter Monday
        assert!(h.contains(&NaiveDate::from_ymd_opt(2026, 5, 4).unwrap()));   // Early May
        assert!(h.contains(&NaiveDate::from_ymd_opt(2026, 5, 25).unwrap()));  // Spring
        assert!(h.contains(&NaiveDate::from_ymd_opt(2026, 8, 31).unwrap()));  // Summer
        assert!(h.contains(&NaiveDate::from_ymd_opt(2026, 12, 25).unwrap())); // Christmas (Fri)
        // Boxing Day Dec 26 2026 is Saturday → observed Monday Dec 28
        assert!(h.contains(&NaiveDate::from_ymd_opt(2026, 12, 28).unwrap()));
        assert!(!h.contains(&NaiveDate::from_ymd_opt(2026, 12, 26).unwrap()));
    }

    #[test]
    fn double_christmas_weekend_cascade() {
        // 2021: Dec 25 (Sat) → Mon 27; Dec 26 (Sun) → would be Mon 27 (collision) → Tue 28.
        let h: Vec<NaiveDate> = gb_holidays(2021).into_iter().map(|x| x.date).collect();
        assert!(h.contains(&NaiveDate::from_ymd_opt(2021, 12, 27).unwrap()),
                "Christmas substitute Mon 27 Dec");
        assert!(h.contains(&NaiveDate::from_ymd_opt(2021, 12, 28).unwrap()),
                "Boxing Day substitute Tue 28 Dec");
    }

    #[test]
    fn new_year_sunday_shifts_to_monday() {
        // 2023-01-01 was Sunday → observed Mon Jan 2
        let h: Vec<NaiveDate> = gb_holidays(2023).into_iter().map(|x| x.date).collect();
        assert!(h.contains(&NaiveDate::from_ymd_opt(2023, 1, 2).unwrap()));
        assert!(!h.contains(&NaiveDate::from_ymd_opt(2023, 1, 1).unwrap()));
    }
}
