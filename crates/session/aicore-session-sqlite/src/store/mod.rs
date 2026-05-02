use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use aicore_foundation::{AicoreClock, AicoreResult, InstanceId, SystemClock};
use aicore_session::traits::{SessionLedger, SessionLedgerReader, SessionLedgerWriter};
use rusqlite::Connection;

use crate::schema;

pub mod active_turn_writer;
pub mod approval_writer;
pub mod audit_writer;
pub mod control_helpers;
pub mod helpers;
pub mod message_writer;
pub mod pending_input_writer;
pub mod reader;
pub mod runtime_state_writer;
pub mod session_turn_writer;
pub mod stop_writer;
pub mod writer;

pub struct SqliteSessionStore {
    _path: PathBuf,
    instance_id: InstanceId,
    connection: Mutex<Connection>,
}

pub type SqliteSessionLedger = SqliteSessionStore;

impl SqliteSessionStore {
    pub fn open(path: impl AsRef<Path>, instance_id: &InstanceId) -> AicoreResult<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| {
                aicore_foundation::AicoreError::InvalidPath(format!(
                    "failed to create session sqlite parent directory: {error}"
                ))
            })?;
        }

        let conn = schema::open_connection(&path)?;
        schema::initialize_or_validate(&conn, instance_id, SystemClock.now())?;

        Ok(Self {
            _path: path,
            instance_id: instance_id.clone(),
            connection: Mutex::new(conn),
        })
    }

    pub fn open_or_init(path: impl AsRef<Path>, instance_id: &InstanceId) -> AicoreResult<Self> {
        Self::open(path, instance_id)
    }

    pub(crate) fn lock_connection(&self) -> AicoreResult<MutexGuard<'_, Connection>> {
        self.connection.lock().map_err(|_| {
            aicore_foundation::AicoreError::Unavailable("session sqlite mutex poisoned".to_string())
        })
    }
}

impl SessionLedger for SqliteSessionStore {
    fn instance_id(&self) -> &InstanceId {
        &self.instance_id
    }

    fn writer(&self) -> &dyn SessionLedgerWriter {
        self
    }

    fn reader(&self) -> &dyn SessionLedgerReader {
        self
    }
}
