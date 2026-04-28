use crate::commands::memory::report::{
    build_memory_audit_report, build_memory_proposals_report, build_memory_search_report,
    build_memory_status_report, build_memory_wiki_page_report, build_memory_wiki_report,
};
use crate::commands::memory::search::{
    MemorySearchOptions, parse_memory_permanence_filter, parse_memory_source_filter,
    parse_memory_type_filter,
};

use super::component_stdio::{
    payload_string, run_component_report_stdio, run_component_report_stdio_with_request,
};

pub(crate) fn run_component_memory_status_stdio() -> i32 {
    run_component_report_stdio(
        "memory.status",
        "memory status component stdin 读取失败",
        build_memory_status_report,
    )
}

pub(crate) fn run_component_memory_search_stdio() -> i32 {
    run_component_report_stdio_with_request(
        "memory.search",
        "memory search component stdin 读取失败",
        |request| {
            let query = payload_string(request, "query", "");
            let options = memory_search_options_from_payload(request)?;
            build_memory_search_report(&query, options)
        },
    )
}

pub(crate) fn run_component_memory_proposals_stdio() -> i32 {
    run_component_report_stdio(
        "memory.proposals",
        "memory proposals component stdin 读取失败",
        build_memory_proposals_report,
    )
}

pub(crate) fn run_component_memory_audit_stdio() -> i32 {
    run_component_report_stdio(
        "memory.audit",
        "memory audit component stdin 读取失败",
        build_memory_audit_report,
    )
}

pub(crate) fn run_component_memory_wiki_stdio() -> i32 {
    run_component_report_stdio(
        "memory.wiki",
        "memory wiki component stdin 读取失败",
        build_memory_wiki_report,
    )
}

pub(crate) fn run_component_memory_wiki_page_stdio() -> i32 {
    run_component_report_stdio_with_request(
        "memory.wiki_page",
        "memory wiki page component stdin 读取失败",
        |request| {
            let page = payload_string(request, "page", "index");
            build_memory_wiki_page_report(&page)
        },
    )
}

fn memory_search_options_from_payload(
    request: &serde_json::Value,
) -> Result<MemorySearchOptions, String> {
    let payload = request.get("payload").unwrap_or(&serde_json::Value::Null);
    if let Some(invalid_filter) = optional_payload_string(payload, "invalid_filter") {
        return Err(format!("不支持的 memory.search filter：{invalid_filter}"));
    }
    let memory_type = optional_payload_string(payload, "type")
        .map(|value| parse_memory_type_filter(&value))
        .transpose()?;
    let source = optional_payload_string(payload, "source")
        .map(|value| parse_memory_source_filter(&value))
        .transpose()?;
    let permanence = optional_payload_string(payload, "permanence")
        .map(|value| parse_memory_permanence_filter(&value))
        .transpose()?;
    let limit = payload.get("limit").map(parse_limit).transpose()?;
    Ok(MemorySearchOptions {
        memory_type,
        source,
        permanence,
        limit,
    })
}

fn optional_payload_string(payload: &serde_json::Value, key: &str) -> Option<String> {
    payload
        .get(key)
        .and_then(|value| value.as_str())
        .filter(|value| !value.trim().is_empty())
        .map(ToString::to_string)
}

fn parse_limit(value: &serde_json::Value) -> Result<usize, String> {
    let limit = if let Some(number) = value.as_u64() {
        usize::try_from(number).map_err(|_| "--limit 必须是正整数。".to_string())?
    } else if let Some(text) = value.as_str() {
        text.parse::<usize>()
            .map_err(|_| "--limit 必须是正整数。".to_string())?
    } else {
        return Err("--limit 必须是正整数。".to_string());
    };
    if limit == 0 {
        return Err("--limit 必须是正整数。".to_string());
    }
    Ok(limit)
}
