mod non_goals;
mod ordering;
mod recovery;
mod schema;
mod transactions;

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use aicore_foundation::InstanceId;
use aicore_session::traits::SessionLedger;

use crate::SqliteSessionStore;

fn temp_db_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time should be after epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("aicore-session-sqlite-{name}-{nanos}"));
    std::fs::create_dir_all(&dir).expect("temp dir should create");
    dir.join("sessions.sqlite")
}

fn open_store(path: &std::path::Path) -> SqliteSessionStore {
    SqliteSessionStore::open(path, &InstanceId::global_main()).expect("store should open")
}
