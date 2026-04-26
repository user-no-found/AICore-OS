use std::{
    fs::{self, OpenOptions},
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
    thread,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::types::MemoryError;

const STALE_LOCK_TIMEOUT_SECS: u64 = 30;
const LOCK_WAIT_TIMEOUT_MILLIS: u64 = 1500;
const LOCK_WAIT_POLL_MILLIS: u64 = 10;

#[derive(Debug)]
pub struct MemoryWriteGuard {
    lock_path: PathBuf,
}

impl MemoryWriteGuard {
    pub fn acquire(lock_path: &Path, operation: &str) -> Result<Self, MemoryError> {
        let started_at = now_millis();

        loop {
            match try_create_lock(lock_path, operation) {
                Ok(()) => {
                    return Ok(Self {
                        lock_path: lock_path.to_path_buf(),
                    });
                }
                Err(error) if error.kind() == ErrorKind::AlreadyExists => {
                    if is_stale_lock(lock_path)? {
                        let _ = fs::remove_file(lock_path);
                        continue;
                    }

                    if now_millis().saturating_sub(started_at) >= LOCK_WAIT_TIMEOUT_MILLIS {
                        return Err(MemoryError(format!(
                            "memory write locked: {}",
                            lock_path.display()
                        )));
                    }

                    thread::sleep(std::time::Duration::from_millis(LOCK_WAIT_POLL_MILLIS));
                }
                Err(error) => return Err(MemoryError(error.to_string())),
            }
        }
    }
}

impl Drop for MemoryWriteGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.lock_path);
    }
}

fn try_create_lock(lock_path: &Path, operation: &str) -> Result<(), std::io::Error> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(lock_path)?;
    writeln!(file, "pid={}", std::process::id())?;
    writeln!(file, "created_at={}", now_secs())?;
    writeln!(file, "operation={operation}")?;
    Ok(())
}

fn is_stale_lock(lock_path: &Path) -> Result<bool, MemoryError> {
    let content = fs::read_to_string(lock_path).map_err(|error| MemoryError(error.to_string()))?;
    let created_at = content
        .lines()
        .find_map(|line| line.strip_prefix("created_at="))
        .and_then(|value| value.parse::<u64>().ok());

    match created_at {
        Some(created_at) => Ok(now_secs().saturating_sub(created_at) > STALE_LOCK_TIMEOUT_SECS),
        None => Ok(false),
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time should move forward")
        .as_secs()
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time should move forward")
        .as_millis() as u64
}
