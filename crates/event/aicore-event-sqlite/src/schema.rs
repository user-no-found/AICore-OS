use aicore_foundation::{AicoreError, AicoreResult, InstanceId, Timestamp};
use rusqlite::{Connection, params};

use crate::error::{sqlite_open_error, sqlite_schema_error};

pub const STORE_SCHEMA_VERSION: i64 = 1;
pub const STORE_KIND: &str = "sqlite_event_ledger";

pub const FORBIDDEN_SCHEMA_FIELDS: [&str; 17] = [
    "raw_stdout",
    "raw_stderr",
    "raw_payload",
    "raw_memory_content",
    "raw_provider_request",
    "raw_provider_response",
    "raw_tool_input",
    "raw_tool_output",
    "secret",
    "secret_ref",
    "token",
    "api_key",
    "cookie",
    "full_prompt",
    "full_log",
    "full_backtrace",
    "full_patch",
];

const SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS event_store_meta (
    schema_version INTEGER NOT NULL,
    store_kind TEXT NOT NULL,
    instance_id TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS events (
    event_id TEXT PRIMARY KEY,
    event_type TEXT NOT NULL,
    schema_version TEXT NOT NULL,
    occurred_at TEXT NOT NULL,
    recorded_at TEXT NOT NULL,
    source_component TEXT NOT NULL,
    source_instance TEXT NOT NULL,
    subject_type TEXT NOT NULL,
    subject_id TEXT NOT NULL,
    summary TEXT NOT NULL,
    retention_class TEXT NOT NULL,
    correlation_id TEXT,
    causation_id TEXT,
    invocation_id TEXT,
    redaction_level TEXT,
    visibility TEXT,
    status TEXT,
    replay_policy TEXT,
    evidence_ref TEXT,
    payload_ref TEXT
);

CREATE INDEX IF NOT EXISTS idx_events_source_instance ON events(source_instance);
CREATE INDEX IF NOT EXISTS idx_events_event_type ON events(event_type);
CREATE INDEX IF NOT EXISTS idx_events_subject ON events(subject_type, subject_id);
CREATE INDEX IF NOT EXISTS idx_events_occurred_at ON events(occurred_at);
CREATE INDEX IF NOT EXISTS idx_events_retention_class ON events(retention_class);
CREATE INDEX IF NOT EXISTS idx_events_status ON events(status);

CREATE TABLE IF NOT EXISTS event_tags (
    event_id TEXT NOT NULL,
    tag TEXT NOT NULL,
    PRIMARY KEY (event_id, tag),
    FOREIGN KEY (event_id) REFERENCES events(event_id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_event_tags_tag ON event_tags(tag);
CREATE INDEX IF NOT EXISTS idx_event_tags_event_id ON event_tags(event_id);

CREATE TABLE IF NOT EXISTS event_confirmed_tags (
    event_id TEXT NOT NULL,
    tag TEXT NOT NULL,
    PRIMARY KEY (event_id, tag),
    FOREIGN KEY (event_id) REFERENCES events(event_id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_event_confirmed_tags_tag ON event_confirmed_tags(tag);
CREATE INDEX IF NOT EXISTS idx_event_confirmed_tags_event_id ON event_confirmed_tags(event_id);

CREATE TABLE IF NOT EXISTS event_refs (
    event_id TEXT NOT NULL,
    ref_kind TEXT NOT NULL CHECK(ref_kind IN ('evidence_ref', 'payload_ref')),
    ref_value TEXT NOT NULL,
    PRIMARY KEY (event_id, ref_kind),
    FOREIGN KEY (event_id) REFERENCES events(event_id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_event_refs_event_id ON event_refs(event_id);
CREATE INDEX IF NOT EXISTS idx_event_refs_ref_kind ON event_refs(ref_kind);

CREATE TABLE IF NOT EXISTS error_index (
    error_event_id TEXT PRIMARY KEY,
    source_instance TEXT NOT NULL,
    source_component TEXT NOT NULL,
    subject_type TEXT NOT NULL,
    subject_id TEXT NOT NULL,
    summary TEXT NOT NULL,
    status TEXT NOT NULL,
    first_seen_at TEXT NOT NULL,
    last_seen_at TEXT NOT NULL,
    retention_class TEXT NOT NULL,
    evidence_ref TEXT
);

CREATE TABLE IF NOT EXISTS fix_index (
    fix_event_id TEXT PRIMARY KEY,
    error_event_id TEXT,
    source_instance TEXT NOT NULL,
    source_component TEXT NOT NULL,
    summary TEXT NOT NULL,
    fix_status TEXT NOT NULL,
    verification_status TEXT NOT NULL,
    recorded_at TEXT NOT NULL,
    retention_class TEXT NOT NULL,
    evidence_ref TEXT
);

CREATE TABLE IF NOT EXISTS compaction_runs (
    run_id TEXT PRIMARY KEY,
    started_at TEXT NOT NULL,
    finished_at TEXT,
    status TEXT NOT NULL,
    records_scanned INTEGER NOT NULL,
    records_compressed INTEGER NOT NULL,
    records_deleted INTEGER NOT NULL,
    error_summary TEXT
);
"#;

pub fn open_connection(path: &std::path::Path) -> AicoreResult<Connection> {
    let conn = Connection::open(path).map_err(sqlite_open_error)?;
    conn.execute("PRAGMA foreign_keys = ON", [])
        .map_err(sqlite_schema_error)?;
    Ok(conn)
}

pub fn initialize_or_validate(
    conn: &Connection,
    instance_id: &InstanceId,
    now: Timestamp,
) -> AicoreResult<()> {
    if schema_contains_forbidden_fields(SCHEMA_SQL) {
        return Err(AicoreError::InvalidState(
            "sqlite schema contains forbidden raw fields".to_string(),
        ));
    }

    conn.execute_batch(SCHEMA_SQL)
        .map_err(sqlite_schema_error)?;

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM event_store_meta", [], |row| {
            row.get(0)
        })
        .map_err(sqlite_schema_error)?;

    if count == 0 {
        let now_text = now.unix_millis().to_string();
        conn.execute(
            "INSERT INTO event_store_meta (schema_version, store_kind, instance_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                STORE_SCHEMA_VERSION,
                STORE_KIND,
                instance_id.as_str(),
                now_text,
                now_text
            ],
        )
        .map_err(sqlite_schema_error)?;
        return Ok(());
    }

    validate_meta(conn, instance_id)
}

pub fn validate_meta(conn: &Connection, instance_id: &InstanceId) -> AicoreResult<()> {
    let (schema_version, store_kind, stored_instance_id): (i64, String, String) = conn
        .query_row(
            "SELECT schema_version, store_kind, instance_id FROM event_store_meta LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(sqlite_schema_error)?;

    if schema_version != STORE_SCHEMA_VERSION {
        return Err(AicoreError::VersionMismatch(format!(
            "unsupported event store schema version: {schema_version}"
        )));
    }

    if store_kind != STORE_KIND {
        return Err(AicoreError::InvalidState(format!(
            "unexpected event store kind: {store_kind}"
        )));
    }

    if stored_instance_id != instance_id.as_str() {
        return Err(AicoreError::Conflict(format!(
            "event store instance id mismatch: expected {}, got {}",
            instance_id.as_str(),
            stored_instance_id
        )));
    }

    Ok(())
}

#[cfg(test)]
pub fn schema_sql() -> &'static str {
    SCHEMA_SQL
}

pub fn schema_contains_forbidden_fields(schema: &str) -> bool {
    FORBIDDEN_SCHEMA_FIELDS
        .iter()
        .any(|forbidden| schema.contains(forbidden))
}
