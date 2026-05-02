use aicore_foundation::{AicoreError, AicoreResult, InstanceId};
use aicore_session::types::{
    ApprovalDecision, ApprovalRecord, ApprovalResponseOutcome, ApprovalResponseRequest,
    ApprovalResponseStatus, ApprovalStatus, ControlEventKind, CreateApprovalRequest,
    LedgerWriteKind, TurnStatus,
};
use rusqlite::params;

use crate::error::{sqlite_read_error, sqlite_write_error};
use crate::store::SqliteSessionStore;
use crate::store::control_helpers::{
    current_millis, invalidate_open_approvals, parse_approval_status, write_control_event,
    write_ledger_write,
};
use crate::store::helpers::ensure_request_instance;

impl SqliteSessionStore {
    pub(crate) fn create_approval_impl(
        &self,
        request: &CreateApprovalRequest,
    ) -> AicoreResult<ApprovalRecord> {
        ensure_request_instance(self.instance_id.as_str(), request.instance_id.as_str())?;
        let now = request.created_at.unix_millis() as i64;
        let mut conn = self.lock_connection()?;
        let tx = conn.transaction().map_err(sqlite_write_error)?;
        let active_turn_id: Option<String> = tx
            .query_row(
                "SELECT active_turn_id FROM instance_runtime_state WHERE instance_id = ?1",
                params![self.instance_id.as_str()],
                |row| row.get(0),
            )
            .map_err(sqlite_read_error)?;
        if active_turn_id.as_deref() != Some(request.turn_id.as_str()) {
            return Err(AicoreError::Conflict(format!(
                "approval requires active turn: {}",
                request.turn_id
            )));
        }
        tx.execute(
            "INSERT INTO approvals (approval_id, instance_id, turn_id, status, scope, summary, created_at)
             VALUES (?1, ?2, ?3, 'pending', ?4, ?5, ?6)",
            params![
                request.approval_id,
                self.instance_id.as_str(),
                request.turn_id,
                request.scope.as_str(),
                request.summary,
                now,
            ],
        )
        .map_err(sqlite_write_error)?;
        tx.execute(
            "UPDATE turns SET status = ?1 WHERE turn_id = ?2",
            params![TurnStatus::WaitingApproval.as_str(), request.turn_id],
        )
        .map_err(sqlite_write_error)?;
        tx.execute(
            "UPDATE instance_runtime_state SET pending_approval_id = ?1, runtime_status = 'waiting_approval', updated_at = ?2 WHERE instance_id = ?3",
            params![request.approval_id, now, self.instance_id.as_str()],
        )
        .map_err(sqlite_write_error)?;
        write_control_event(
            &tx,
            self.instance_id.as_str(),
            Some(&request.turn_id),
            ControlEventKind::ApprovalCreated,
            "approval_created",
            now,
        )?;
        write_ledger_write(
            &tx,
            self.instance_id.as_str(),
            Some(&request.turn_id),
            LedgerWriteKind::Insert,
            "approvals",
            &request.approval_id,
            now,
        )?;
        tx.commit().map_err(sqlite_write_error)?;
        Ok(ApprovalRecord {
            approval_id: request.approval_id.clone(),
            instance_id: self.instance_id.as_str().to_string(),
            turn_id: request.turn_id.clone(),
            status: ApprovalStatus::Pending,
            scope: request.scope,
            summary: request.summary.clone(),
            created_at: now as u128,
            resolved_at: None,
            resolved_response_id: None,
        })
    }

    pub(crate) fn respond_approval_first_writer_wins_impl(
        &self,
        request: &ApprovalResponseRequest,
    ) -> AicoreResult<ApprovalResponseOutcome> {
        ensure_request_instance(self.instance_id.as_str(), request.instance_id.as_str())?;
        let now = request.responded_at.unix_millis() as i64;
        let mut conn = self.lock_connection()?;
        let tx = conn.transaction().map_err(sqlite_write_error)?;
        let row: (String, String, Option<String>, Option<String>) = tx
            .query_row(
                "SELECT status, turn_id, resolved_response_id,
                        (SELECT active_turn_id FROM instance_runtime_state WHERE instance_id = approvals.instance_id)
                 FROM approvals WHERE approval_id = ?1 AND instance_id = ?2",
                params![request.approval_id, self.instance_id.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .map_err(sqlite_read_error)?;
        let (approval_status, turn_id, resolved_response_id, active_turn_id) = row;
        let (response_status, final_status, final_winner) = if approval_status != "pending" {
            let stale = approval_status.starts_with("invalidated") || approval_status == "stale";
            (
                if stale {
                    ApprovalResponseStatus::RejectedStale
                } else {
                    ApprovalResponseStatus::RejectedAlreadyResolved
                },
                approval_status,
                resolved_response_id,
            )
        } else if active_turn_id.as_deref() != Some(turn_id.as_str()) {
            (
                ApprovalResponseStatus::RejectedTurnNotActive,
                approval_status,
                resolved_response_id,
            )
        } else {
            let accepted_status = match request.decision {
                ApprovalDecision::Approve => ApprovalStatus::Approved,
                ApprovalDecision::Reject => ApprovalStatus::Rejected,
            };
            let updated = tx
                .execute(
                    "UPDATE approvals
                     SET status = ?1, resolved_at = ?2, resolved_response_id = ?3
                     WHERE approval_id = ?4 AND status = 'pending'",
                    params![
                        accepted_status.as_str(),
                        now,
                        request.response_id,
                        request.approval_id
                    ],
                )
                .map_err(sqlite_write_error)?;
            if updated == 1 {
                tx.execute(
                    "UPDATE instance_runtime_state SET pending_approval_id = NULL, runtime_status = 'running', updated_at = ?1 WHERE instance_id = ?2",
                    params![now, self.instance_id.as_str()],
                )
                .map_err(sqlite_write_error)?;
                tx.execute(
                    "UPDATE turns SET status = ?1 WHERE turn_id = ?2",
                    params![TurnStatus::Running.as_str(), turn_id],
                )
                .map_err(sqlite_write_error)?;
                (
                    ApprovalResponseStatus::Accepted,
                    accepted_status.as_str().to_string(),
                    Some(request.response_id.clone()),
                )
            } else {
                let (current_status, current_winner): (String, Option<String>) = tx
                    .query_row(
                        "SELECT status, resolved_response_id
                         FROM approvals WHERE approval_id = ?1 AND instance_id = ?2",
                        params![request.approval_id, self.instance_id.as_str()],
                        |row| Ok((row.get(0)?, row.get(1)?)),
                    )
                    .map_err(sqlite_read_error)?;
                let stale = current_status.starts_with("invalidated") || current_status == "stale";
                (
                    if stale {
                        ApprovalResponseStatus::RejectedStale
                    } else {
                        ApprovalResponseStatus::RejectedAlreadyResolved
                    },
                    current_status,
                    current_winner,
                )
            }
        };
        tx.execute(
            "INSERT INTO approval_responses (response_id, approval_id, instance_id, decision, status, responder_client_id, responder_client_kind, responded_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                request.response_id,
                request.approval_id,
                self.instance_id.as_str(),
                request.decision.as_str(),
                response_status.as_str(),
                request.responder_client_id.as_deref(),
                request.responder_client_kind.as_deref(),
                now,
            ],
        )
        .map_err(sqlite_write_error)?;
        write_control_event(
            &tx,
            self.instance_id.as_str(),
            Some(&turn_id),
            ControlEventKind::ApprovalResolved,
            "approval_resolved",
            now,
        )?;
        write_ledger_write(
            &tx,
            self.instance_id.as_str(),
            Some(&turn_id),
            LedgerWriteKind::Insert,
            "approval_responses",
            &request.response_id,
            now,
        )?;
        tx.commit().map_err(sqlite_write_error)?;
        Ok(ApprovalResponseOutcome {
            response_id: request.response_id.clone(),
            status: response_status,
            approval_status: parse_approval_status(&final_status),
            resolved_response_id: final_winner,
        })
    }

    pub(crate) fn invalidate_open_approvals_for_turn_impl(
        &self,
        instance_id: &InstanceId,
        turn_id: &str,
        status: ApprovalStatus,
    ) -> AicoreResult<u64> {
        ensure_request_instance(self.instance_id.as_str(), instance_id.as_str())?;
        let mut conn = self.lock_connection()?;
        let tx = conn.transaction().map_err(sqlite_write_error)?;
        let now = current_millis();
        let updated =
            invalidate_open_approvals(&tx, self.instance_id.as_str(), turn_id, status, now)?;
        if updated > 0 {
            write_control_event(
                &tx,
                self.instance_id.as_str(),
                Some(turn_id),
                ControlEventKind::ApprovalInvalidated,
                "approval_invalidated",
                now,
            )?;
        }
        tx.commit().map_err(sqlite_write_error)?;
        Ok(updated)
    }
}
