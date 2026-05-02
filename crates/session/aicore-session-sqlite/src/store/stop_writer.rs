use aicore_foundation::AicoreResult;
use aicore_session::types::{
    ApprovalStatus, ControlEventKind, LedgerWriteKind, StopTurnOutcome, StopTurnRequest,
    StopTurnStatus,
};
use rusqlite::params;

use crate::error::{sqlite_read_error, sqlite_write_error};
use crate::store::SqliteSessionStore;
use crate::store::control_helpers::{
    invalidate_open_approvals, write_control_event, write_ledger_write,
};
use crate::store::helpers::ensure_request_instance;

impl SqliteSessionStore {
    pub(crate) fn request_stop_active_turn_impl(
        &self,
        request: &StopTurnRequest,
    ) -> AicoreResult<StopTurnOutcome> {
        ensure_request_instance(self.instance_id.as_str(), request.instance_id.as_str())?;
        let now = request.requested_at.unix_millis() as i64;
        let mut conn = self.lock_connection()?;
        let tx = conn.transaction().map_err(sqlite_write_error)?;
        let (active_turn_id, lock_version): (Option<String>, i64) = tx
            .query_row(
                "SELECT active_turn_id, lock_version FROM instance_runtime_state WHERE instance_id = ?1",
                params![self.instance_id.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(sqlite_read_error)?;
        let Some(turn_id) = active_turn_id else {
            tx.commit().map_err(sqlite_write_error)?;
            return Ok(StopTurnOutcome {
                status: StopTurnStatus::NoActiveTurn,
                turn_id: None,
                lock_version: lock_version as u64,
            });
        };
        tx.execute(
            "UPDATE turns SET status = 'stopped', finished_at = ?1 WHERE turn_id = ?2",
            params![now, turn_id],
        )
        .map_err(sqlite_write_error)?;
        let invalidated = invalidate_open_approvals(
            &tx,
            self.instance_id.as_str(),
            &turn_id,
            ApprovalStatus::InvalidatedByStop,
            now,
        )?;
        let next_lock = lock_version + 1;
        tx.execute(
            "UPDATE instance_runtime_state
             SET active_turn_id = NULL, pending_approval_id = NULL, runtime_status = 'idle',
                 lock_version = ?1, updated_at = ?2
             WHERE instance_id = ?3",
            params![next_lock, now, self.instance_id.as_str()],
        )
        .map_err(sqlite_write_error)?;
        write_control_event(
            &tx,
            self.instance_id.as_str(),
            Some(&turn_id),
            ControlEventKind::StopRequested,
            "stop_requested",
            now,
        )?;
        write_ledger_write(
            &tx,
            self.instance_id.as_str(),
            Some(&turn_id),
            LedgerWriteKind::Update,
            "turns",
            &turn_id,
            now,
        )?;
        if invalidated > 0 {
            write_control_event(
                &tx,
                self.instance_id.as_str(),
                Some(&turn_id),
                ControlEventKind::ApprovalInvalidated,
                "approval_invalidated",
                now,
            )?;
        }
        tx.commit().map_err(sqlite_write_error)?;
        Ok(StopTurnOutcome {
            status: StopTurnStatus::StopRequested,
            turn_id: Some(turn_id),
            lock_version: next_lock as u64,
        })
    }
}
