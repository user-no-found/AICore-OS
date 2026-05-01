use aicore_foundation::{AicoreClock, AicoreResult, SystemClock};
use aicore_session::traits::SessionLedgerWriter;
use aicore_session::types::{
    AppendMessageRequest, BeginTurnRequest, ControlEventType, CreateSessionRequest,
    FinishTurnRequest, LedgerWriteType, TurnStatus,
};
use rusqlite::params;

use crate::error::{sqlite_schema_error, sqlite_write_error};
use crate::store::SqliteSessionStore;
use crate::store::helpers::{next_event_seq, next_write_seq, uuidv7_str};

impl SessionLedgerWriter for SqliteSessionStore {
    fn create_session(&self, request: &CreateSessionRequest) -> AicoreResult<()> {
        let now = SystemClock.now().unix_millis() as i64;
        let mut conn = self.lock_connection()?;
        let tx = conn.transaction().map_err(sqlite_write_error)?;

        let metadata_json = request
            .metadata
            .as_ref()
            .map(|v| v.to_string())
            .unwrap_or_default();

        tx.execute(
            "INSERT INTO sessions (session_id, title, status, created_at, updated_at, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                request.session_id.as_str(),
                request.title,
                "active",
                now,
                now,
                if metadata_json.is_empty() {
                    None
                } else {
                    Some(&metadata_json)
                },
            ],
        )
        .map_err(sqlite_write_error)?;

        // Update instance_runtime_state
        tx.execute(
            "UPDATE instance_runtime_state
             SET active_session_id = ?1, active_turn_id = NULL, runtime_status = 'idle',
                 updated_at = ?2
             WHERE instance_id = ?3",
            params![request.session_id.as_str(), now, self.instance_id.as_str()],
        )
        .map_err(sqlite_write_error)?;

        // Control event
        let event_seq = next_event_seq(&tx, self.instance_id.as_str())?;
        let event_id = uuidv7_str();
        tx.execute(
            "INSERT INTO control_events (event_id, instance_id, event_seq, event_type, detail, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                &event_id,
                self.instance_id.as_str(),
                event_seq,
                ControlEventType::SessionCreated.as_str(),
                format!("session_created: {}", request.session_id.as_str()),
                now,
            ],
        )
        .map_err(sqlite_write_error)?;

        // Ledger audit
        let write_seq = next_write_seq(&tx, self.instance_id.as_str())?;
        let write_id = uuidv7_str();
        tx.execute(
            "INSERT INTO ledger_writes (write_id, instance_id, write_seq, write_type, target_table, target_id, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                &write_id,
                self.instance_id.as_str(),
                write_seq,
                LedgerWriteType::Insert.as_str(),
                "sessions",
                request.session_id.as_str(),
                now,
            ],
        )
        .map_err(sqlite_write_error)?;

        tx.commit().map_err(sqlite_write_error)
    }

    fn begin_turn(&self, request: &BeginTurnRequest) -> AicoreResult<()> {
        let now = SystemClock.now().unix_millis() as i64;
        let mut conn = self.lock_connection()?;
        let tx = conn.transaction().map_err(sqlite_write_error)?;

        // Verify session exists
        let session_exists: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM sessions WHERE session_id = ?1",
                params![request.session_id.as_str()],
                |row| row.get(0),
            )
            .map_err(sqlite_schema_error)?;
        if session_exists == 0 {
            return Err(aicore_foundation::AicoreError::Missing(format!(
                "session not found: {}",
                request.session_id.as_str()
            )));
        }

        tx.execute(
            "INSERT INTO turns (turn_id, session_id, turn_seq, status, started_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                request.turn_id,
                request.session_id.as_str(),
                request.turn_seq as i64,
                TurnStatus::Active.as_str(),
                now,
            ],
        )
        .map_err(sqlite_write_error)?;

        // Update instance_runtime_state
        tx.execute(
            "UPDATE instance_runtime_state
             SET active_session_id = ?1, active_turn_id = ?2, runtime_status = 'running',
                 updated_at = ?3
             WHERE instance_id = ?4",
            params![
                request.session_id.as_str(),
                request.turn_id,
                now,
                self.instance_id.as_str(),
            ],
        )
        .map_err(sqlite_write_error)?;

        // Control event
        let event_seq = next_event_seq(&tx, self.instance_id.as_str())?;
        let event_id = uuidv7_str();
        tx.execute(
            "INSERT INTO control_events (event_id, instance_id, turn_id, event_seq, event_type, detail, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                &event_id,
                self.instance_id.as_str(),
                request.turn_id,
                event_seq,
                ControlEventType::TurnBegan.as_str(),
                format!("turn_began: {} in {}", request.turn_id, request.session_id.as_str()),
                now,
            ],
        )
        .map_err(sqlite_write_error)?;

        // Ledger audit
        let write_seq = next_write_seq(&tx, self.instance_id.as_str())?;
        let write_id = uuidv7_str();
        tx.execute(
            "INSERT INTO ledger_writes (write_id, instance_id, turn_id, write_seq, write_type, target_table, target_id, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                &write_id,
                self.instance_id.as_str(),
                request.turn_id,
                write_seq,
                LedgerWriteType::Insert.as_str(),
                "turns",
                request.turn_id,
                now,
            ],
        )
        .map_err(sqlite_write_error)?;

        tx.commit().map_err(sqlite_write_error)
    }

    fn finish_turn(&self, request: &FinishTurnRequest) -> AicoreResult<()> {
        let now = SystemClock.now().unix_millis() as i64;
        let mut conn = self.lock_connection()?;
        let tx = conn.transaction().map_err(sqlite_write_error)?;

        // Verify turn exists
        let turn_exists: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM turns WHERE turn_id = ?1",
                params![request.turn_id],
                |row| row.get(0),
            )
            .map_err(sqlite_schema_error)?;
        if turn_exists == 0 {
            return Err(aicore_foundation::AicoreError::Missing(format!(
                "turn not found: {}",
                request.turn_id
            )));
        }

        tx.execute(
            "UPDATE turns
             SET status = ?1, finished_at = ?2
             WHERE turn_id = ?3",
            params![request.terminal_status.as_str(), now, request.turn_id],
        )
        .map_err(sqlite_write_error)?;

        // Clear active_turn_id
        tx.execute(
            "UPDATE instance_runtime_state
             SET active_turn_id = NULL, runtime_status = 'idle', updated_at = ?1
             WHERE instance_id = ?2",
            params![now, self.instance_id.as_str()],
        )
        .map_err(sqlite_write_error)?;

        // Control event
        let event_seq = next_event_seq(&tx, self.instance_id.as_str())?;
        let event_id = uuidv7_str();
        tx.execute(
            "INSERT INTO control_events (event_id, instance_id, turn_id, event_seq, event_type, detail, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                &event_id,
                self.instance_id.as_str(),
                request.turn_id,
                event_seq,
                ControlEventType::TurnFinished.as_str(),
                format!(
                    "turn_finished: {} -> {}",
                    request.turn_id,
                    request.terminal_status.as_str()
                ),
                now,
            ],
        )
        .map_err(sqlite_write_error)?;

        // Ledger audit
        let write_seq = next_write_seq(&tx, self.instance_id.as_str())?;
        let write_id = uuidv7_str();
        tx.execute(
            "INSERT INTO ledger_writes (write_id, instance_id, turn_id, write_seq, write_type, target_table, target_id, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                &write_id,
                self.instance_id.as_str(),
                request.turn_id,
                write_seq,
                LedgerWriteType::Update.as_str(),
                "turns",
                request.turn_id,
                now,
            ],
        )
        .map_err(sqlite_write_error)?;

        tx.commit().map_err(sqlite_write_error)
    }

    fn append_message(&self, request: &AppendMessageRequest) -> AicoreResult<()> {
        let now = SystemClock.now().unix_millis() as i64;
        let mut conn = self.lock_connection()?;
        let tx = conn.transaction().map_err(sqlite_write_error)?;

        // Verify session exists
        let session_exists: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM sessions WHERE session_id = ?1",
                params![request.session_id.as_str()],
                |row| row.get(0),
            )
            .map_err(sqlite_schema_error)?;
        if session_exists == 0 {
            return Err(aicore_foundation::AicoreError::Missing(format!(
                "session not found: {}",
                request.session_id.as_str()
            )));
        }

        // Verify turn exists if provided
        if let Some(ref turn_id) = request.turn_id {
            let turn_exists: i64 = tx
                .query_row(
                    "SELECT COUNT(*) FROM turns WHERE turn_id = ?1",
                    params![turn_id],
                    |row| row.get(0),
                )
                .map_err(sqlite_schema_error)?;
            if turn_exists == 0 {
                return Err(aicore_foundation::AicoreError::Missing(format!(
                    "turn not found: {turn_id}"
                )));
            }
        }

        let metadata_json = request
            .metadata
            .as_ref()
            .map(|v| v.to_string())
            .unwrap_or_default();

        tx.execute(
            "INSERT INTO messages (message_id, session_id, turn_id, message_seq, kind, content, created_at, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                request.message_id,
                request.session_id.as_str(),
                request.turn_id.as_deref(),
                request.message_seq as i64,
                request.kind.as_str(),
                request.content,
                now,
                if metadata_json.is_empty() {
                    None
                } else {
                    Some(&metadata_json)
                },
            ],
        )
        .map_err(sqlite_write_error)?;

        // Update last_message_seq
        tx.execute(
            "UPDATE instance_runtime_state
             SET last_message_seq = ?1, updated_at = ?2
             WHERE instance_id = ?3",
            params![request.message_seq as i64, now, self.instance_id.as_str()],
        )
        .map_err(sqlite_write_error)?;

        // Update session updated_at
        tx.execute(
            "UPDATE sessions SET updated_at = ?1 WHERE session_id = ?2",
            params![now, request.session_id.as_str()],
        )
        .map_err(sqlite_write_error)?;

        // Ledger audit (no control event by default)
        let write_seq = next_write_seq(&tx, self.instance_id.as_str())?;
        let write_id = uuidv7_str();
        tx.execute(
            "INSERT INTO ledger_writes (write_id, instance_id, turn_id, write_seq, write_type, target_table, target_id, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                &write_id,
                self.instance_id.as_str(),
                request.turn_id.as_deref(),
                write_seq,
                LedgerWriteType::Insert.as_str(),
                "messages",
                request.message_id,
                now,
            ],
        )
        .map_err(sqlite_write_error)?;

        tx.commit().map_err(sqlite_write_error)
    }

    fn create_pending_input(&self) -> AicoreResult<()> {
        Err(crate::error::unsupported_api("create_pending_input"))
    }

    fn submit_approval(&self) -> AicoreResult<()> {
        Err(crate::error::unsupported_api("submit_approval"))
    }

    fn respond_approval(&self) -> AicoreResult<()> {
        Err(crate::error::unsupported_api("respond_approval"))
    }
}
