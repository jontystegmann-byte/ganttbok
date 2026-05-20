use chrono::NaiveDate;
use serde::Deserialize;
use tauri::State;
use crate::commands::Db;
use crate::db::models::{Job, NewJob, NewPhase, NewTask};
use crate::repo::{job as job_repo, phase as phase_repo, task as task_repo};
use crate::GbResult;

#[derive(Debug, Deserialize)]
pub struct InstantiateArgs {
    pub template_id: i64,
    pub new_name: String,
    pub client: Option<String>,
    pub address: Option<String>,
    pub project_start_date: NaiveDate,
}

#[tauri::command]
pub fn save_as_template(db: State<Db>, source_job_id: i64, template_name: String) -> GbResult<Job> {
    let conn = db.0.lock().unwrap();
    save_as_template_inner(&conn, source_job_id, &template_name)
}

#[tauri::command]
pub fn instantiate_template(db: State<Db>, args: InstantiateArgs) -> GbResult<Job> {
    let conn = db.0.lock().unwrap();
    instantiate_template_inner(&conn, args)
}

fn save_as_template_inner(conn: &rusqlite::Connection, source_id: i64, name: &str) -> GbResult<Job> {
    let source = job_repo::get(conn, source_id)?;
    let new_template = job_repo::create(conn, &NewJob {
        name: name.to_string(),
        client: None,
        address: None,
        project_start_date: source.project_start_date,
        is_template: true,
    })?;
    let phases = phase_repo::list_for_job(conn, source.id)?;
    for p in phases {
        let new_p = phase_repo::create(conn, &NewPhase {
            job_id: new_template.id,
            name: p.name,
            colour: p.colour,
            order_index: p.order_index,
            collapsed: p.collapsed,
        })?;
        let tasks = task_repo::list_for_phase(conn, p.id)?;
        for t in tasks {
            task_repo::create(conn, &NewTask {
                phase_id: new_p.id,
                name: t.name,
                start_date: source.project_start_date,
                duration_workdays: 1,
                order_index: t.order_index,
                notes: None,
            })?;
        }
    }
    Ok(new_template)
}

fn instantiate_template_inner(conn: &rusqlite::Connection, args: InstantiateArgs) -> GbResult<Job> {
    let template = job_repo::get(conn, args.template_id)?;
    if !template.is_template {
        return Err(crate::GbError::Validation(format!("job {} is not a template", args.template_id)));
    }
    let new_job = job_repo::create(conn, &NewJob {
        name: args.new_name,
        client: args.client,
        address: args.address,
        project_start_date: args.project_start_date,
        is_template: false,
    })?;
    let phases = phase_repo::list_for_job(conn, template.id)?;
    for p in phases {
        let new_p = phase_repo::create(conn, &NewPhase {
            job_id: new_job.id,
            name: p.name,
            colour: p.colour,
            order_index: p.order_index,
            collapsed: true,
        })?;
        let tasks = task_repo::list_for_phase(conn, p.id)?;
        for t in tasks {
            task_repo::create(conn, &NewTask {
                phase_id: new_p.id,
                name: t.name,
                start_date: args.project_start_date,
                duration_workdays: 1,
                order_index: t.order_index,
                notes: None,
            })?;
        }
    }
    Ok(new_job)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection::open_in_memory;

    #[test]
    fn save_and_instantiate_template_stacks_tasks_at_start() {
        let conn = open_in_memory().unwrap();
        let source = job_repo::create(&conn, &NewJob {
            name: "Std reno".into(), client: None, address: None,
            project_start_date: NaiveDate::from_ymd_opt(2026,6,5).unwrap(),
            is_template: false,
        }).unwrap();
        let p = phase_repo::create(&conn, &NewPhase {
            job_id: source.id, name: "Plumbing".into(), colour: "#3B82F6".into(),
            order_index: 0, collapsed: true,
        }).unwrap();
        task_repo::create(&conn, &NewTask {
            phase_id: p.id, name: "First-fix".into(),
            start_date: NaiveDate::from_ymd_opt(2026,6,8).unwrap(),
            duration_workdays: 3, order_index: 0, notes: None,
        }).unwrap();
        task_repo::create(&conn, &NewTask {
            phase_id: p.id, name: "Second-fix".into(),
            start_date: NaiveDate::from_ymd_opt(2026,6,15).unwrap(),
            duration_workdays: 2, order_index: 1, notes: None,
        }).unwrap();

        let tmpl = save_as_template_inner(&conn, source.id, "Std reno tmpl").unwrap();
        assert!(tmpl.is_template);

        let instantiated = instantiate_template_inner(&conn, InstantiateArgs {
            template_id: tmpl.id,
            new_name: "Camps Bay".into(),
            client: Some("J. Botha".into()),
            address: None,
            project_start_date: NaiveDate::from_ymd_opt(2026, 7, 6).unwrap(),
        }).unwrap();

        let tasks = task_repo::list_for_job(&conn, instantiated.id).unwrap();
        assert_eq!(tasks.len(), 2);
        assert!(tasks.iter().all(|t| t.duration_workdays == 1));
        assert!(tasks.iter().all(|t| t.start_date == NaiveDate::from_ymd_opt(2026,7,6).unwrap()));
    }
}
