use aicore_foundation::{AicoreError, AicoreResult};
use aicore_session::types::{ApprovalStatus, ControlEventKind, LedgerWriteKind};
use rusqlite::{Transaction, params};

use crate::error::{sqlite_schema_error, sqlite_write_error};
use crate::store::helpers::{next_event_seq, next_write_seq, uuidv7_str};

pub(crate) fn ensure_session_exists(tx: &Transaction<'_>, session_id: &str) -> AicoreResult<()> {
    let count: i64 = tx
        .query_row(
            "SELECT COUNT(*) FROM sessions WHERE session_id = ?1",
            params![session_id],
            |row| row.get(0),
        )
        .map_err(sqlite_schema_error)?;
    if count == 0 {
        Err(AicoreError::Missing(format!(
            "session not found: {session_id}"
        )))
    } else {
        Ok(())
    }
}

pub(crate) fn invalidate_open_approvals(
    tx: &Transaction<'_>,
    instance_id: &str,
    turn_id: &str,
    status: ApprovalStatus,
    now: i64,
) -> AicoreResult<u64> {
    let updated = tx
        .execute(
            "UPDATE approvals
             SET status = ?1, resolved_at = ?2
             WHERE instance_id = ?3 AND turn_id = ?4 AND status = 'pending'",
            params![status.as_str(), now, instance_id, turn_id],
        )
        .map_err(sqlite_write_error)?;
    Ok(updated as u64)
}

pub(crate) fn write_control_event(
    tx: &Transaction<'_>,
    instance_id: &str,
    turn_id: Option<&str>,
    kind: ControlEventKind,
    detail: &str,
    now: i64,
) -> AicoreResult<()> {
    let event_seq = next_event_seq(tx, instance_id, turn_id)?;
    let event_id = uuidv7_str();
    tx.execute(
        "INSERT INTO control_events (event_id, instance_id, turn_id, event_seq, event_type, detail, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            event_id,
            instance_id,
            turn_id,
            event_seq,
            kind.as_str(),
            detail,
            now
        ],
    )
    .map_err(sqlite_write_error)?;
    Ok(())
}

pub(crate) fn write_ledger_write(
    tx: &Transaction<'_>,
    instance_id: &str,
    turn_id: Option<&str>,
    kind: LedgerWriteKind,
    target_table: &str,
    target_id: &str,
    now: i64,
) -> AicoreResult<()> {
    let write_seq = next_write_seq(tx, instance_id, turn_id)?;
    let write_id = uuidv7_str();
    tx.execute(
        "INSERT INTO ledger_writes (write_id, instance_id, turn_id, write_seq, write_type, target_table, target_id, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            write_id,
            instance_id,
            turn_id,
            write_seq,
            kind.as_str(),
            target_table,
            target_id,
            now
        ],
    )
    .map_err(sqlite_write_error)?;
    Ok(())
}

pub(crate) fn parse_approval_status(value: &str) -> ApprovalStatus {
    match value {
        "approved" => ApprovalStatus::Approved,
        "rejected" => ApprovalStatus::Rejected,
        "cancelled" => ApprovalStatus::Cancelled,
        "expired" => ApprovalStatus::Expired,
        "stale" => ApprovalStatus::Stale,
        "invalidated_by_stop" => ApprovalStatus::InvalidatedByStop,
        "invalidated_by_turn_close" => ApprovalStatus::InvalidatedByTurnClose,
        "invalidated_by_recovery" => ApprovalStatus::InvalidatedByRecovery,
        _ => ApprovalStatus::Pending,
    }
}

pub(crate) fn current_millis() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default()
}
