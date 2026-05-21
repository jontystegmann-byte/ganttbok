use chrono::{Datelike, NaiveDate};
use super::sa_holidays::Holiday;

/// China — statutory public holidays per State Council annual notice (国务院办公厅通知).
///
/// Each year, the State Council publishes the exact observed dates including extensions
/// (3-day or 7-day clusters formed by transferring adjacent weekends to working days,
/// known as 调休). This table encodes the actual observed dates.
///
/// **Not modelled**: the makeup-working-weekend days (调休). The Gantt chart treats
/// the listed dates as no-work days only. Users wanting to mark transferred weekends
/// as working days can do so manually via right-click on the header strip.
///
/// Years beyond the table fall back to fixed-Gregorian-only.
pub fn cn_holidays(year: i32) -> Vec<Holiday> {
    let mut out: Vec<Holiday> = Vec::new();

    if let Some(table) = annual_table(year) {
        for &(m, d, name) in table {
            out.push(Holiday { date: NaiveDate::from_ymd_opt(year, m, d).unwrap(), name });
        }
    } else {
        // Fallback: just the fixed Gregorian anchors.
        for &(m, d, name) in &[
            (1u32, 1u32, "New Year's Day"),
            (5, 1, "Labour Day"),
            (10, 1, "National Day"),
        ] {
            out.push(Holiday { date: NaiveDate::from_ymd_opt(year, m, d).unwrap(), name });
        }
    }

    out.sort_by_key(|h| h.date);
    out
}

pub fn cn_holidays_for_range(from: NaiveDate, to: NaiveDate) -> Vec<Holiday> {
    if to < from { return vec![]; }
    let mut out = Vec::new();
    for y in from.year()..=to.year() {
        for h in cn_holidays(y) {
            if h.date >= from && h.date <= to { out.push(h); }
        }
    }
    out
}

/// Per-year observed-date table. Each entry covers all consecutive days of a holiday
/// cluster (e.g. Spring Festival is typically 7 days).
fn annual_table(year: i32) -> Option<&'static [(u32, u32, &'static str)]> {
    match year {
        // 2026 State Council notice (issued late 2025). Spring Festival Feb 17 = lunar New Year.
        2026 => Some(&[
            (1, 1,   "New Year's Day"),
            (2, 16,  "Spring Festival"),
            (2, 17,  "Spring Festival"),
            (2, 18,  "Spring Festival"),
            (2, 19,  "Spring Festival"),
            (2, 20,  "Spring Festival"),
            (2, 21,  "Spring Festival"),
            (2, 22,  "Spring Festival"),
            (4, 4,   "Qingming Festival"),
            (4, 5,   "Qingming Festival"),
            (4, 6,   "Qingming Festival"),
            (5, 1,   "Labour Day"),
            (5, 2,   "Labour Day"),
            (5, 3,   "Labour Day"),
            (5, 4,   "Labour Day"),
            (5, 5,   "Labour Day"),
            (6, 19,  "Dragon Boat Festival"),
            (6, 20,  "Dragon Boat Festival"),
            (6, 21,  "Dragon Boat Festival"),
            (9, 25,  "Mid-Autumn Festival"),
            (9, 26,  "Mid-Autumn Festival"),
            (9, 27,  "Mid-Autumn Festival"),
            (10, 1,  "National Day"),
            (10, 2,  "National Day"),
            (10, 3,  "National Day"),
            (10, 4,  "National Day"),
            (10, 5,  "National Day"),
            (10, 6,  "National Day"),
            (10, 7,  "National Day"),
        ]),
        // 2027 — Spring Festival Feb 6. Dates are projections — replace with the
        // official State Council notice once published (late 2026).
        2027 => Some(&[
            (1, 1,   "New Year's Day"),
            (2, 5,   "Spring Festival"),
            (2, 6,   "Spring Festival"),
            (2, 7,   "Spring Festival"),
            (2, 8,   "Spring Festival"),
            (2, 9,   "Spring Festival"),
            (2, 10,  "Spring Festival"),
            (2, 11,  "Spring Festival"),
            (4, 5,   "Qingming Festival"),
            (4, 6,   "Qingming Festival"),
            (4, 7,   "Qingming Festival"),
            (5, 1,   "Labour Day"),
            (5, 2,   "Labour Day"),
            (5, 3,   "Labour Day"),
            (5, 4,   "Labour Day"),
            (5, 5,   "Labour Day"),
            (6, 8,   "Dragon Boat Festival"),
            (6, 9,   "Dragon Boat Festival"),
            (6, 10,  "Dragon Boat Festival"),
            (9, 14,  "Mid-Autumn Festival"),
            (9, 15,  "Mid-Autumn Festival"),
            (9, 16,  "Mid-Autumn Festival"),
            (10, 1,  "National Day"),
            (10, 2,  "National Day"),
            (10, 3,  "National Day"),
            (10, 4,  "National Day"),
            (10, 5,  "National Day"),
            (10, 6,  "National Day"),
            (10, 7,  "National Day"),
        ]),
        // Additional years (2028–2035) should be added when the State Council notice
        // for each is published. Without a table, falls back to 3 fixed Gregorian dates.
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spring_festival_2026_covers_seven_days() {
        let h: Vec<NaiveDate> = cn_holidays(2026).into_iter().map(|x| x.date).collect();
        for d in 16..=22 {
            assert!(h.contains(&NaiveDate::from_ymd_opt(2026, 2, d).unwrap()),
                    "expected Feb {d} 2026 in Spring Festival cluster");
        }
    }

    #[test]
    fn national_day_2026_seven_days() {
        let h: Vec<NaiveDate> = cn_holidays(2026).into_iter().map(|x| x.date).collect();
        for d in 1..=7 {
            assert!(h.contains(&NaiveDate::from_ymd_opt(2026, 10, d).unwrap()),
                    "expected Oct {d} 2026 in National Day cluster");
        }
    }

    #[test]
    fn beyond_table_falls_back_to_three_fixed_dates() {
        let h = cn_holidays(2050);
        assert_eq!(h.len(), 3, "fallback gives 3 fixed dates");
    }
}
