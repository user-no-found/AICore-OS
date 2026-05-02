use aicore_foundation::AicoreResult;
use aicore_session::types::{
    ControlEventKind, LedgerWriteKind, PendingInputCancelOutcome, PendingInputCancelRequest,
    PendingInputStatus, PendingInputSubmitOutcome, PendingInputSubmitRequest,
};
use rusqlite::{OptionalExtension, params};

use crate::error::{sqlite_read_error, sqlite_write_error};
use crate::store::SqliteSessionStore;
use crate::store::control_helpers::{write_control_event, write_ledger_write};
use crate::store::helpers::ensure_request_instance;

impl SqliteSessionStore {
    pub(crate) fn submit_or_replace_pending_input_impl(
        &self,
        request: &PendingInputSubmitRequest,
    ) -> AicoreResult<PendingInputSubmitOutcome> {
        ensure_request_instance(self.instance_id.as_str(), request.instance_id.as_str())?;
        let now = request.submitted_at.unix_millis() as i64;
        let mut conn = self.lock_connection()?;
        let tx = conn.transaction().map_err(sqlite_write_error)?;
        let replaced: Option<String> = tx
            .query_row(
                "SELECT pending_input_id FROM pending_inputs WHERE instance_id = ?1 AND status = 'pending'",
                params![self.instance_id.as_str()],
                |row| row.get(0),
            )
            .optional()
            .map_err(sqlite_read_error)?;
        if let Some(ref pending_input_id) = replaced {
            tx.execute(
                "UPDATE pending_inputs SET status = 'replaced', updated_at = ?1 WHERE pending_input_id = ?2",
                params![now, pending_input_id],
            )
            .map_err(sqlite_write_error)?;
        }
        tx.execute(
            "INSERT INTO pending_inputs (pending_input_id, instance_id, session_id, turn_id, content, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 'pending', ?6, ?6)",
            params![
                request.pending_input_id,
                self.instance_id.as_str(),
                request.session_id.as_deref(),
                request.turn_id.as_deref(),
                request.content,
                now,
            ],
        )
        .map_err(sqlite_write_error)?;
        tx.execute(
            "UPDATE instance_runtime_state SET pending_input_id = ?1, updated_at = ?2 WHERE instance_id = ?3",
            params![request.pending_input_id, now, self.instance_id.as_str()],
        )
        .map_err(sqlite_write_error)?;
        write_control_event(
            &tx,
            self.instance_id.as_str(),
            request.turn_id.as_deref(),
            ControlEventKind::PendingInputSubmitted,
            "pending_input_submitted",
            now,
        )?;
        write_ledger_write(
            &tx,
            self.instance_id.as_str(),
            request.turn_id.as_deref(),
            LedgerWriteKind::Insert,
            "pending_inputs",
            &request.pending_input_id,
            now,
        )?;
        tx.commit().map_err(sqlite_write_error)?;
        Ok(PendingInputSubmitOutcome {
            pending_input_id: request.pending_input_id.clone(),
            replaced_pending_input_id: replaced,
            status: PendingInputStatus::Pending,
        })
    }

    pub(crate) fn cancel_pending_input_impl(
        &self,
        request: &PendingInputCancelRequest,
    ) -> AicoreResult<PendingInputCancelOutcome> {
        ensure_request_instance(self.instance_id.as_str(), request.instance_id.as_str())?;
        let now = request.cancelled_at.unix_millis() as i64;
        let mut conn = self.lock_connection()?;
        let tx = conn.transaction().map_err(sqlite_write_error)?;
        let current: Option<String> = tx
            .query_row(
                "SELECT pending_input_id FROM pending_inputs WHERE instance_id = ?1 AND status = 'pending'",
                params![self.instance_id.as_str()],
                |row| row.get(0),
            )
            .optional()
            .map_err(sqlite_read_error)?;
        if let Some(ref pending_input_id) = current {
            tx.execute(
                "UPDATE pending_inputs SET status = 'cancelled', updated_at = ?1 WHERE pending_input_id = ?2",
                params![now, pending_input_id],
            )
            .map_err(sqlite_write_error)?;
            tx.execute(
                "UPDATE instance_runtime_state SET pending_input_id = NULL, updated_at = ?1 WHERE instance_id = ?2",
                params![now, self.instance_id.as_str()],
            )
            .map_err(sqlite_write_error)?;
            write_control_event(
                &tx,
                self.instance_id.as_str(),
                None,
                ControlEventKind::PendingInputCancelled,
                "pending_input_cancelled",
                now,
            )?;
            write_ledger_write(
                &tx,
                self.instance_id.as_str(),
                None,
                LedgerWriteKind::Update,
                "pending_inputs",
                pending_input_id,
                now,
            )?;
        }
        tx.commit().map_err(sqlite_write_error)?;
        Ok(PendingInputCancelOutcome {
            cancelled_pending_input_id: current,
        })
    }
}
