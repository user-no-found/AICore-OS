use std::{fs, path::Path};

use crate::{
    safety::blocks_secret,
    types::{MemoryRecord, MemoryStatus, MemoryType, ProjectionState},
};

pub fn rebuild_projections(
    core_path: &Path,
    status_path: &Path,
    permanent_path: &Path,
    decisions_path: &Path,
    records: &[MemoryRecord],
    should_fail: bool,
) -> Result<(String, String, String, String), String> {
    if should_fail {
        return Err("projection failure injected for tests".to_string());
    }

    let core = build_core_projection(records);
    let status = build_status_projection(records);
    let permanent = build_permanent_projection(records);
    let decisions = build_decisions_projection(records);

    if let Some(parent) = core_path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    if let Some(parent) = status_path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    if let Some(parent) = permanent_path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    if let Some(parent) = decisions_path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }

    fs::write(core_path, &core).map_err(|error| error.to_string())?;
    fs::write(status_path, &status).map_err(|error| error.to_string())?;
    fs::write(permanent_path, &permanent).map_err(|error| error.to_string())?;
    fs::write(decisions_path, &decisions).map_err(|error| error.to_string())?;

    Ok((core, status, permanent, decisions))
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

pub fn build_permanent_projection(records: &[MemoryRecord]) -> String {
    let mut output = String::from("# PERMANENT\n\n");

    for record in records.iter().filter(|record| {
        record.status == MemoryStatus::Active
            && matches!(record.permanence, crate::types::MemoryPermanence::Permanent)
            && !blocks_secret(&record.content)
    }) {
        output.push_str(&format!("- {}\n", record.content));
    }

    output
}

pub fn build_decisions_projection(records: &[MemoryRecord]) -> String {
    let mut output = String::from("# DECISIONS\n\n");

    for record in records.iter().filter(|record| {
        record.status == MemoryStatus::Active
            && record.memory_type == MemoryType::Decision
            && !blocks_secret(&record.content)
    }) {
        output.push_str(&format!("- {}\n", record.content));
    }

    output
}

pub fn projection_state(
    stale: bool,
    warning: Option<String>,
    last_rebuild_at: Option<String>,
) -> ProjectionState {
    ProjectionState {
        stale,
        warning,
        last_rebuild_at,
    }
}
