use std::{fs, path::Path};

use crate::{
    safety::blocks_secret,
    types::{MemoryRecord, MemoryStatus, MemoryType, ProjectionState},
};

struct WikiProjectionMetadata<'a> {
    generated_at: &'a str,
    last_rebuild_at: &'a str,
    projection_stale: bool,
    projection_warning: Option<&'a str>,
}

pub fn rebuild_projections(
    core_path: &Path,
    status_path: &Path,
    permanent_path: &Path,
    decisions_path: &Path,
    wiki_index_path: &Path,
    wiki_core_path: &Path,
    wiki_decisions_path: &Path,
    wiki_status_path: &Path,
    records: &[MemoryRecord],
    generated_at: &str,
    should_fail: bool,
) -> Result<
    (
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
    ),
    String,
> {
    if should_fail {
        return Err("projection failure injected for tests".to_string());
    }

    let metadata = WikiProjectionMetadata {
        generated_at,
        last_rebuild_at: generated_at,
        projection_stale: false,
        projection_warning: None,
    };
    let core = build_core_projection(records);
    let status = build_status_projection(records);
    let permanent = build_permanent_projection(records);
    let decisions = build_decisions_projection(records);
    let wiki_index = build_wiki_index_projection(&metadata);
    let wiki_core = build_wiki_page_projection("Core Memories", &metadata, records, |record| {
        record.memory_type == MemoryType::Core
    });
    let wiki_decisions = build_wiki_page_projection("Decisions", &metadata, records, |record| {
        record.memory_type == MemoryType::Decision
    });
    let wiki_status = build_wiki_page_projection("Status", &metadata, records, |record| {
        record.memory_type == MemoryType::Status
    });

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
    if let Some(parent) = wiki_index_path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    if let Some(parent) = wiki_core_path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    if let Some(parent) = wiki_decisions_path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    if let Some(parent) = wiki_status_path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }

    write_atomic(core_path, &core)?;
    write_atomic(status_path, &status)?;
    write_atomic(permanent_path, &permanent)?;
    write_atomic(decisions_path, &decisions)?;
    write_atomic(wiki_index_path, &wiki_index)?;
    write_atomic(wiki_core_path, &wiki_core)?;
    write_atomic(wiki_decisions_path, &wiki_decisions)?;
    write_atomic(wiki_status_path, &wiki_status)?;

    Ok((
        core,
        status,
        permanent,
        decisions,
        wiki_index,
        wiki_core,
        wiki_decisions,
        wiki_status,
    ))
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

fn build_wiki_index_projection(metadata: &WikiProjectionMetadata<'_>) -> String {
    format!(
        "# Memory Wiki\n\n> 这是 generated projection。\n> 不是事实来源。\n> 事实来源仍然是 memory.db / MemoryRecord / Memory Event Ledger。\n> 不应手工编辑后期待反向同步。\n\n- generated_at: {}\n- last_rebuild_at: {}\n- projection_stale: {}\n- projection_warning: {}\n- source: memory_records\n- truth_source: memory.db / MemoryRecord / Memory Event Ledger\n\n- [Core](core.md): 当前 active core 记忆列表。\n- [Decisions](decisions.md): 当前 active decision 记忆列表。\n- [Status](status.md): 当前 active status 记忆列表。\n",
        metadata.generated_at,
        metadata.last_rebuild_at,
        metadata.projection_stale,
        metadata.projection_warning.unwrap_or("<none>"),
    )
}

fn build_wiki_page_projection(
    title: &str,
    metadata: &WikiProjectionMetadata<'_>,
    records: &[MemoryRecord],
    predicate: impl Fn(&MemoryRecord) -> bool,
) -> String {
    let mut output = format!(
        "# {title}\n\n> 这是 generated projection。\n> 不是事实来源。\n> 事实来源仍然是 memory.db / MemoryRecord / Memory Event Ledger。\n> 不应手工编辑后期待反向同步。\n\n- generated_at: {}\n- last_rebuild_at: {}\n- projection_stale: {}\n- projection_warning: {}\n- source: memory_records\n- truth_source: memory.db / MemoryRecord / Memory Event Ledger\n\n",
        metadata.generated_at,
        metadata.last_rebuild_at,
        metadata.projection_stale,
        metadata.projection_warning.unwrap_or("<none>"),
    );

    let mut found = false;
    for record in records.iter().filter(|record| {
        record.status == MemoryStatus::Active
            && predicate(record)
            && !blocks_secret(&record.content)
    }) {
        found = true;
        output.push_str(&format!(
            "- memory_id: {}\n  - memory_type: {}\n  - source: {}\n  - permanence: {}\n  - scope: {}\n  - record_version: {}\n  - updated_at: {}\n",
            record.memory_id,
            memory_type_name(&record.memory_type),
            memory_source_name(&record.source),
            memory_permanence_name(&record.permanence),
            memory_scope_name(&record.scope),
            record.record_version,
            record.updated_at,
        ));
        if record.memory_type == MemoryType::Status {
            if let Some(state_key) = &record.state_key {
                output.push_str(&format!("  - state_key: {state_key}\n"));
            }
            if let Some(current_state) = &record.current_state {
                output.push_str(&format!("  - current_state: {current_state}\n"));
            }
        }
        output.push_str(&format!("  - content: {}\n", record.localized_summary));
    }

    if !found {
        output.push_str("- <empty>\n");
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

fn write_atomic(path: &Path, content: &str) -> Result<(), String> {
    let file_name = path
        .file_name()
        .ok_or_else(|| format!("invalid projection path: {}", path.display()))?;
    let temp_path = path.with_file_name(format!("{}.tmp", file_name.to_string_lossy()));
    fs::write(&temp_path, content).map_err(|error| error.to_string())?;
    fs::rename(&temp_path, path).map_err(|error| {
        let _ = fs::remove_file(&temp_path);
        error.to_string()
    })?;
    Ok(())
}

fn memory_type_name(memory_type: &MemoryType) -> &'static str {
    match memory_type {
        MemoryType::Core => "core",
        MemoryType::Working => "working",
        MemoryType::Status => "status",
        MemoryType::Decision => "decision",
    }
}

fn memory_source_name(source: &crate::types::MemorySource) -> &'static str {
    match source {
        crate::types::MemorySource::UserExplicit => "user_explicit",
        crate::types::MemorySource::UserCorrection => "user_correction",
        crate::types::MemorySource::AssistantSummary => "assistant_summary",
        crate::types::MemorySource::RuleBasedAgent => "rule_based_agent",
    }
}

fn memory_permanence_name(permanence: &crate::types::MemoryPermanence) -> &'static str {
    match permanence {
        crate::types::MemoryPermanence::Standard => "standard",
        crate::types::MemoryPermanence::Permanent => "permanent",
    }
}

fn memory_scope_name(scope: &crate::types::MemoryScope) -> String {
    match scope {
        crate::types::MemoryScope::GlobalMain { instance_id } => {
            format!("global-main:{instance_id}")
        }
        crate::types::MemoryScope::Workspace {
            instance_id,
            workspace_root,
        } => {
            format!("workspace:{instance_id}:{workspace_root}")
        }
    }
}
