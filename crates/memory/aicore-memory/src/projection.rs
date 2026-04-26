use std::{fs, path::Path};

use crate::{
    safety::blocks_secret,
    types::{MemoryRecord, MemoryStatus, MemoryType, ProjectionState},
};

pub fn rebuild_projections(
    core_path: &Path,
    status_path: &Path,
    records: &[MemoryRecord],
    should_fail: bool,
) -> Result<(String, String), String> {
    if should_fail {
        return Err("projection failure injected for tests".to_string());
    }

    let core = build_core_projection(records);
    let status = build_status_projection(records);

    if let Some(parent) = core_path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    if let Some(parent) = status_path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }

    fs::write(core_path, &core).map_err(|error| error.to_string())?;
    fs::write(status_path, &status).map_err(|error| error.to_string())?;

    Ok((core, status))
}

pub fn build_core_projection(records: &[MemoryRecord]) -> String {
    let mut output = String::from("# CORE\n\n");

    for record in records.iter().filter(|record| {
        record.status == MemoryStatus::Active
            && record.memory_type == MemoryType::Core
            && !blocks_secret(&record.content)
    }) {
        output.push_str(&format!("- {}\n", record.content));
    }

    output
}

pub fn build_status_projection(records: &[MemoryRecord]) -> String {
    let mut output = String::from("# STATUS\n\n");

    for record in records.iter().filter(|record| {
        record.status == MemoryStatus::Active && record.memory_type == MemoryType::Status
    }) {
        if let Some(state) = &record.current_state {
            output.push_str(&format!(
                "- {}: {}\n",
                record.state_key.as_deref().unwrap_or("state"),
                state
            ));
        }
    }

    output
}

pub fn projection_state(stale: bool, warning: Option<String>) -> ProjectionState {
    ProjectionState { stale, warning }
}
