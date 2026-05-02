use aicore_foundation::{AicoreClock, AicoreResult, SystemClock};
use aicore_session::types::{
    BeginTurnRequest, ControlEventKind, CreateSessionRequest, FinishTurnRequest, LedgerWriteKind,
    TurnStatus,
};
use rusqlite::params;

use crate::error::{sqlite_read_error, sqlite_schema_error, sqlite_write_error};
use crate::store::SqliteSessionStore;
use crate::store::helpers::{ensure_request_instance, next_event_seq, next_write_seq, uuidv7_str};

impl SqliteSessionStore {
    pub(crate) fn create_session_impl(&self, request: &CreateSessionRequest) -> AicoreResult<()> {
        ensure_request_instance(self.instance_id.as_str(), request.instance_id.as_str())?;
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

        tx.execute(
            "UPDATE instance_runtime_state
             SET active_session_id = ?1, active_turn_id = NULL, runtime_status = 'idle', updated_at = ?2
             WHERE instance_id = ?3",
            params![request.session_id.as_str(), now, self.instance_id.as_str()],
        )
        .map_err(sqlite_write_error)?;

        let event_seq = next_event_seq(&tx, self.instance_id.as_str(), None)?;
        let event_id = uuidv7_str();
        tx.execute(
            "INSERT INTO control_events (event_id, instance_id, event_seq, event_type, detail, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                &event_id,
                self.instance_id.as_str(),
                event_seq,
                ControlEventKind::SessionCreated.as_str(),
                format!("session_created: {}", request.session_id.as_str()),
                now,
            ],
        )
        .map_err(sqlite_write_error)?;

        let write_seq = next_write_seq(&tx, self.instance_id.as_str(), None)?;
        let write_id = uuidv7_str();
        tx.execute(
            "INSERT INTO ledger_writes (write_id, instance_id, write_seq, write_type, target_table, target_id, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                &write_id,
                self.instance_id.as_str(),
                write_seq,
                LedgerWriteKind::Insert.as_str(),
                "sessions",
                request.session_id.as_str(),
                now,
            ],
        )
        .map_err(sqlite_write_error)?;

        tx.commit().map_err(sqlite_write_error)
    }

    pub(crate) fn begin_turn_impl(&self, request: &BeginTurnRequest) -> AicoreResult<()> {
        ensure_request_instance(self.instance_id.as_str(), request.instance_id.as_str())?;
        let now = SystemClock.now().unix_millis() as i64;
        let mut conn = self.lock_connection()?;
        let tx = conn.transaction().map_err(sqlite_write_error)?;

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

        let (active_turn_id, lock_version): (Option<String>, i64) = tx
            .query_row(
                "SELECT active_turn_id, lock_version FROM instance_runtime_state WHERE instance_id = ?1",
                params![self.instance_id.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(sqlite_read_error)?;
        if let Some(active_turn_id) = active_turn_id {
            return Err(aicore_foundation::AicoreError::Conflict(format!(
                "active turn already exists: {active_turn_id}"
            )));
        }

        tx.execute(
            "INSERT INTO turns (turn_id, session_id, turn_seq, status, started_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                request.turn_id,
                request.session_id.as_str(),
                request.turn_seq as i64,
                TurnStatus::Running.as_str(),
                now,
            ],
        )
        .map_err(sqlite_write_error)?;

        let next_lock = lock_version + 1;
        tx.execute(
            "UPDATE instance_runtime_state
             SET active_session_id = ?1, active_turn_id = ?2, runtime_status = 'running',
                 lock_version = ?3, updated_at = ?4
             WHERE instance_id = ?5",
            params![
                request.session_id.as_str(),
                request.turn_id,
                next_lock,
                now,
                self.instance_id.as_str()
            ],
        )
        .map_err(sqlite_write_error)?;

        let event_seq = next_event_seq(&tx, self.instance_id.as_str(), Some(&request.turn_id))?;
        let event_id = uuidv7_str();
        tx.execute(
            "INSERT INTO control_events (event_id, instance_id, turn_id, event_seq, event_type, detail, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                &event_id,
                self.instance_id.as_str(),
                request.turn_id,
                event_seq,
                ControlEventKind::TurnBegan.as_str(),
                format!("turn_began: {} in {}", request.turn_id, request.session_id.as_str()),
                now,
            ],
        )
        .map_err(sqlite_write_error)?;

        let write_seq = next_write_seq(&tx, self.instance_id.as_str(), Some(&request.turn_id))?;
        let write_id = uuidv7_str();
        tx.execute(
            "INSERT INTO ledger_writes (write_id, instance_id, turn_id, write_seq, write_type, target_table, target_id, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                &write_id,
                self.instance_id.as_str(),
                request.turn_id,
                write_seq,
                LedgerWriteKind::Insert.as_str(),
                "turns",
                request.turn_id,
                now,
            ],
        )
        .map_err(sqlite_write_error)?;

        tx.commit().map_err(sqlite_write_error)
    }

    pub(crate) fn finish_turn_impl(&self, request: &FinishTurnRequest) -> AicoreResult<()> {
        ensure_request_instance(self.instance_id.as_str(), request.instance_id.as_str())?;
        let now = SystemClock.now().unix_millis() as i64;
        let mut conn = self.lock_connection()?;
        let tx = conn.transaction().map_err(sqlite_write_error)?;

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

        if !request.terminal_status.is_terminal() {
            return Err(aicore_foundation::AicoreError::InvalidState(
                "finish_turn requires terminal status".to_string(),
            ));
        }
        let (active_turn_id, lock_version): (Option<String>, i64) = tx
            .query_row(
                "SELECT active_turn_id, lock_version FROM instance_runtime_state WHERE instance_id = ?1",
                params![self.instance_id.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(sqlite_read_error)?;
        if active_turn_id.as_deref() != Some(request.turn_id.as_str()) {
            return Err(aicore_foundation::AicoreError::Conflict(format!(
                "cannot finish non-active turn: {}",
                request.turn_id
            )));
        }

        tx.execute(
            "UPDATE turns SET status = ?1, finished_at = ?2 WHERE turn_id = ?3",
            params![request.terminal_status.as_str(), now, request.turn_id],
        )
        .map_err(sqlite_write_error)?;

        let next_lock = lock_version + 1;
        tx.execute(
            "UPDATE instance_runtime_state
             SET active_turn_id = NULL, runtime_status = 'idle', lock_version = ?1, updated_at = ?2
             WHERE instance_id = ?3",
            params![next_lock, now, self.instance_id.as_str()],
        )
        .map_err(sqlite_write_error)?;

        let event_seq = next_event_seq(&tx, self.instance_id.as_str(), Some(&request.turn_id))?;
        let event_id = uuidv7_str();
        tx.execute(
            "INSERT INTO control_events (event_id, instance_id, turn_id, event_seq, event_type, detail, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                &event_id,
                self.instance_id.as_str(),
                request.turn_id,
                event_seq,
                ControlEventKind::TurnFinished.as_str(),
                format!("turn_finished: {} -> {}", request.turn_id, request.terminal_status.as_str()),
                now,
            ],
        )
        .map_err(sqlite_write_error)?;

        let write_seq = next_write_seq(
            &tx,
            self.instance_id.as_str(),
            Some(request.turn_id.as_str()),
        )?;
        let write_id = uuidv7_str();
        tx.execute(
            "INSERT INTO ledger_writes (write_id, instance_id, turn_id, write_seq, write_type, target_table, target_id, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                &write_id,
                self.instance_id.as_str(),
                request.turn_id,
                write_seq,
                LedgerWriteKind::Update.as_str(),
                "turns",
                request.turn_id,
                now,
            ],
        )
        .map_err(sqlite_write_error)?;

        tx.commit().map_err(sqlite_write_error)
    }
}
