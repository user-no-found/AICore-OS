use aicore_foundation::AicoreError;
use rusqlite::{Error as SqliteError, ErrorCode};

pub fn sqlite_open_error(error: SqliteError) -> AicoreError {
    AicoreError::Unavailable(format!("sqlite open failed: {error}"))
}

pub fn sqlite_schema_error(error: SqliteError) -> AicoreError {
    AicoreError::InvalidState(format!("sqlite schema error: {error}"))
}

pub fn sqlite_write_error(error: SqliteError) -> AicoreError {
    if let SqliteError::SqliteFailure(inner, _) = &error {
        if inner.code == ErrorCode::ConstraintViolation {
            return AicoreError::Duplicate(format!("sqlite constraint violation: {error}"));
        }
    }
    AicoreError::Unavailable(format!("sqlite write failed: {error}"))
}

pub fn sqlite_read_error(error: SqliteError) -> AicoreError {
    AicoreError::Unavailable(format!("sqlite read failed: {error}"))
}

pub fn unsupported_api(name: &str) -> AicoreError {
    AicoreError::Unavailable(format!("{name} not implemented yet"))
}
