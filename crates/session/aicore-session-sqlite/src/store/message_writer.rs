use aicore_foundation::{AicoreClock, AicoreResult, SystemClock};
use aicore_session::types::{AppendMessageRequest, LedgerWriteKind};
use rusqlite::params;

use crate::error::{sqlite_schema_error, sqlite_write_error};
use crate::store::SqliteSessionStore;
use crate::store::helpers::{ensure_request_instance, next_write_seq, uuidv7_str};

impl SqliteSessionStore {
    pub(crate) fn append_message_impl(&self, request: &AppendMessageRequest) -> AicoreResult<()> {
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

        if let Some(ref turn_id) = request.turn_id {
            let turn_exists: i64 = tx
                .query_row(
                    "SELECT COUNT(*) FROM turns WHERE turn_id = ?1",
                    params![turn_id],
                    |row| row.get(0),
                )
                .map_err(sqlite_schema_error)?;
            if turn_exists == 0 {
                return Err(aicore_foundation::AicoreError::Missing(format!(
                    "turn not found: {turn_id}"
                )));
            }
        }

        let metadata_json = request
            .metadata
            .as_ref()
            .map(|v| v.to_string())
            .unwrap_or_default();
        tx.execute(
            "INSERT INTO messages (message_id, session_id, turn_id, message_seq, kind, content, created_at, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                request.message_id,
                request.session_id.as_str(),
                request.turn_id.as_deref(),
                request.message_seq as i64,
                request.kind.as_str(),
                request.content,
                now,
                if metadata_json.is_empty() { None } else { Some(&metadata_json) },
            ],
        )
        .map_err(sqlite_write_error)?;

        tx.execute(
            "UPDATE instance_runtime_state SET last_message_seq = ?1, updated_at = ?2 WHERE instance_id = ?3",
            params![request.message_seq as i64, now, self.instance_id.as_str()],
        )
        .map_err(sqlite_write_error)?;

        tx.execute(
            "UPDATE sessions SET updated_at = ?1 WHERE session_id = ?2",
            params![now, request.session_id.as_str()],
        )
        .map_err(sqlite_write_error)?;

        let write_seq = next_write_seq(&tx, self.instance_id.as_str(), request.turn_id.as_deref())?;
        let write_id = uuidv7_str();
        tx.execute(
            "INSERT INTO ledger_writes (write_id, instance_id, turn_id, write_seq, write_type, target_table, target_id, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                &write_id,
                self.instance_id.as_str(),
                request.turn_id.as_deref(),
                write_seq,
                LedgerWriteKind::Insert.as_str(),
                "messages",
                request.message_id,
                now,
            ],
        )
        .map_err(sqlite_write_error)?;

        tx.commit().map_err(sqlite_write_error)
    }
}
