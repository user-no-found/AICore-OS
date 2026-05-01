use aicore_foundation::{AicoreResult, SessionId};
use aicore_session::traits::SessionLedgerReader;
use aicore_session::types::{
    InstanceRuntimeSnapshot, MessageRecord, SessionRecord, SessionSummary,
};
use rusqlite::{OptionalExtension, params};

use crate::error::sqlite_read_error;
use crate::store::SqliteSessionStore;
use crate::store::helpers::{parse_message_kind, parse_runtime_status, parse_session_status};

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

    fn read_messages(&self, session_id: &SessionId) -> AicoreResult<Vec<MessageRecord>> {
        let conn = self.lock_connection()?;
        let mut stmt = conn
            .prepare(
                "SELECT message_id, session_id, turn_id, message_seq, kind, content, created_at, metadata
                 FROM messages WHERE session_id = ?1 ORDER BY created_at, message_seq",
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

    fn get_current_snapshot(&self) -> AicoreResult<InstanceRuntimeSnapshot> {
        let conn = self.lock_connection()?;
        let row = conn
            .query_row(
                "SELECT instance_id, active_session_id, active_turn_id, last_message_seq,
                        runtime_status, dirty_shutdown, recovery_required
                 FROM instance_runtime_state
                 WHERE instance_id = ?1",
                params![self.instance_id.as_str()],
                |row| {
                    let status: String = row.get(4)?;
                    Ok(InstanceRuntimeSnapshot {
                        instance_id: row.get(0)?,
                        active_session_id: row.get(1)?,
                        active_turn_id: row.get(2)?,
                        last_message_seq: row.get::<_, Option<i64>>(3)?.map(|v| v as u64),
                        runtime_status: parse_runtime_status(&status),
                        dirty_shutdown: row.get::<_, i64>(5)? != 0,
                        recovery_required: row.get::<_, i64>(6)? != 0,
                    })
                },
            )
            .map_err(sqlite_read_error)?;
        Ok(row)
    }

    fn read_pending_inputs(&self) -> AicoreResult<()> {
        Err(crate::error::unsupported_api("read_pending_inputs"))
    }

    fn read_approvals(&self) -> AicoreResult<()> {
        Err(crate::error::unsupported_api("read_approvals"))
    }

    fn read_approval_responses(&self) -> AicoreResult<()> {
        Err(crate::error::unsupported_api("read_approval_responses"))
    }
}
