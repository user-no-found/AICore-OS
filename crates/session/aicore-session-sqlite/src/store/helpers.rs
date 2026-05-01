use std::time::{SystemTime, UNIX_EPOCH};

use aicore_session::types::{MessageKind, RuntimeStatus, SessionStatus};

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

pub fn next_event_seq(
    tx: &rusqlite::Connection,
    instance_id: &str,
) -> Result<i64, aicore_foundation::AicoreError> {
    use rusqlite::OptionalExtension;
    use rusqlite::params;
    let current: Option<i64> = tx
        .query_row(
            "SELECT MAX(event_seq) FROM control_events WHERE instance_id = ?1",
            params![instance_id],
            |row| row.get::<_, Option<i64>>(0),
        )
        .optional()
        .map_err(crate::error::sqlite_schema_error)?
        .flatten();
    Ok(current.unwrap_or(0) + 1)
}

pub fn next_write_seq(
    tx: &rusqlite::Connection,
    instance_id: &str,
) -> Result<i64, aicore_foundation::AicoreError> {
    use rusqlite::OptionalExtension;
    use rusqlite::params;
    let current: Option<i64> = tx
        .query_row(
            "SELECT MAX(write_seq) FROM ledger_writes WHERE instance_id = ?1",
            params![instance_id],
            |row| row.get::<_, Option<i64>>(0),
        )
        .optional()
        .map_err(crate::error::sqlite_schema_error)?
        .flatten();
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
