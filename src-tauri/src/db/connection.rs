use rusqlite::Connection;
use std::path::Path;
use crate::GbResult;
use super::migrations::apply_migrations;

pub fn open(path: &Path) -> GbResult<Connection> {
    let conn = Connection::open(path)?;
    apply_migrations(&conn)?;
    Ok(conn)
}

pub fn open_in_memory() -> GbResult<Connection> {
    let conn = Connection::open_in_memory()?;
    apply_migrations(&conn)?;
    Ok(conn)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_in_memory_applies_migrations() {
        let conn = open_in_memory().unwrap();
        let v: i64 = conn
            .query_row("SELECT CAST(value AS INTEGER) FROM app_meta WHERE key='schema_version'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(v, 1);
    }
}
