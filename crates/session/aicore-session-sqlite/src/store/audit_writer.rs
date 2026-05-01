use aicore_foundation::AicoreResult;
use aicore_session::types::{AppendControlEventRequest, AppendLedgerWriteRequest};
use rusqlite::params;

use crate::error::sqlite_write_error;
use crate::store::SqliteSessionStore;
use crate::store::helpers::{ensure_request_instance, next_event_seq, next_write_seq};

impl SqliteSessionStore {
    pub(crate) fn append_control_event_impl(
        &self,
        request: &AppendControlEventRequest,
    ) -> AicoreResult<()> {
        ensure_request_instance(self.instance_id.as_str(), request.instance_id.as_str())?;
        let created_at = request.created_at.unix_millis() as i64;
        let mut conn = self.lock_connection()?;
        let tx = conn.transaction().map_err(sqlite_write_error)?;

        let event_seq = next_event_seq(&tx, self.instance_id.as_str(), request.turn_id.as_deref())?;
        tx.execute(
            "INSERT INTO control_events (event_id, instance_id, turn_id, event_seq, event_type, detail, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                request.event_id,
                self.instance_id.as_str(),
                request.turn_id.as_deref(),
                event_seq,
                request.event_kind.as_str(),
                request.detail,
                created_at,
            ],
        )
        .map_err(sqlite_write_error)?;

        tx.execute(
            "UPDATE instance_runtime_state SET last_control_event_seq = ?1, updated_at = ?2 WHERE instance_id = ?3",
            params![event_seq, created_at, self.instance_id.as_str()],
        )
        .map_err(sqlite_write_error)?;

        tx.commit().map_err(sqlite_write_error)
    }

    pub(crate) fn append_ledger_write_impl(
        &self,
        request: &AppendLedgerWriteRequest,
    ) -> AicoreResult<()> {
        ensure_request_instance(self.instance_id.as_str(), request.instance_id.as_str())?;
        let created_at = request.created_at.unix_millis() as i64;
        let mut conn = self.lock_connection()?;
        let tx = conn.transaction().map_err(sqlite_write_error)?;

        let write_seq = next_write_seq(&tx, self.instance_id.as_str(), request.turn_id.as_deref())?;
        tx.execute(
            "INSERT INTO ledger_writes (write_id, instance_id, turn_id, write_seq, write_type, target_table, target_id, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                request.write_id,
                self.instance_id.as_str(),
                request.turn_id.as_deref(),
                write_seq,
                request.write_kind.as_str(),
                request.target_table,
                request.target_id,
                created_at,
            ],
        )
        .map_err(sqlite_write_error)?;

        tx.execute(
            "UPDATE instance_runtime_state SET last_write_seq = ?1, updated_at = ?2 WHERE instance_id = ?3",
            params![write_seq, created_at, self.instance_id.as_str()],
        )
        .map_err(sqlite_write_error)?;

        tx.commit().map_err(sqlite_write_error)
    }
}
