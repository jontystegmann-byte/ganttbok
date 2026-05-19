use chrono::{Datelike, Duration, NaiveDate, Weekday};
use super::easter::easter_sunday;

#[derive(Debug, Clone, PartialEq)]
pub struct Holiday {
    pub date: NaiveDate,
    pub name: &'static str,
}

pub fn sa_holidays(year: i32) -> Vec<Holiday> {
    let easter = easter_sunday(year);
    let good_friday = easter - Duration::days(2);
    let family_day = easter + Duration::days(1);

    let fixed: &[(u32, u32, &'static str)] = &[
        (1, 1,   "New Year's Day"),
        (3, 21,  "Human Rights Day"),
        (4, 27,  "Freedom Day"),
        (5, 1,   "Workers' Day"),
        (6, 16,  "Youth Day"),
        (8, 9,   "National Women's Day"),
        (9, 24,  "Heritage Day"),
        (12, 16, "Day of Reconciliation"),
        (12, 25, "Christmas Day"),
        (12, 26, "Day of Goodwill"),
    ];

    let mut out: Vec<Holiday> = Vec::with_capacity(12);
    for &(m, d, name) in fixed {
        let raw = NaiveDate::from_ymd_opt(year, m, d).unwrap();
        let observed = if raw.weekday() == Weekday::Sun { raw + Duration::days(1) } else { raw };
        out.push(Holiday { date: observed, name });
    }
    out.push(Holiday { date: good_friday, name: "Good Friday" });
    out.push(Holiday { date: family_day,  name: "Family Day" });
    out.sort_by_key(|h| h.date);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dates(year: i32) -> Vec<(NaiveDate, &'static str)> {
        sa_holidays(year).into_iter().map(|h| (h.date, h.name)).collect()
    }

    #[test]
    fn twelve_holidays_per_year() {
        assert_eq!(sa_holidays(2026).len(), 12);
    }

    #[test]
    fn fixed_holidays_in_2026() {
        let h = dates(2026);
        assert!(h.contains(&(NaiveDate::from_ymd_opt(2026,1,1).unwrap(),  "New Year's Day")));
        assert!(h.contains(&(NaiveDate::from_ymd_opt(2026,3,21).unwrap(), "Human Rights Day")));
        assert!(h.contains(&(NaiveDate::from_ymd_opt(2026,4,27).unwrap(), "Freedom Day")));
        assert!(h.contains(&(NaiveDate::from_ymd_opt(2026,5,1).unwrap(),  "Workers' Day")));
        assert!(h.contains(&(NaiveDate::from_ymd_opt(2026,6,16).unwrap(), "Youth Day")));
        // 9 Aug 2026 falls on Sunday → observed shifts to Mon 10 Aug (Public Holidays Act 1994)
        assert!(h.contains(&(NaiveDate::from_ymd_opt(2026,8,10).unwrap(), "National Women's Day")));
        assert!(h.contains(&(NaiveDate::from_ymd_opt(2026,9,24).unwrap(), "Heritage Day")));
        assert!(h.contains(&(NaiveDate::from_ymd_opt(2026,12,16).unwrap(),"Day of Reconciliation")));
        assert!(h.contains(&(NaiveDate::from_ymd_opt(2026,12,25).unwrap(),"Christmas Day")));
        assert!(h.contains(&(NaiveDate::from_ymd_opt(2026,12,26).unwrap(),"Day of Goodwill")));
    }

    #[test]
    fn easter_derived_in_2026() {
        let h = dates(2026);
        // Easter Sun 2026 = 5 April; Good Friday = 3 April; Family Day = 6 April
        assert!(h.contains(&(NaiveDate::from_ymd_opt(2026,4,3).unwrap(), "Good Friday")));
        assert!(h.contains(&(NaiveDate::from_ymd_opt(2026,4,6).unwrap(), "Family Day")));
    }

    #[test]
    fn workers_day_on_sunday_shifts_to_monday_in_2022() {
        // 1 May 2022 was a Sunday; observed holiday is Mon 2 May 2022
        let h = sa_holidays(2022);
        let dates: Vec<NaiveDate> = h.iter().map(|x| x.date).collect();
        assert!(dates.contains(&NaiveDate::from_ymd_opt(2022, 5, 2).unwrap()),
                "expected observed Workers' Day on Mon 2 May 2022");
        assert!(!dates.contains(&NaiveDate::from_ymd_opt(2022, 5, 1).unwrap()),
                "raw 1 May Sunday should not appear");
    }
}
