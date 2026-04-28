use super::*;

pub(super) fn row_to_scope(row: &SqliteRow) -> Result<MemoryScope, MemoryError> {
    let scope_kind = row.get::<String, _>("scope_kind");
    let instance_id = row.get::<String, _>("instance_id");
    let workspace_root = row.get::<Option<String>, _>("workspace_root");

    match scope_kind.as_str() {
        "global_main" => Ok(MemoryScope::GlobalMain { instance_id }),
        "workspace" => Ok(MemoryScope::Workspace {
            instance_id,
            workspace_root: workspace_root.unwrap_or_default(),
        }),
        _ => Err(MemoryError(format!("unknown scope_kind: {scope_kind}"))),
    }
}

pub(super) fn row_to_record(row: SqliteRow) -> Result<MemoryRecord, MemoryError> {
    Ok(MemoryRecord {
        memory_id: row.get("memory_id"),
        record_version: row.get("record_version"),
        memory_type: parse_memory_type(&row.get::<String, _>("memory_type"))?,
        status: parse_memory_status(&row.get::<String, _>("status"))?,
        permanence: parse_memory_permanence(&row.get::<String, _>("permanence"))?,
        scope: row_to_scope(&row)?,
        content: row.get("content"),
        content_language: row.get("content_language"),
        normalized_content: row.get("normalized_content"),
        normalized_language: row.get("normalized_language"),
        localized_summary: row.get("localized_summary"),
        source: parse_memory_source(&row.get::<String, _>("source"))?,
        evidence_json: row.get("evidence_json"),
        state_key: row.get("state_key"),
        state_version: row.get("state_version"),
        current_state: row.get("current_state"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

pub(super) fn row_to_proposal(row: SqliteRow) -> Result<MemoryProposal, MemoryError> {
    Ok(MemoryProposal {
        proposal_id: row.get("proposal_id"),
        memory_type: parse_memory_type(&row.get::<String, _>("memory_type"))?,
        scope: row_to_scope(&row)?,
        source: parse_memory_source(&row.get::<String, _>("source"))?,
        status: parse_proposal_status(&row.get::<String, _>("status"))?,
        content: row.get("content"),
        content_language: row.get("content_language"),
        normalized_content: row.get("normalized_content"),
        normalized_language: row.get("normalized_language"),
        localized_summary: row.get("localized_summary"),
        created_at: row.get("created_at"),
    })
}

pub(super) fn row_to_event(row: SqliteRow) -> Result<MemoryEvent, MemoryError> {
    Ok(MemoryEvent {
        event_id: row.get("event_id"),
        event_kind: parse_event_kind(&row.get::<String, _>("event_kind"))?,
        memory_id: row.get("memory_id"),
        proposal_id: row.get("proposal_id"),
        scope: row_to_scope(&row)?,
        actor: row.get("actor"),
        reason: row.get("reason"),
        evidence_json: row.get("evidence_json"),
        created_at: row.get("created_at"),
    })
}

pub(super) fn memory_type_name(value: &MemoryType) -> &'static str {
    match value {
        MemoryType::Core => "core",
        MemoryType::Working => "working",
        MemoryType::Status => "status",
        MemoryType::Decision => "decision",
    }
}

pub(super) fn memory_status_name(value: &MemoryStatus) -> &'static str {
    match value {
        MemoryStatus::Active => "active",
        MemoryStatus::Superseded => "superseded",
        MemoryStatus::Invalidated => "invalidated",
        MemoryStatus::Archived => "archived",
        MemoryStatus::Forgotten => "forgotten",
    }
}

pub(super) fn memory_permanence_name(value: &MemoryPermanence) -> &'static str {
    match value {
        MemoryPermanence::Standard => "standard",
        MemoryPermanence::Permanent => "permanent",
    }
}

pub(super) fn memory_source_name(value: &MemorySource) -> &'static str {
    match value {
        MemorySource::UserExplicit => "user_explicit",
        MemorySource::UserCorrection => "user_correction",
        MemorySource::AssistantSummary => "assistant_summary",
        MemorySource::RuleBasedAgent => "rule_based_agent",
    }
}

pub(super) fn proposal_status_name(value: &MemoryProposalStatus) -> &'static str {
    match value {
        MemoryProposalStatus::Open => "open",
        MemoryProposalStatus::Accepted => "accepted",
        MemoryProposalStatus::Rejected => "rejected",
    }
}

pub(super) fn event_kind_name(value: &MemoryEventKind) -> &'static str {
    match value {
        MemoryEventKind::Accepted => "accepted",
        MemoryEventKind::Proposed => "proposed",
        MemoryEventKind::Rejected => "rejected",
        MemoryEventKind::Corrected => "corrected",
        MemoryEventKind::Archived => "archived",
        MemoryEventKind::Forgotten => "forgotten",
    }
}

fn parse_memory_type(value: &str) -> Result<MemoryType, MemoryError> {
    match value {
        "core" => Ok(MemoryType::Core),
        "working" => Ok(MemoryType::Working),
        "status" => Ok(MemoryType::Status),
        "decision" => Ok(MemoryType::Decision),
        _ => Err(MemoryError(format!("unknown memory_type: {value}"))),
    }
}

fn parse_memory_status(value: &str) -> Result<MemoryStatus, MemoryError> {
    match value {
        "active" => Ok(MemoryStatus::Active),
        "superseded" => Ok(MemoryStatus::Superseded),
        "invalidated" => Ok(MemoryStatus::Invalidated),
        "archived" => Ok(MemoryStatus::Archived),
        "forgotten" => Ok(MemoryStatus::Forgotten),
        _ => Err(MemoryError(format!("unknown memory_status: {value}"))),
    }
}

fn parse_memory_permanence(value: &str) -> Result<MemoryPermanence, MemoryError> {
    match value {
        "standard" => Ok(MemoryPermanence::Standard),
        "permanent" => Ok(MemoryPermanence::Permanent),
        _ => Err(MemoryError(format!("unknown memory_permanence: {value}"))),
    }
}

fn parse_memory_source(value: &str) -> Result<MemorySource, MemoryError> {
    match value {
        "user_explicit" => Ok(MemorySource::UserExplicit),
        "user_correction" => Ok(MemorySource::UserCorrection),
        "assistant_summary" => Ok(MemorySource::AssistantSummary),
        "rule_based_agent" => Ok(MemorySource::RuleBasedAgent),
        _ => Err(MemoryError(format!("unknown memory_source: {value}"))),
    }
}

fn parse_proposal_status(value: &str) -> Result<MemoryProposalStatus, MemoryError> {
    match value {
        "open" => Ok(MemoryProposalStatus::Open),
        "accepted" => Ok(MemoryProposalStatus::Accepted),
        "rejected" => Ok(MemoryProposalStatus::Rejected),
        _ => Err(MemoryError(format!("unknown proposal_status: {value}"))),
    }
}

fn parse_event_kind(value: &str) -> Result<MemoryEventKind, MemoryError> {
    match value {
        "accepted" => Ok(MemoryEventKind::Accepted),
        "proposed" => Ok(MemoryEventKind::Proposed),
        "rejected" => Ok(MemoryEventKind::Rejected),
        "corrected" => Ok(MemoryEventKind::Corrected),
        "archived" => Ok(MemoryEventKind::Archived),
        "forgotten" => Ok(MemoryEventKind::Forgotten),
        _ => Err(MemoryError(format!("unknown event_kind: {value}"))),
    }
}
