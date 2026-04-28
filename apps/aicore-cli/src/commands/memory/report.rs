use std::fs;

use aicore_memory::{MemoryAuditReport, MemoryProposalStatus, SearchQuery};

use crate::config_store::{global_main_memory_scope, real_memory_kernel, real_memory_paths};
use crate::errors::memory_error;
use crate::names::{memory_permanence_name, memory_source_name, memory_type_name};

use super::search::MemorySearchOptions;
use super::wiki::{resolve_memory_wiki_page, wiki_projection_status_lines};

pub(crate) fn build_memory_status_report() -> Result<(String, serde_json::Value), String> {
    let paths = real_memory_paths()?;
    let kernel = aicore_memory::MemoryKernel::open(paths.clone()).map_err(memory_error)?;
    let projection = kernel.projection_state();
    Ok((
        format!("Memory status 读取完成：{} 条记录", kernel.records().len()),
        serde_json::json!({
            "operation": "memory.status",
            "scope": "global-main",
            "record_count": kernel.records().len().to_string(),
            "proposal_count": kernel.proposals().len().to_string(),
            "event_count": kernel.events().len().to_string(),
            "wiki_pages": "index,core,decisions,status",
            "db_path": paths.db_path.display().to_string(),
            "memory_root": paths.root.display().to_string(),
            "projection_status": if projection.stale { "stale" } else { "fresh" },
            "projection_stale": projection.stale.to_string(),
            "projection_warning": projection.warning.as_deref().unwrap_or("<none>"),
            "last_rebuild_at": projection.last_rebuild_at.as_deref().unwrap_or("<none>"),
            "kernel_invocation_path": "binary"
        }),
    ))
}

pub(crate) fn build_memory_search_report(
    query: &str,
    options: MemorySearchOptions,
) -> Result<(String, serde_json::Value), String> {
    let kernel = real_memory_kernel()?;
    let results = kernel
        .search(SearchQuery {
            text: query.to_string(),
            scope: Some(global_main_memory_scope()),
            memory_type: options.memory_type.clone(),
            source: options.source.clone(),
            permanence: options.permanence.clone(),
            limit: options.limit,
        })
        .map_err(memory_error)?;
    let result_values = results
        .into_iter()
        .map(|result| {
            let record = result.record;
            serde_json::json!({
                "memory_id": record.memory_id,
                "memory_type": memory_type_name(&record.memory_type),
                "source": memory_source_name(&record.source),
                "permanence": memory_permanence_name(&record.permanence),
                "score": result.score,
                "matched_fields": result.matched_fields,
                "content": record.content,
                "localized_summary": record.localized_summary
            })
        })
        .collect::<Vec<_>>();
    Ok((
        format!("记忆搜索完成：{} 条结果", result_values.len()),
        serde_json::json!({
            "operation": "memory.search",
            "query": query,
            "filters": options.to_json(),
            "result_count": result_values.len().to_string(),
            "results": serde_json::to_string(&result_values).expect("memory results should encode"),
            "kernel_invocation_path": "binary"
        }),
    ))
}

pub(crate) fn build_memory_proposals_report() -> Result<(String, serde_json::Value), String> {
    let kernel = real_memory_kernel()?;
    let proposals = kernel
        .list_open_proposals()
        .into_iter()
        .map(|proposal| {
            serde_json::json!({
                "proposal_id": proposal.proposal_id,
                "memory_type": memory_type_name(&proposal.memory_type),
                "source": memory_source_name(&proposal.source),
                "status": proposal_status_name(&proposal.status),
                "content": display_proposal_content(&proposal),
                "localized_summary": proposal.localized_summary
            })
        })
        .collect::<Vec<_>>();
    Ok((
        format!("记忆提案读取完成：{} 条 open proposal", proposals.len()),
        serde_json::json!({
            "operation": "memory.proposals",
            "proposal_count": proposals.len().to_string(),
            "proposals": serde_json::to_string(&proposals).expect("memory proposals should encode"),
            "kernel_invocation_path": "binary"
        }),
    ))
}

pub(crate) fn build_memory_audit_report() -> Result<(String, serde_json::Value), String> {
    let kernel = real_memory_kernel()?;
    let checked_records = kernel.records().len();
    let report = kernel.verify_ledger_consistency();
    Ok((
        format!(
            "Memory audit 完成：{}",
            if report.ok { "ok" } else { "failed" }
        ),
        memory_audit_fields(&report, checked_records),
    ))
}

pub(crate) fn build_memory_wiki_report() -> Result<(String, serde_json::Value), String> {
    let paths = real_memory_paths()?;
    let kernel = aicore_memory::MemoryKernel::open(paths).map_err(memory_error)?;
    let projection = kernel.projection_state();
    Ok((
        "Memory wiki index 读取完成".to_string(),
        serde_json::json!({
            "operation": "memory.wiki",
            "pages": "index,core,decisions,status",
            "not_truth_source_notice": not_truth_source_notice(),
            "stale": projection.stale.to_string(),
            "warnings": projection.warning.as_deref().unwrap_or("<none>"),
            "kernel_invocation_path": "binary"
        }),
    ))
}

pub(crate) fn build_memory_wiki_page_report(
    page: &str,
) -> Result<(String, serde_json::Value), String> {
    let paths = real_memory_paths()?;
    let kernel = aicore_memory::MemoryKernel::open(paths.clone()).map_err(memory_error)?;
    let (page_name, page_path) = resolve_memory_wiki_page(&paths, page)?;
    if !page_path.exists() {
        return Err("缺少 Wiki Projection，请先写入记忆或重建 projection。".to_string());
    }
    let markdown = fs::read_to_string(&page_path)
        .map_err(|error| format!("无法读取 Wiki Projection {}: {error}", page_path.display()))?;
    let projection = kernel.projection_state();
    Ok((
        format!("Memory wiki page 读取完成：{page_name}"),
        serde_json::json!({
            "operation": "memory.wiki_page",
            "page": page_name,
            "markdown": markdown,
            "not_truth_source_notice": not_truth_source_notice(),
            "stale": projection.stale.to_string(),
            "warnings": wiki_projection_status_lines(projection).join("; "),
            "path": page_path.display().to_string(),
            "kernel_invocation_path": "binary"
        }),
    ))
}

fn memory_audit_fields(report: &MemoryAuditReport, checked_records: usize) -> serde_json::Value {
    serde_json::json!({
        "operation": "memory.audit",
        "ok": report.ok.to_string(),
        "checked_records": checked_records.to_string(),
        "checked_events": report.checked_events.to_string(),
        "errors": serde_json::to_string(&report.issues).expect("audit issues should encode"),
        "warnings": "[]",
        "kernel_invocation_path": "binary"
    })
}

fn display_proposal_content(proposal: &aicore_memory::MemoryProposal) -> String {
    if !proposal.localized_summary.is_empty() {
        proposal.localized_summary.clone()
    } else if !proposal.content.is_empty() {
        proposal.content.clone()
    } else {
        proposal.normalized_content.clone()
    }
}

fn proposal_status_name(status: &MemoryProposalStatus) -> &'static str {
    match status {
        MemoryProposalStatus::Open => "open",
        MemoryProposalStatus::Accepted => "accepted",
        MemoryProposalStatus::Rejected => "rejected",
    }
}

fn not_truth_source_notice() -> &'static str {
    "这是 generated projection，不是事实来源，不应手工编辑后期待反向同步"
}
