use thiserror::Error;

#[derive(Debug, Error)]
pub enum GbError {
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("migration error: {0}")]
    Migration(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("validation: {0}")]
    Validation(String),
    #[error("cycle detected when creating dependency {0} -> {1}")]
    DependencyCycle(i64, i64),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("serde: {0}")]
    Serde(#[from] serde_json::Error),
}

pub type GbResult<T> = Result<T, GbError>;

impl serde::Serialize for GbError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}
