use aicore_foundation::{AicoreResult, SessionId};
use aicore_session::traits::SessionLedgerReader;
use aicore_session::types::{
    ApprovalRecord, ApprovalResponseRecord, InstanceRuntimeSnapshot, MessageRecord,
    PendingInputRecord, SessionRecord, SessionSummary, TurnRecord,
};
use rusqlite::{OptionalExtension, params};

use crate::error::sqlite_read_error;
use crate::store::SqliteSessionStore;
use crate::store::helpers::{
    parse_approval_decision, parse_approval_response_status, parse_approval_scope,
    parse_approval_status, parse_message_kind, parse_pending_input_status, parse_runtime_status,
    parse_session_status, parse_turn_status,
};

impl SessionLedgerReader for SqliteSessionStore {
    fn get_session(&self, session_id: &SessionId) -> AicoreResult<Option<SessionRecord>> {
        let conn = self.lock_connection()?;
        let row = conn
            .query_row(
                "SELECT session_id, title, status, created_at, updated_at, metadata
                 FROM sessions WHERE session_id = ?1",
                params![session_id.as_str()],
                |row| {
                    let meta: Option<String> = row.get(5)?;
                    Ok(SessionRecord {
                        session_id: row.get(0)?,
                        title: row.get(1)?,
                        status: parse_session_status(&row.get::<_, String>(2)?),
                        created_at: row.get::<_, i64>(3)? as u128,
                        updated_at: row.get::<_, i64>(4)? as u128,
                        metadata: meta
                            .filter(|s| !s.is_empty())
                            .map(|s| serde_json::from_str(&s).ok())
                            .flatten(),
                    })
                },
            )
            .optional()
            .map_err(sqlite_read_error)?;
        Ok(row)
    }

    fn list_sessions(&self) -> AicoreResult<Vec<SessionSummary>> {
        let conn = self.lock_connection()?;
        let mut stmt = conn
            .prepare(
                "SELECT s.session_id, s.title, s.status, s.created_at, s.updated_at,
                        COUNT(t.turn_id) as turn_count
                 FROM sessions s
                 LEFT JOIN turns t ON t.session_id = s.session_id
                 GROUP BY s.session_id
                 ORDER BY s.updated_at DESC",
            )
            .map_err(sqlite_read_error)?;
        let rows = stmt
            .query_map([], |row| {
                Ok(SessionSummary {
                    session_id: row.get(0)?,
                    title: row.get(1)?,
                    status: parse_session_status(&row.get::<_, String>(2)?),
                    created_at: row.get::<_, i64>(3)? as u128,
                    updated_at: row.get::<_, i64>(4)? as u128,
                    turn_count: row.get::<_, i64>(5)? as u64,
                })
            })
            .map_err(sqlite_read_error)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(sqlite_read_error)
    }

    fn get_turn(&self, turn_id: &str) -> AicoreResult<Option<TurnRecord>> {
        let conn = self.lock_connection()?;
        conn.query_row(
            "SELECT turn_id, session_id, turn_seq, status, started_at, finished_at
             FROM turns WHERE turn_id = ?1",
            params![turn_id],
            |row| {
                let status: String = row.get(3)?;
                Ok(TurnRecord {
                    turn_id: row.get(0)?,
                    session_id: row.get(1)?,
                    turn_seq: row.get::<_, i64>(2)? as u64,
                    status: parse_turn_status(&status),
                    started_at: row.get::<_, i64>(4)? as u128,
                    finished_at: row.get::<_, Option<i64>>(5)?.map(|value| value as u128),
                })
            },
        )
        .optional()
        .map_err(sqlite_read_error)
    }

    fn read_messages(&self, session_id: &SessionId) -> AicoreResult<Vec<MessageRecord>> {
        let conn = self.lock_connection()?;
        let mut stmt = conn
            .prepare(
                "SELECT message_id, session_id, turn_id, message_seq, kind, content, created_at, metadata
                 FROM messages WHERE session_id = ?1 ORDER BY created_at, COALESCE(message_seq, 0)",
            )
            .map_err(sqlite_read_error)?;
        let rows = stmt
            .query_map(params![session_id.as_str()], |row| {
                let meta: Option<String> = row.get(7)?;
                Ok(MessageRecord {
                    message_id: row.get(0)?,
                    session_id: row.get(1)?,
                    turn_id: row.get(2)?,
                    message_seq: row.get::<_, i64>(3)? as u64,
                    kind: parse_message_kind(&row.get::<_, String>(4)?),
                    content: row.get(5)?,
                    created_at: row.get::<_, i64>(6)? as u128,
                    metadata: meta
                        .filter(|s| !s.is_empty())
                        .map(|s| serde_json::from_str(&s).ok())
                        .flatten(),
                })
            })
            .map_err(sqlite_read_error)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(sqlite_read_error)
    }

    fn get_messages_for_turn(&self, turn_id: &str) -> AicoreResult<Vec<MessageRecord>> {
        let conn = self.lock_connection()?;
        let mut stmt = conn
            .prepare(
                "SELECT message_id, session_id, turn_id, message_seq, kind, content, created_at, metadata
                 FROM messages WHERE turn_id = ?1 ORDER BY message_seq, created_at",
            )
            .map_err(sqlite_read_error)?;
        let rows = stmt
            .query_map(params![turn_id], |row| {
                let meta: Option<String> = row.get(7)?;
                Ok(MessageRecord {
                    message_id: row.get(0)?,
                    session_id: row.get(1)?,
                    turn_id: row.get(2)?,
                    message_seq: row.get::<_, i64>(3)? as u64,
                    kind: parse_message_kind(&row.get::<_, String>(4)?),
                    content: row.get(5)?,
                    created_at: row.get::<_, i64>(6)? as u128,
                    metadata: meta.and_then(|value| serde_json::from_str(&value).ok()),
                })
            })
            .map_err(sqlite_read_error)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(sqlite_read_error)
    }

    fn get_runtime_state(&self) -> AicoreResult<InstanceRuntimeSnapshot> {
        self.get_current_snapshot()
    }

    fn get_current_snapshot(&self) -> AicoreResult<InstanceRuntimeSnapshot> {
        let conn = self.lock_connection()?;
        let row = conn
            .query_row(
                "SELECT instance_id, active_session_id, active_turn_id, last_message_seq,
                        pending_input_id, pending_approval_id, runtime_status, lock_version,
                        dirty_shutdown, recovery_required
                 FROM instance_runtime_state
                 WHERE instance_id = ?1",
                params![self.instance_id.as_str()],
                |row| {
                    let status: String = row.get(6)?;
                    Ok(InstanceRuntimeSnapshot {
                        instance_id: row.get(0)?,
                        active_session_id: row.get(1)?,
                        active_turn_id: row.get(2)?,
                        last_message_seq: row.get::<_, Option<i64>>(3)?.map(|v| v as u64),
                        pending_input_id: row.get(4)?,
                        pending_approval_id: row.get(5)?,
                        runtime_status: parse_runtime_status(&status),
                        lock_version: row.get::<_, i64>(7)? as u64,
                        dirty_shutdown: row.get::<_, i64>(8)? != 0,
                        recovery_required: row.get::<_, i64>(9)? != 0,
                    })
                },
            )
            .map_err(sqlite_read_error)?;
        Ok(row)
    }

    fn get_pending_input(&self) -> AicoreResult<Option<PendingInputRecord>> {
        let conn = self.lock_connection()?;
        conn.query_row(
            "SELECT pending_input_id, instance_id, session_id, turn_id, content, status, created_at, updated_at
             FROM pending_inputs WHERE instance_id = ?1 AND status = 'pending'",
            params![self.instance_id.as_str()],
            |row| {
                let status: String = row.get(5)?;
                Ok(PendingInputRecord {
                    pending_input_id: row.get(0)?,
                    instance_id: row.get(1)?,
                    session_id: row.get(2)?,
                    turn_id: row.get(3)?,
                    content: row.get(4)?,
                    status: parse_pending_input_status(&status),
                    created_at: row.get::<_, i64>(6)? as u128,
                    updated_at: row.get::<_, i64>(7)? as u128,
                })
            },
        )
        .optional()
        .map_err(sqlite_read_error)
    }

    fn list_approvals_for_turn(&self, turn_id: &str) -> AicoreResult<Vec<ApprovalRecord>> {
        let conn = self.lock_connection()?;
        let mut stmt = conn
            .prepare(
                "SELECT approval_id, instance_id, turn_id, status, scope, summary, created_at, resolved_at, resolved_response_id
                 FROM approvals WHERE instance_id = ?1 AND turn_id = ?2 ORDER BY created_at",
            )
            .map_err(sqlite_read_error)?;
        let rows = stmt
            .query_map(params![self.instance_id.as_str(), turn_id], |row| {
                let status: String = row.get(3)?;
                let scope: String = row.get(4)?;
                Ok(ApprovalRecord {
                    approval_id: row.get(0)?,
                    instance_id: row.get(1)?,
                    turn_id: row.get(2)?,
                    status: parse_approval_status(&status),
                    scope: parse_approval_scope(&scope),
                    summary: row.get(5)?,
                    created_at: row.get::<_, i64>(6)? as u128,
                    resolved_at: row.get::<_, Option<i64>>(7)?.map(|value| value as u128),
                    resolved_response_id: row.get(8)?,
                })
            })
            .map_err(sqlite_read_error)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(sqlite_read_error)
    }

    fn list_approval_responses(
        &self,
        approval_id: &str,
    ) -> AicoreResult<Vec<ApprovalResponseRecord>> {
        let conn = self.lock_connection()?;
        let mut stmt = conn
            .prepare(
                "SELECT response_id, approval_id, instance_id, decision, status, responder_client_id, responder_client_kind, responded_at
                 FROM approval_responses WHERE instance_id = ?1 AND approval_id = ?2 ORDER BY responded_at",
            )
            .map_err(sqlite_read_error)?;
        let rows = stmt
            .query_map(params![self.instance_id.as_str(), approval_id], |row| {
                let decision: String = row.get(3)?;
                let status: String = row.get(4)?;
                Ok(ApprovalResponseRecord {
                    response_id: row.get(0)?,
                    approval_id: row.get(1)?,
                    instance_id: row.get(2)?,
                    decision: parse_approval_decision(&decision),
                    status: parse_approval_response_status(&status),
                    responder_client_id: row.get(5)?,
                    responder_client_kind: row.get(6)?,
                    responded_at: row.get::<_, i64>(7)? as u128,
                })
            })
            .map_err(sqlite_read_error)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(sqlite_read_error)
    }
}
