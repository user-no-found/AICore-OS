use aicore_foundation::{AicoreError, AicoreResult};
use aicore_session::types::{
    ActiveTurnAcquireOutcome, ActiveTurnAcquireRequest, ActiveTurnAcquireStatus,
    ActiveTurnReleaseOutcome, ActiveTurnReleaseRequest, ControlEventKind, LedgerWriteKind,
    TurnStatus,
};
use rusqlite::params;

use crate::error::{sqlite_read_error, sqlite_schema_error, sqlite_write_error};
use crate::store::SqliteSessionStore;
use crate::store::control_helpers::{
    ensure_session_exists, write_control_event, write_ledger_write,
};
use crate::store::helpers::ensure_request_instance;

impl SqliteSessionStore {
    pub(crate) fn acquire_active_turn_impl(
        &self,
        request: &ActiveTurnAcquireRequest,
    ) -> AicoreResult<ActiveTurnAcquireOutcome> {
        ensure_request_instance(self.instance_id.as_str(), request.instance_id.as_str())?;
        let now = request.requested_at.unix_millis() as i64;
        let mut conn = self.lock_connection()?;
        let tx = conn.transaction().map_err(sqlite_write_error)?;

        ensure_session_exists(&tx, request.session_id.as_str())?;
        let (current, lock_version): (Option<String>, i64) = tx
            .query_row(
                "SELECT active_turn_id, lock_version FROM instance_runtime_state WHERE instance_id = ?1",
                params![self.instance_id.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(sqlite_read_error)?;
        if let Some(active_turn_id) = current {
            tx.commit().map_err(sqlite_write_error)?;
            return Ok(ActiveTurnAcquireOutcome {
                status: ActiveTurnAcquireStatus::AlreadyActive,
                active_turn_id: Some(active_turn_id),
                lock_version: lock_version as u64,
            });
        }

        let next_turn_seq: i64 = tx
            .query_row(
                "SELECT COALESCE(MAX(turn_seq), 0) + 1 FROM turns WHERE session_id = ?1",
                params![request.session_id.as_str()],
                |row| row.get(0),
            )
            .map_err(sqlite_schema_error)?;
        tx.execute(
            "INSERT INTO turns (turn_id, session_id, turn_seq, status, started_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                request.turn_id,
                request.session_id.as_str(),
                next_turn_seq,
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
                self.instance_id.as_str(),
            ],
        )
        .map_err(sqlite_write_error)?;
        write_control_event(
            &tx,
            self.instance_id.as_str(),
            Some(&request.turn_id),
            ControlEventKind::ActiveTurnAcquired,
            "active_turn_acquired",
            now,
        )?;
        write_ledger_write(
            &tx,
            self.instance_id.as_str(),
            Some(&request.turn_id),
            LedgerWriteKind::Insert,
            "turns",
            &request.turn_id,
            now,
        )?;
        tx.commit().map_err(sqlite_write_error)?;
        Ok(ActiveTurnAcquireOutcome {
            status: ActiveTurnAcquireStatus::Acquired,
            active_turn_id: Some(request.turn_id.clone()),
            lock_version: next_lock as u64,
        })
    }

    pub(crate) fn release_active_turn_impl(
        &self,
        request: &ActiveTurnReleaseRequest,
    ) -> AicoreResult<ActiveTurnReleaseOutcome> {
        ensure_request_instance(self.instance_id.as_str(), request.instance_id.as_str())?;
        if !request.terminal_status.is_terminal() {
            return Err(AicoreError::InvalidState(
                "active turn release requires terminal status".to_string(),
            ));
        }
        let now = request.released_at.unix_millis() as i64;
        let mut conn = self.lock_connection()?;
        let tx = conn.transaction().map_err(sqlite_write_error)?;
        let (active_turn_id, lock_version): (Option<String>, i64) = tx
            .query_row(
                "SELECT active_turn_id, lock_version FROM instance_runtime_state WHERE instance_id = ?1",
                params![self.instance_id.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(sqlite_read_error)?;
        if active_turn_id.as_deref() != Some(request.turn_id.as_str()) {
            return Err(AicoreError::Conflict(format!(
                "cannot release non-active turn: {}",
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
        write_control_event(
            &tx,
            self.instance_id.as_str(),
            Some(&request.turn_id),
            ControlEventKind::ActiveTurnReleased,
            "active_turn_released",
            now,
        )?;
        write_ledger_write(
            &tx,
            self.instance_id.as_str(),
            Some(&request.turn_id),
            LedgerWriteKind::Update,
            "turns",
            &request.turn_id,
            now,
        )?;
        tx.commit().map_err(sqlite_write_error)?;
        Ok(ActiveTurnReleaseOutcome {
            released: true,
            lock_version: next_lock as u64,
        })
    }
}
