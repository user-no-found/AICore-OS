mod non_goals;
mod ordering;
mod recovery;
mod schema;
mod transactions;

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::SqliteSessionStore;
use aicore_foundation::InstanceId;

const FORBIDDEN_FIELDS: &[&str] = &[
    "raw_provider_request",
    "raw_provider_response",
    "raw_tool_input",
    "raw_tool_output",
    "raw_stdout",
    "raw_stderr",
    "raw_memory_content",
    "raw_prompt",
    "secret",
    "token",
    "api_key",
    "cookie",
    "credential",
    "authorization",
    "password",
];

struct TestStorePath {
    root: PathBuf,
    db_path: PathBuf,
}

impl TestStorePath {
    fn db_path(&self) -> &Path {
        &self.db_path
    }
}

impl Drop for TestStorePath {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn temp_store_path(name: &str) -> TestStorePath {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time should be after epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("aicore-session-sqlite-{name}-{nanos}"));
    std::fs::create_dir_all(&dir).expect("temp dir should create");
    TestStorePath {
        db_path: dir.join("sessions.sqlite"),
        root: dir,
    }
}

fn open_store(path: &Path) -> SqliteSessionStore {
    SqliteSessionStore::open(path, &InstanceId::global_main()).expect("store should open")
}
