use std::time::{SystemTime, UNIX_EPOCH};

use aicore_session::types::{
    ApprovalDecision, ApprovalResponseStatus, ApprovalScope, ApprovalStatus, MessageKind,
    PendingInputStatus, RuntimeStatus, SessionStatus, TurnStatus,
};

pub fn parse_session_status(s: &str) -> SessionStatus {
    match s {
        "archived" => SessionStatus::Archived,
        _ => SessionStatus::Active,
    }
}

pub fn parse_message_kind(s: &str) -> MessageKind {
    match s {
        "assistant_delta" => MessageKind::AssistantDelta,
        "assistant_final" => MessageKind::AssistantFinal,
        "system" => MessageKind::System,
        "tool_call" => MessageKind::ToolCall,
        "tool_result" => MessageKind::ToolResult,
        _ => MessageKind::User,
    }
}

pub fn parse_runtime_status(s: &str) -> RuntimeStatus {
    match s {
        "running" => RuntimeStatus::Running,
        "waiting_approval" => RuntimeStatus::WaitingApproval,
        "stopping" => RuntimeStatus::Stopping,
        _ => RuntimeStatus::Idle,
    }
}

pub fn ensure_request_instance(
    store_instance_id: &str,
    request_instance_id: &str,
) -> aicore_foundation::AicoreResult<()> {
    if store_instance_id == request_instance_id {
        Ok(())
    } else {
        Err(aicore_foundation::AicoreError::Conflict(format!(
            "session ledger instance id mismatch: expected {store_instance_id}, got {request_instance_id}"
        )))
    }
}

pub fn next_event_seq(
    tx: &rusqlite::Connection,
    instance_id: &str,
    turn_id: Option<&str>,
) -> Result<i64, aicore_foundation::AicoreError> {
    use rusqlite::OptionalExtension;
    use rusqlite::params;
    let current: Option<i64> = if let Some(turn_id) = turn_id {
        tx.query_row(
            "SELECT MAX(event_seq) FROM control_events WHERE turn_id = ?1",
            params![turn_id],
            |row| row.get::<_, Option<i64>>(0),
        )
        .optional()
        .map_err(crate::error::sqlite_schema_error)?
        .flatten()
    } else {
        tx.query_row(
            "SELECT MAX(event_seq) FROM control_events WHERE instance_id = ?1 AND turn_id IS NULL",
            params![instance_id],
            |row| row.get::<_, Option<i64>>(0),
        )
        .optional()
        .map_err(crate::error::sqlite_schema_error)?
        .flatten()
    };
    Ok(current.unwrap_or(0) + 1)
}

pub fn next_write_seq(
    tx: &rusqlite::Connection,
    instance_id: &str,
    turn_id: Option<&str>,
) -> Result<i64, aicore_foundation::AicoreError> {
    use rusqlite::OptionalExtension;
    use rusqlite::params;
    let current: Option<i64> = if let Some(turn_id) = turn_id {
        tx.query_row(
            "SELECT MAX(write_seq) FROM ledger_writes WHERE turn_id = ?1",
            params![turn_id],
            |row| row.get::<_, Option<i64>>(0),
        )
        .optional()
        .map_err(crate::error::sqlite_schema_error)?
        .flatten()
    } else {
        tx.query_row(
            "SELECT MAX(write_seq) FROM ledger_writes WHERE instance_id = ?1 AND turn_id IS NULL",
            params![instance_id],
            |row| row.get::<_, Option<i64>>(0),
        )
        .optional()
        .map_err(crate::error::sqlite_schema_error)?
        .flatten()
    };
    Ok(current.unwrap_or(0) + 1)
}

pub fn uuidv7_str() -> String {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let suffix = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!(
        "{:08x}-{:04x}-7{:03x}-{:04x}-{:012x}",
        (ts >> 32) as u32,
        ((ts >> 16) & 0xFFFF) as u16,
        (ts & 0x0FFF) as u16,
        ((suffix >> 32) as u16) & 0x3FFF | 0x8000,
        suffix & 0xFFFFFFFFFFFF,
    )
}

pub fn parse_turn_status(s: &str) -> TurnStatus {
    match s {
        "running" => TurnStatus::Running,
        "waiting_approval" => TurnStatus::WaitingApproval,
        "stopping" => TurnStatus::Stopping,
        "stopped" => TurnStatus::Stopped,
        "completed" => TurnStatus::Completed,
        "interrupted" => TurnStatus::Interrupted,
        "cancelled" => TurnStatus::Cancelled,
        "failed" => TurnStatus::Failed,
        "interrupted_by_recovery" => TurnStatus::InterruptedByRecovery,
        _ => TurnStatus::Active,
    }
}

pub fn parse_pending_input_status(s: &str) -> PendingInputStatus {
    match s {
        "confirmed" => PendingInputStatus::Confirmed,
        "cancelled" => PendingInputStatus::Cancelled,
        "replaced" => PendingInputStatus::Replaced,
        "stale" => PendingInputStatus::Stale,
        _ => PendingInputStatus::Pending,
    }
}

pub fn parse_approval_status(s: &str) -> ApprovalStatus {
    match s {
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

pub fn parse_approval_scope(s: &str) -> ApprovalScope {
    match s {
        "single_tool_call" => ApprovalScope::SingleToolCall,
        _ => ApprovalScope::SingleToolCall,
    }
}

pub fn parse_approval_decision(s: &str) -> ApprovalDecision {
    match s {
        "reject" => ApprovalDecision::Reject,
        _ => ApprovalDecision::Approve,
    }
}

pub fn parse_approval_response_status(s: &str) -> ApprovalResponseStatus {
    match s {
        "rejected_stale" => ApprovalResponseStatus::RejectedStale,
        "rejected_already_resolved" => ApprovalResponseStatus::RejectedAlreadyResolved,
        "rejected_turn_not_active" => ApprovalResponseStatus::RejectedTurnNotActive,
        _ => ApprovalResponseStatus::Accepted,
    }
}
