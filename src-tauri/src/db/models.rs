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
    pub holidays_block_work: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewJob {
    pub name: String,
    pub client: Option<String>,
    pub address: Option<String>,
    pub project_start_date: NaiveDate,
    pub is_template: bool,
    pub holidays_block_work: bool,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NoWorkDay {
    pub id: i64,
    pub job_id: i64,
    pub date: NaiveDate,
    pub reason: String,
    pub source: String,   // 'sa_public_holiday' | 'manual'
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewNoWorkDay {
    pub job_id: i64,
    pub date: NaiveDate,
    pub reason: String,
    pub source: String,
}

use rusqlite::Connection;

pub fn meta_get(conn: &Connection, key: &str) -> crate::GbResult<Option<String>> {
    let res = conn
        .query_row("SELECT value FROM app_meta WHERE key = ?1", [key], |r| r.get::<_, String>(0))
        .ok();
    Ok(res)
}

pub fn meta_set(conn: &Connection, key: &str, value: &str) -> crate::GbResult<()> {
    conn.execute(
        "INSERT INTO app_meta (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        rusqlite::params![key, value],
    )?;
    Ok(())
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
            holidays_block_work: true,
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

    #[test]
    fn no_work_day_serializes_to_json() {
        let n = NoWorkDay {
            id: 1, job_id: 1,
            date: NaiveDate::from_ymd_opt(2026, 6, 16).unwrap(),
            reason: "Youth Day".into(),
            source: "sa_public_holiday".into(),
        };
        let s = serde_json::to_string(&n).unwrap();
        let back: NoWorkDay = serde_json::from_str(&s).unwrap();
        assert_eq!(n, back);
    }

    #[test]
    fn meta_get_set_roundtrip() {
        let conn = crate::db::connection::open_in_memory().unwrap();
        assert!(meta_get(&conn, "last_open_job_id").unwrap().is_none());
        meta_set(&conn, "last_open_job_id", "42").unwrap();
        assert_eq!(meta_get(&conn, "last_open_job_id").unwrap(), Some("42".into()));
        meta_set(&conn, "last_open_job_id", "43").unwrap();
        assert_eq!(meta_get(&conn, "last_open_job_id").unwrap(), Some("43".into()));
    }
}
