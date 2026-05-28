use std::collections::HashSet;
use chrono::NaiveDate;
use ganttbok_lib::{
    calendar::workday::count_workdays,
    db::{connection::open_in_memory, models::*},
    deps::ripple::compute_ripple,
    repo::{job, phase, task, dependency, no_work_day},
};

#[test]
fn full_job_lifecycle_with_template_drag_and_sa_sync() {
    let conn = open_in_memory().unwrap();

    // 1. Create a template with 2 phases / 3 tasks.
    let tmpl = job::create(&conn, &NewJob {
        name: "Std reno".into(), client: None, address: None,
        project_start_date: NaiveDate::from_ymd_opt(2026,1,1).unwrap(),
        is_template: true, holidays_block_work: true,
            region: "ZA".into(),
        auto_shift_dependents: true,
    }).unwrap();
    let p1 = phase::create(&conn, &NewPhase {
        job_id: tmpl.id, name: "Plumbing".into(), colour: "#3B82F6".into(),
        order_index: 0, collapsed: true,
    }).unwrap();
    let p2 = phase::create(&conn, &NewPhase {
        job_id: tmpl.id, name: "Electrical".into(), colour: "#EF4444".into(),
        order_index: 1, collapsed: true,
    }).unwrap();
    task::create(&conn, &NewTask {
        phase_id: p1.id, name: "First-fix".into(),
        start_date: NaiveDate::from_ymd_opt(2026,1,1).unwrap(),
        duration_workdays: 1, order_index: 0, notes: None,
    }).unwrap();
    task::create(&conn, &NewTask {
        phase_id: p1.id, name: "Second-fix".into(),
        start_date: NaiveDate::from_ymd_opt(2026,1,1).unwrap(),
        duration_workdays: 1, order_index: 1, notes: None,
    }).unwrap();
    task::create(&conn, &NewTask {
        phase_id: p2.id, name: "Wiring".into(),
        start_date: NaiveDate::from_ymd_opt(2026,1,1).unwrap(),
        duration_workdays: 1, order_index: 0, notes: None,
    }).unwrap();

    // 2. Instantiate into a real job starting Mon 8 Jun 2026.
    let project_start = NaiveDate::from_ymd_opt(2026, 6, 8).unwrap();
    let new_job = job::create(&conn, &NewJob {
        name: "Sea Point".into(), client: Some("M. Botha".into()), address: None,
        project_start_date: project_start, is_template: false, holidays_block_work: true,
            region: "ZA".into(),
        auto_shift_dependents: true,
    }).unwrap();
    for p in phase::list_for_job(&conn, tmpl.id).unwrap() {
        let np = phase::create(&conn, &NewPhase {
            job_id: new_job.id, name: p.name, colour: p.colour,
            order_index: p.order_index, collapsed: true,
        }).unwrap();
        for t in task::list_for_phase(&conn, p.id).unwrap() {
            task::create(&conn, &NewTask {
                phase_id: np.id, name: t.name,
                start_date: project_start, duration_workdays: 1,
                order_index: t.order_index, notes: None,
            }).unwrap();
        }
    }

    // 3. Sync SA holidays for the new job (Jan-Dec 2026).
    let inserted = no_work_day::sync_sa_holidays(
        &conn, new_job.id,
        NaiveDate::from_ymd_opt(2026,1,1).unwrap(),
        NaiveDate::from_ymd_opt(2026,12,31).unwrap(),
    ).unwrap();
    assert_eq!(inserted, 12);

    // 4. Link First-fix -> Second-fix -> Wiring (cross-phase chain).
    let tasks = task::list_for_job(&conn, new_job.id).unwrap();
    let first = tasks.iter().find(|t| t.name == "First-fix").unwrap();
    let second = tasks.iter().find(|t| t.name == "Second-fix").unwrap();
    let wiring = tasks.iter().find(|t| t.name == "Wiring").unwrap();
    dependency::create(&conn, &NewDependency {
        predecessor_id: first.id, successor_id: second.id, lag_days: 0,
    }).unwrap();
    dependency::create(&conn, &NewDependency {
        predecessor_id: second.id, successor_id: wiring.id, lag_days: 0,
    }).unwrap();

    // 5. Drag First-fix +2 workdays. Expect Second-fix and Wiring to shift +2.
    let tasks = task::list_for_job(&conn, new_job.id).unwrap();
    let deps  = dependency::list_for_job(&conn, new_job.id).unwrap();
    let nwds: HashSet<NaiveDate> = no_work_day::list_for_job(&conn, new_job.id).unwrap()
        .into_iter().map(|n| n.date).collect();

    let ripple = compute_ripple(&tasks, &deps, first.id, 2, &nwds, false);
    assert_eq!(ripple.len(), 2, "two downstream tasks expected");
    assert_eq!(*ripple.get(&second.id).unwrap(), NaiveDate::from_ymd_opt(2026,6,10).unwrap());
    assert_eq!(*ripple.get(&wiring.id).unwrap(), NaiveDate::from_ymd_opt(2026,6,10).unwrap());

    // 6. Sanity.
    assert_eq!(count_workdays(NaiveDate::from_ymd_opt(2026,6,8).unwrap(), NaiveDate::from_ymd_opt(2026,6,12).unwrap(), false), 5);
}
