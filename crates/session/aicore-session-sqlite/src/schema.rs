use aicore_foundation::{AicoreError, AicoreResult, InstanceId, Timestamp};
use rusqlite::{Connection, params};

use crate::error::{sqlite_open_error, sqlite_schema_error, sqlite_write_error};

pub const STORE_SCHEMA_VERSION: i64 = 2;
pub const STORE_KIND: &str = "sqlite_session_ledger";

const SCHEMA_SQL: &str = r#"
-- ledger_meta
CREATE TABLE IF NOT EXISTS ledger_meta (
    schema_version INTEGER NOT NULL,
    store_kind TEXT NOT NULL,
    instance_id TEXT PRIMARY KEY NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

-- sessions
CREATE TABLE IF NOT EXISTS sessions (
    session_id TEXT PRIMARY KEY NOT NULL,
    title TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('active', 'archived')),
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    metadata TEXT
);

-- turns
CREATE TABLE IF NOT EXISTS turns (
    turn_id TEXT PRIMARY KEY NOT NULL,
    session_id TEXT NOT NULL,
    turn_seq INTEGER NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('active', 'running', 'waiting_approval', 'stopping', 'stopped', 'completed', 'interrupted', 'cancelled', 'failed', 'interrupted_by_recovery')),
    started_at INTEGER NOT NULL,
    finished_at INTEGER,
    FOREIGN KEY (session_id) REFERENCES sessions(session_id) ON DELETE CASCADE,
    UNIQUE(session_id, turn_seq)
);

CREATE INDEX IF NOT EXISTS idx_turns_session_id ON turns(session_id);
CREATE INDEX IF NOT EXISTS idx_turns_turn_seq ON turns(turn_seq);

-- messages
CREATE TABLE IF NOT EXISTS messages (
    message_id TEXT PRIMARY KEY NOT NULL,
    session_id TEXT NOT NULL,
    turn_id TEXT,
    message_seq INTEGER,
    kind TEXT NOT NULL CHECK(kind IN ('user', 'assistant_delta', 'assistant_final', 'system', 'tool_call', 'tool_result')),
    content TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    metadata TEXT,
    FOREIGN KEY (session_id) REFERENCES sessions(session_id) ON DELETE CASCADE,
    FOREIGN KEY (turn_id) REFERENCES turns(turn_id) ON DELETE CASCADE,
    UNIQUE(turn_id, message_seq)
);

CREATE INDEX IF NOT EXISTS idx_messages_session_id ON messages(session_id);
CREATE INDEX IF NOT EXISTS idx_messages_turn_id ON messages(turn_id);

-- pending_inputs
CREATE TABLE IF NOT EXISTS pending_inputs (
    pending_input_id TEXT PRIMARY KEY NOT NULL,
    instance_id TEXT NOT NULL,
    session_id TEXT,
    turn_id TEXT,
    content TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('pending', 'confirmed', 'cancelled', 'replaced', 'stale')),
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    FOREIGN KEY (session_id) REFERENCES sessions(session_id) ON DELETE CASCADE,
    FOREIGN KEY (turn_id) REFERENCES turns(turn_id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_pending_inputs_one_open
    ON pending_inputs(instance_id)
    WHERE status = 'pending';

-- approvals
CREATE TABLE IF NOT EXISTS approvals (
    approval_id TEXT PRIMARY KEY NOT NULL,
    instance_id TEXT NOT NULL,
    turn_id TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('pending', 'approved', 'rejected', 'cancelled', 'expired', 'stale', 'invalidated_by_stop', 'invalidated_by_turn_close', 'invalidated_by_recovery')),
    scope TEXT NOT NULL DEFAULT 'single_tool_call' CHECK(scope IN ('single_tool_call')),
    summary TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    resolved_at INTEGER,
    resolved_response_id TEXT,
    FOREIGN KEY (turn_id) REFERENCES turns(turn_id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_approvals_turn_id ON approvals(turn_id);

-- approval_responses
CREATE TABLE IF NOT EXISTS approval_responses (
    response_id TEXT PRIMARY KEY NOT NULL,
    approval_id TEXT NOT NULL,
    instance_id TEXT NOT NULL,
    decision TEXT NOT NULL CHECK(decision IN ('approve', 'reject')),
    status TEXT NOT NULL CHECK(status IN ('accepted', 'rejected_stale', 'rejected_already_resolved', 'rejected_turn_not_active')),
    responder_client_id TEXT,
    responder_client_kind TEXT,
    responded_at INTEGER NOT NULL,
    FOREIGN KEY (approval_id) REFERENCES approvals(approval_id) ON DELETE CASCADE
);

-- instance_runtime_state (single row per instance)
CREATE TABLE IF NOT EXISTS instance_runtime_state (
    instance_id TEXT PRIMARY KEY NOT NULL,
    active_session_id TEXT,
    active_turn_id TEXT,
    pending_input_id TEXT,
    pending_approval_id TEXT,
    last_message_seq INTEGER,
    last_control_event_seq INTEGER,
    last_write_seq INTEGER,
    runtime_status TEXT NOT NULL CHECK(runtime_status IN ('idle', 'running', 'waiting_approval', 'stopping')),
    lock_version INTEGER NOT NULL DEFAULT 0,
    dirty_shutdown INTEGER NOT NULL DEFAULT 0,
    recovery_required INTEGER NOT NULL DEFAULT 0,
    updated_at INTEGER NOT NULL
);

-- control_events
CREATE TABLE IF NOT EXISTS control_events (
    event_id TEXT PRIMARY KEY NOT NULL,
    instance_id TEXT NOT NULL,
    turn_id TEXT,
    event_seq INTEGER NOT NULL,
    event_type TEXT NOT NULL CHECK(event_type IN ('session_created', 'turn_began', 'turn_finished', 'message_appended', 'turn_interrupted', 'runtime_state_updated', 'active_turn_acquired', 'active_turn_released', 'stop_requested', 'pending_input_submitted', 'pending_input_cancelled', 'approval_created', 'approval_resolved', 'approval_invalidated', 'custom')),
    detail TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_control_events_instance_id ON control_events(instance_id);
CREATE INDEX IF NOT EXISTS idx_control_events_turn_id ON control_events(turn_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_control_events_turn_seq
    ON control_events(turn_id, event_seq)
    WHERE turn_id IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_control_events_instance_seq
    ON control_events(instance_id, event_seq)
    WHERE turn_id IS NULL;

-- ledger_writes
CREATE TABLE IF NOT EXISTS ledger_writes (
    write_id TEXT PRIMARY KEY NOT NULL,
    instance_id TEXT NOT NULL,
    turn_id TEXT,
    write_seq INTEGER NOT NULL,
    write_type TEXT NOT NULL CHECK(write_type IN ('insert', 'update', 'delete')),
    target_table TEXT NOT NULL,
    target_id TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_ledger_writes_instance_id ON ledger_writes(instance_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_ledger_writes_turn_seq
    ON ledger_writes(turn_id, write_seq)
    WHERE turn_id IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_ledger_writes_instance_seq
    ON ledger_writes(instance_id, write_seq)
    WHERE turn_id IS NULL;
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
    conn.execute_batch(SCHEMA_SQL)
        .map_err(sqlite_schema_error)?;

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM ledger_meta", [], |row| row.get(0))
        .map_err(sqlite_schema_error)?;

    let now_millis = now.unix_millis() as i64;

    if count == 0 {
        conn.execute(
            "INSERT INTO ledger_meta (schema_version, store_kind, instance_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                STORE_SCHEMA_VERSION,
                STORE_KIND,
                instance_id.as_str(),
                now_millis,
                now_millis
            ],
        )
        .map_err(sqlite_write_error)?;

        // Initialize instance_runtime_state row if absent
        conn.execute(
            "INSERT OR IGNORE INTO instance_runtime_state (
                instance_id, active_session_id, active_turn_id, pending_input_id, pending_approval_id,
                last_message_seq, last_control_event_seq, last_write_seq,
                runtime_status, lock_version, dirty_shutdown, recovery_required, updated_at
            ) VALUES (?1, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 'idle', 0, 0, 0, ?2)",
            params![instance_id.as_str(), now_millis],
        )
        .map_err(sqlite_write_error)?;

        return Ok(());
    }

    validate_meta(conn, instance_id)
}

pub fn validate_meta(conn: &Connection, instance_id: &InstanceId) -> AicoreResult<()> {
    let (schema_version, store_kind, stored_instance_id): (i64, String, String) = conn
        .query_row(
            "SELECT schema_version, store_kind, instance_id FROM ledger_meta LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(sqlite_schema_error)?;

    if schema_version != STORE_SCHEMA_VERSION {
        return Err(AicoreError::VersionMismatch(format!(
            "unsupported session ledger schema version: {schema_version}"
        )));
    }

    if store_kind != STORE_KIND {
        return Err(AicoreError::InvalidState(format!(
            "unexpected session ledger kind: {store_kind}"
        )));
    }

    if stored_instance_id != instance_id.as_str() {
        return Err(AicoreError::Conflict(format!(
            "session ledger instance id mismatch: expected {}, got {}",
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

pub fn table_names(conn: &Connection) -> AicoreResult<Vec<String>> {
    let mut stmt = conn
        .prepare(
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name"
        )
        .map_err(sqlite_schema_error)?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(sqlite_schema_error)?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(sqlite_schema_error)
}
