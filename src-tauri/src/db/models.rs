use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Job {
    pub id: i64,
    pub name: String,
    pub client: Option<String>,
    pub address: Option<String>,
    pub project_start_date: NaiveDate,
    pub is_template: bool,
    pub archived: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewJob {
    pub name: String,
    pub client: Option<String>,
    pub address: Option<String>,
    pub project_start_date: NaiveDate,
    pub is_template: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn job_serializes_to_json() {
        let job = Job {
            id: 1,
            name: "Sea Point reno".into(),
            client: Some("M. Botha".into()),
            address: None,
            project_start_date: NaiveDate::from_ymd_opt(2026, 6, 5).unwrap(),
            is_template: false,
            archived: false,
            created_at: "2026-05-19T20:00:00".into(),
        };
        let s = serde_json::to_string(&job).unwrap();
        let back: Job = serde_json::from_str(&s).unwrap();
        assert_eq!(job, back);
    }
}
