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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Phase {
    pub id: i64,
    pub job_id: i64,
    pub name: String,
    pub colour: String,        // hex e.g. "#3B82F6"
    pub order_index: i64,
    pub collapsed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewPhase {
    pub job_id: i64,
    pub name: String,
    pub colour: String,
    pub order_index: i64,
    pub collapsed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Task {
    pub id: i64,
    pub phase_id: i64,
    pub name: String,
    pub start_date: NaiveDate,
    pub duration_workdays: i64,
    pub order_index: i64,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewTask {
    pub phase_id: i64,
    pub name: String,
    pub start_date: NaiveDate,
    pub duration_workdays: i64,
    pub order_index: i64,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Dependency {
    pub id: i64,
    pub predecessor_id: i64,
    pub successor_id: i64,
    pub r#type: String,    // 'FS' for v1
    pub lag_days: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewDependency {
    pub predecessor_id: i64,
    pub successor_id: i64,
    pub lag_days: i64,
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

    #[test]
    fn phase_serializes_to_json() {
        let p = Phase {
            id: 1, job_id: 1, name: "Plumbing".into(),
            colour: "#3B82F6".into(), order_index: 0, collapsed: true,
        };
        let s = serde_json::to_string(&p).unwrap();
        let back: Phase = serde_json::from_str(&s).unwrap();
        assert_eq!(p, back);
    }

    #[test]
    fn task_serializes_to_json() {
        let t = Task {
            id: 1, phase_id: 1, name: "First-fix".into(),
            start_date: NaiveDate::from_ymd_opt(2026, 6, 8).unwrap(),
            duration_workdays: 3, order_index: 0, notes: None,
        };
        let s = serde_json::to_string(&t).unwrap();
        let back: Task = serde_json::from_str(&s).unwrap();
        assert_eq!(t, back);
    }

    #[test]
    fn dependency_serializes_to_json() {
        let d = Dependency { id: 1, predecessor_id: 1, successor_id: 2, r#type: "FS".into(), lag_days: 0 };
        let s = serde_json::to_string(&d).unwrap();
        let back: Dependency = serde_json::from_str(&s).unwrap();
        assert_eq!(d, back);
    }
}
