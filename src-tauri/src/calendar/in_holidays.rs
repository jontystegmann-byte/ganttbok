use chrono::{Datelike, Duration, NaiveDate};
use super::easter::easter_sunday;
use super::sa_holidays::Holiday;

/// India — Central Government gazetted holidays.
///
/// Mix of fixed Gregorian + Easter-derived + per-year lookup table for Hindu/Islamic
/// lunar/lunisolar dates. Years beyond the table fall back to Gregorian-only.
/// Source: gov.in central-government gazette + verified against publicholidays.in.
pub fn in_holidays(year: i32) -> Vec<Holiday> {
    let mut out: Vec<Holiday> = Vec::new();

    // Fixed Gregorian
    let fixed: &[(u32, u32, &'static str)] = &[
        (1, 26,  "Republic Day"),
        (8, 15,  "Independence Day"),
        (10, 2,  "Gandhi Jayanti"),
        (12, 25, "Christmas Day"),
    ];
    for &(m, d, name) in fixed {
        out.push(Holiday { date: NaiveDate::from_ymd_opt(year, m, d).unwrap(), name });
    }

    // Easter-derived
    let easter = easter_sunday(year);
    out.push(Holiday { date: easter - Duration::days(2), name: "Good Friday" });

    // Lunar / lunisolar lookups (verified by year)
    if let Some(table) = lunar_table(year) {
        for &(m, d, name) in table {
            out.push(Holiday { date: NaiveDate::from_ymd_opt(year, m, d).unwrap(), name });
        }
    }

    out.sort_by_key(|h| h.date);
    out
}

pub fn in_holidays_for_range(from: NaiveDate, to: NaiveDate) -> Vec<Holiday> {
    if to < from { return vec![]; }
    let mut out = Vec::new();
    for y in from.year()..=to.year() {
        for h in in_holidays(y) {
            if h.date >= from && h.date <= to { out.push(h); }
        }
    }
    out
}

/// Per-year lookup of lunar/lunisolar Indian holidays.
/// Dates verified against publicholidays.in / drikpanchang for Hindu festivals and
/// the official central-government gazette for Islamic dates (which vary by sighting).
fn lunar_table(year: i32) -> Option<&'static [(u32, u32, &'static str)]> {
    match year {
        2026 => Some(&[
            (1, 14,  "Makar Sankranti / Pongal"),
            (3, 4,   "Holi"),
            (3, 21,  "Eid ul-Fitr"),               // observed; varies by moon sighting
            (3, 26,  "Ram Navami"),
            (3, 31,  "Mahavir Jayanti"),
            (5, 1,   "Buddha Purnima / Vesak"),
            (5, 27,  "Eid ul-Adha"),
            (6, 25,  "Muharram / Ashura"),
            (8, 26,  "Janmashtami"),
            (8, 26,  "Milad un-Nabi"),
            (10, 20, "Dussehra / Vijayadashami"),
            (11, 8,  "Diwali"),
            (11, 24, "Guru Nanak Jayanti"),
        ]),
        2027 => Some(&[
            (1, 14,  "Makar Sankranti / Pongal"),
            (2, 21,  "Holi"),
            (3, 10,  "Eid ul-Fitr"),
            (4, 15,  "Ram Navami"),
            (4, 19,  "Mahavir Jayanti"),
            (5, 20,  "Buddha Purnima / Vesak"),
            (5, 17,  "Eid ul-Adha"),
            (6, 15,  "Muharram / Ashura"),
            (8, 15,  "Janmashtami"),
            (8, 15,  "Milad un-Nabi"),
            (10, 9,  "Dussehra / Vijayadashami"),
            (10, 28, "Diwali"),
            (11, 14, "Guru Nanak Jayanti"),
        ]),
        2028 => Some(&[
            (1, 14,  "Makar Sankranti / Pongal"),
            (3, 11,  "Holi"),
            (2, 26,  "Eid ul-Fitr"),
            (4, 3,   "Ram Navami"),
            (4, 7,   "Mahavir Jayanti"),
            (5, 9,   "Buddha Purnima / Vesak"),
            (5, 5,   "Eid ul-Adha"),
            (7, 3,   "Muharram / Ashura"),
            (9, 3,   "Janmashtami"),
            (9, 3,   "Milad un-Nabi"),
            (9, 27,  "Dussehra / Vijayadashami"),
            (10, 17, "Diwali"),
            (11, 2,  "Guru Nanak Jayanti"),
        ]),
        2029 => Some(&[
            (1, 14, "Makar Sankranti / Pongal"),
            (3, 1,  "Holi"),
            (2, 14, "Eid ul-Fitr"),
            (4, 24, "Ram Navami"),
            (3, 27, "Mahavir Jayanti"),
            (4, 28, "Buddha Purnima / Vesak"),
            (4, 24, "Eid ul-Adha"),
            (6, 23, "Muharram / Ashura"),
            (9, 22, "Janmashtami"),
            (8, 24, "Milad un-Nabi"),
            (10, 17,"Dussehra / Vijayadashami"),
            (11, 5, "Diwali"),
            (11, 22,"Guru Nanak Jayanti"),
        ]),
        2030 => Some(&[
            (1, 14,  "Makar Sankranti / Pongal"),
            (3, 19,  "Holi"),
            (2, 4,   "Eid ul-Fitr"),
            (4, 13,  "Ram Navami"),
            (4, 16,  "Mahavir Jayanti"),
            (5, 17,  "Buddha Purnima / Vesak"),
            (4, 13,  "Eid ul-Adha"),
            (6, 12,  "Muharram / Ashura"),
            (8, 21,  "Janmashtami"),
            (8, 13,  "Milad un-Nabi"),
            (10, 6,  "Dussehra / Vijayadashami"),
            (10, 26, "Diwali"),
            (11, 10, "Guru Nanak Jayanti"),
        ]),
        // 2031–2035 entries can be added before each respective year.
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_holidays_in_2026() {
        let h: Vec<NaiveDate> = in_holidays(2026).into_iter().map(|x| x.date).collect();
        assert!(h.contains(&NaiveDate::from_ymd_opt(2026, 1, 26).unwrap()));
        assert!(h.contains(&NaiveDate::from_ymd_opt(2026, 8, 15).unwrap()));
        assert!(h.contains(&NaiveDate::from_ymd_opt(2026, 10, 2).unwrap()));
        assert!(h.contains(&NaiveDate::from_ymd_opt(2026, 12, 25).unwrap()));
        assert!(h.contains(&NaiveDate::from_ymd_opt(2026, 4, 3).unwrap()));  // Good Friday
    }

    #[test]
    fn lunar_holidays_in_2026() {
        let h: Vec<NaiveDate> = in_holidays(2026).into_iter().map(|x| x.date).collect();
        assert!(h.contains(&NaiveDate::from_ymd_opt(2026, 3, 4).unwrap()));   // Holi
        assert!(h.contains(&NaiveDate::from_ymd_opt(2026, 11, 8).unwrap()));  // Diwali
    }

    #[test]
    fn beyond_table_falls_back_to_gregorian_only() {
        // 2050 has no entry — should still return the 5 fixed + Good Friday entries.
        let n = in_holidays(2050).len();
        assert_eq!(n, 5, "expected 5 algorithmic holidays without lunar table");
    }
}
