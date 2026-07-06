use std::sync::Mutex;
use rusqlite::Connection;

pub mod job;
pub mod template;
pub mod phase;
pub mod task;
pub mod drag;
pub mod dependency;
pub mod no_work_day;
pub mod meta;
pub mod sync;
pub mod chaser;
pub mod patches;
pub mod claude;
pub mod boq;
pub mod boq_export;

/// Wraps the singleton SQLite connection in a Mutex so Tauri can pass it
/// to command handlers as `tauri::State<Db>`.
pub struct Db(pub Mutex<Connection>);

impl Db {
    pub fn new(conn: Connection) -> Self { Self(Mutex::new(conn)) }
}
