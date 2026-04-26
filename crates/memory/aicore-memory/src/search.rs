use crate::types::{
    MemoryPermanence, MemoryRecord, MemoryScope, MemoryStatus, MemoryType, SearchQuery,
    SearchResult,
};

pub fn filter_records(records: &[MemoryRecord], query: &SearchQuery) -> Vec<SearchResult> {
    let needle = query.text.to_ascii_lowercase();
    let mut results: Vec<SearchResult> = records
        .iter()
        .filter(|record| record.status == MemoryStatus::Active)
        .filter(|record| match &query.scope {
            Some(scope) => &record.scope == scope,
            None => true,
        })
        .filter(|record| match &query.memory_type {
            Some(memory_type) => &record.memory_type == memory_type,
            None => true,
        })
        .filter(|record| match &query.source {
            Some(source) => &record.source == source,
            None => true,
        })
        .filter(|record| match &query.permanence {
            Some(permanence) => &record.permanence == permanence,
            None => true,
        })
        .filter_map(|record| build_search_result(record, &needle))
        .collect();

    results.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| record_created_at(&right.record).cmp(&record_created_at(&left.record)))
            .then_with(|| left.record.memory_id.cmp(&right.record.memory_id))
    });

    if let Some(limit) = query.limit {
        results.truncate(limit);
    }

    results
}

pub fn build_memory_pack_for_tests(
    records: &[MemoryRecord],
    token_budget: usize,
) -> Vec<MemoryRecord> {
    let candidates = filter_records(
        records,
        &SearchQuery {
            text: String::new(),
            scope: None,
            memory_type: None,
            source: None,
            permanence: None,
            limit: None,
        },
    );

    let mut used = 0usize;
    let mut packed = Vec::new();

    for result in candidates {
        let record = result.record;
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

fn build_search_result(record: &MemoryRecord, needle: &str) -> Option<SearchResult> {
    let mut matched_fields = Vec::new();
    let mut score = 0i64;

    if needle.is_empty() {
        score += 1;
    } else {
        if record
            .localized_summary
            .to_ascii_lowercase()
            .contains(needle)
        {
            matched_fields.push("localized_summary".to_string());
            score += 400;
        }
        if record.content.to_ascii_lowercase().contains(needle) {
            matched_fields.push("content".to_string());
            score += 200;
        }
        if record
            .normalized_content
            .to_ascii_lowercase()
            .contains(needle)
        {
            matched_fields.push("normalized_content".to_string());
            score += 100;
        }

        if matched_fields.is_empty() {
            return None;
        }
    }

    if matches!(record.permanence, MemoryPermanence::Permanent) {
        score += 40;
    }
    if matches!(record.memory_type, MemoryType::Core) {
        score += 30;
    }
    if matches!(record.memory_type, MemoryType::Decision) {
        score += 20;
    }

    Some(SearchResult {
        record: record.clone(),
        score,
        matched_fields,
    })
}

fn record_created_at(record: &MemoryRecord) -> i64 {
    record.created_at.parse::<i64>().unwrap_or_default()
}
