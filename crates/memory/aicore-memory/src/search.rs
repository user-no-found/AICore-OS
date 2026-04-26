use std::cmp::Reverse;

use crate::types::{
    MemoryPermanence, MemoryRecord, MemoryScope, MemoryStatus, MemoryType, SearchQuery,
};

pub fn filter_records(records: &[MemoryRecord], query: &SearchQuery) -> Vec<MemoryRecord> {
    let needle = query.text.to_ascii_lowercase();

    records
        .iter()
        .filter(|record| record.status == MemoryStatus::Active)
        .filter(|record| match &query.scope {
            Some(scope) => &record.scope == scope,
            None => true,
        })
        .filter(|record| {
            record.content.to_ascii_lowercase().contains(&needle)
                || record.normalized_content.to_ascii_lowercase().contains(&needle)
                || record.localized_summary.to_ascii_lowercase().contains(&needle)
        })
        .cloned()
        .collect()
}

pub fn build_memory_pack_for_tests(records: &[MemoryRecord], token_budget: usize) -> Vec<MemoryRecord> {
    let mut candidates: Vec<MemoryRecord> = records
        .iter()
        .filter(|record| record.status == MemoryStatus::Active)
        .cloned()
        .collect();

    candidates.sort_by_key(|record| {
        (
            Reverse(matches!(record.permanence, MemoryPermanence::Permanent)),
            Reverse(matches!(record.memory_type, MemoryType::Core)),
            record.created_at.clone(),
        )
    });

    let mut used = 0usize;
    let mut packed = Vec::new();

    for record in candidates {
        let cost = estimate_tokens(&record);
        if used + cost > token_budget {
            continue;
        }

        used += cost;
        packed.push(record);
    }

    packed
}

fn estimate_tokens(record: &MemoryRecord) -> usize {
    let summary = record.localized_summary.trim();
    if !summary.is_empty() {
        summary.chars().count()
    } else {
        record.content.chars().count()
    }
}

pub fn scope_kind(scope: &MemoryScope) -> &'static str {
    match scope {
        MemoryScope::GlobalMain { .. } => "global_main",
        MemoryScope::Workspace { .. } => "workspace",
    }
}

pub fn instance_id(scope: &MemoryScope) -> &str {
    match scope {
        MemoryScope::GlobalMain { instance_id } => instance_id,
        MemoryScope::Workspace { instance_id, .. } => instance_id,
    }
}

pub fn workspace_root(scope: &MemoryScope) -> Option<&str> {
    match scope {
        MemoryScope::GlobalMain { .. } => None,
        MemoryScope::Workspace { workspace_root, .. } => Some(workspace_root),
    }
}
