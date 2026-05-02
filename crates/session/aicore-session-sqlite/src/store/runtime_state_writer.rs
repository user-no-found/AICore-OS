use aicore_foundation::AicoreResult;
use aicore_session::types::SetRuntimeStateRequest;
use rusqlite::params;

use crate::error::sqlite_write_error;
use crate::store::SqliteSessionStore;
use crate::store::helpers::ensure_request_instance;

impl SqliteSessionStore {
    pub(crate) fn set_runtime_state_impl(
        &self,
        request: &SetRuntimeStateRequest,
    ) -> AicoreResult<()> {
        ensure_request_instance(self.instance_id.as_str(), request.instance_id.as_str())?;
        let updated_at = request.updated_at.unix_millis() as i64;
        let mut conn = self.lock_connection()?;
        let tx = conn.transaction().map_err(sqlite_write_error)?;

        tx.execute(
            "UPDATE instance_runtime_state
             SET active_session_id = ?1, active_turn_id = ?2, pending_input_id = ?3,
                 pending_approval_id = ?4, runtime_status = ?5,
                 lock_version = COALESCE(?6, lock_version), dirty_shutdown = ?7,
                 recovery_required = ?8, updated_at = ?9
             WHERE instance_id = ?10",
            params![
                request.active_session_id.as_deref(),
                request.active_turn_id.as_deref(),
                request.pending_input_id.as_deref(),
                request.pending_approval_id.as_deref(),
                request.runtime_status.as_str(),
                request.lock_version.map(|value| value as i64),
                i64::from(request.dirty_shutdown),
                i64::from(request.recovery_required),
                updated_at,
                self.instance_id.as_str(),
            ],
        )
        .map_err(sqlite_write_error)?;

        tx.commit().map_err(sqlite_write_error)
    }
}
