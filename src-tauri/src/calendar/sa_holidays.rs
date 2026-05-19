use chrono::{Duration, NaiveDate};
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
    vec![
        Holiday { date: NaiveDate::from_ymd_opt(year, 1, 1).unwrap(),   name: "New Year's Day" },
        Holiday { date: NaiveDate::from_ymd_opt(year, 3, 21).unwrap(),  name: "Human Rights Day" },
        Holiday { date: good_friday,                                    name: "Good Friday" },
        Holiday { date: family_day,                                     name: "Family Day" },
        Holiday { date: NaiveDate::from_ymd_opt(year, 4, 27).unwrap(),  name: "Freedom Day" },
        Holiday { date: NaiveDate::from_ymd_opt(year, 5, 1).unwrap(),   name: "Workers' Day" },
        Holiday { date: NaiveDate::from_ymd_opt(year, 6, 16).unwrap(),  name: "Youth Day" },
        Holiday { date: NaiveDate::from_ymd_opt(year, 8, 9).unwrap(),   name: "National Women's Day" },
        Holiday { date: NaiveDate::from_ymd_opt(year, 9, 24).unwrap(),  name: "Heritage Day" },
        Holiday { date: NaiveDate::from_ymd_opt(year, 12, 16).unwrap(), name: "Day of Reconciliation" },
        Holiday { date: NaiveDate::from_ymd_opt(year, 12, 25).unwrap(), name: "Christmas Day" },
        Holiday { date: NaiveDate::from_ymd_opt(year, 12, 26).unwrap(), name: "Day of Goodwill" },
    ]
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
        assert!(h.contains(&(NaiveDate::from_ymd_opt(2026,8,9).unwrap(),  "National Women's Day")));
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
}
