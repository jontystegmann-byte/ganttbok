use chrono::{Datelike, NaiveDate, Weekday};
use super::sa_holidays::Holiday;

/// US federal holidays for `year`. 11 since Juneteenth was added in 2021.
///
/// Observation rule (federal practice): a fixed-date holiday falling on Saturday
/// is observed the preceding Friday; on Sunday, the following Monday. Floating
/// Monday/Thursday holidays are already on weekdays by definition.
pub fn us_holidays(year: i32) -> Vec<Holiday> {
    let mut out: Vec<Holiday> = Vec::with_capacity(11);

    // Fixed-date federal holidays — observed-date shifts apply.
    for &(m, d, name) in &[
        (1u32, 1u32,  "New Year's Day"),
        (6,    19,    "Juneteenth"),
        (7,    4,     "Independence Day"),
        (11,   11,    "Veterans Day"),
        (12,   25,    "Christmas Day"),
    ] {
        let raw = NaiveDate::from_ymd_opt(year, m, d).unwrap();
        let observed = match raw.weekday() {
            Weekday::Sat => raw - chrono::Duration::days(1),
            Weekday::Sun => raw + chrono::Duration::days(1),
            _ => raw,
        };
        out.push(Holiday { date: observed, name });
    }

    // Floating Monday holidays.
    out.push(Holiday { date: nth_weekday(year, 1,  Weekday::Mon, 3),     name: "Martin Luther King Jr. Day" });
    out.push(Holiday { date: nth_weekday(year, 2,  Weekday::Mon, 3),     name: "Presidents' Day" });
    out.push(Holiday { date: last_weekday(year, 5, Weekday::Mon),         name: "Memorial Day" });
    out.push(Holiday { date: nth_weekday(year, 9,  Weekday::Mon, 1),     name: "Labor Day" });
    out.push(Holiday { date: nth_weekday(year, 10, Weekday::Mon, 2),     name: "Columbus Day" });

    // Floating Thursday holiday.
    out.push(Holiday { date: nth_weekday(year, 11, Weekday::Thu, 4),     name: "Thanksgiving Day" });

    out.sort_by_key(|h| h.date);
    out
}

pub fn us_holidays_for_range(from: NaiveDate, to: NaiveDate) -> Vec<Holiday> {
    if to < from { return vec![]; }
    let mut out = Vec::new();
    for y in from.year()..=to.year() {
        for h in us_holidays(y) {
            if h.date >= from && h.date <= to { out.push(h); }
        }
    }
    out
}

/// Nth occurrence of `weekday` in (year, month). n=1 → first, 2 → second, etc.
fn nth_weekday(year: i32, month: u32, weekday: Weekday, n: u32) -> NaiveDate {
    let first = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
    let offset = (7 + weekday.num_days_from_monday() as i32 - first.weekday().num_days_from_monday() as i32) % 7;
    first + chrono::Duration::days(offset as i64 + 7 * (n as i64 - 1))
}

/// Last occurrence of `weekday` in (year, month).
fn last_weekday(year: i32, month: u32, weekday: Weekday) -> NaiveDate {
    // Walk to the first of the next month, step back to the desired weekday.
    let next_month_first = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap()
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1).unwrap()
    };
    let last_of_month = next_month_first - chrono::Duration::days(1);
    let offset = (7 + last_of_month.weekday().num_days_from_monday() as i32 - weekday.num_days_from_monday() as i32) % 7;
    last_of_month - chrono::Duration::days(offset as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eleven_holidays_per_year() {
        assert_eq!(us_holidays(2026).len(), 11);
    }

    #[test]
    fn fixed_holidays_in_2026() {
        let h: Vec<NaiveDate> = us_holidays(2026).into_iter().map(|x| x.date).collect();
        // New Year's Jan 1 is Thu → Jan 1
        assert!(h.contains(&NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()));
        // Juneteenth Jun 19 is Fri → Jun 19
        assert!(h.contains(&NaiveDate::from_ymd_opt(2026, 6, 19).unwrap()));
        // Independence Day Jul 4 is Saturday → observed Friday Jul 3
        assert!(h.contains(&NaiveDate::from_ymd_opt(2026, 7, 3).unwrap()));
        assert!(!h.contains(&NaiveDate::from_ymd_opt(2026, 7, 4).unwrap()));
        // Veterans Day Nov 11 is Wed → Nov 11
        assert!(h.contains(&NaiveDate::from_ymd_opt(2026, 11, 11).unwrap()));
        // Christmas Dec 25 is Friday → Dec 25
        assert!(h.contains(&NaiveDate::from_ymd_opt(2026, 12, 25).unwrap()));
    }

    #[test]
    fn floating_holidays_in_2026() {
        let h: Vec<NaiveDate> = us_holidays(2026).into_iter().map(|x| x.date).collect();
        assert!(h.contains(&NaiveDate::from_ymd_opt(2026, 1, 19).unwrap()));  // MLK
        assert!(h.contains(&NaiveDate::from_ymd_opt(2026, 2, 16).unwrap()));  // Presidents'
        assert!(h.contains(&NaiveDate::from_ymd_opt(2026, 5, 25).unwrap()));  // Memorial
        assert!(h.contains(&NaiveDate::from_ymd_opt(2026, 9, 7).unwrap()));   // Labor
        assert!(h.contains(&NaiveDate::from_ymd_opt(2026, 10, 12).unwrap())); // Columbus
        assert!(h.contains(&NaiveDate::from_ymd_opt(2026, 11, 26).unwrap())); // Thanksgiving
    }

    #[test]
    fn new_years_sunday_shifts_to_monday() {
        // 2023-01-01 was a Sunday → observed Mon 2023-01-02
        let h: Vec<NaiveDate> = us_holidays(2023).into_iter().map(|x| x.date).collect();
        assert!(h.contains(&NaiveDate::from_ymd_opt(2023, 1, 2).unwrap()));
        assert!(!h.contains(&NaiveDate::from_ymd_opt(2023, 1, 1).unwrap()));
    }

    #[test]
    fn christmas_saturday_shifts_to_friday() {
        // 2021-12-25 was a Saturday → observed Fri 2021-12-24
        let h: Vec<NaiveDate> = us_holidays(2021).into_iter().map(|x| x.date).collect();
        assert!(h.contains(&NaiveDate::from_ymd_opt(2021, 12, 24).unwrap()));
        assert!(!h.contains(&NaiveDate::from_ymd_opt(2021, 12, 25).unwrap()));
    }
}
