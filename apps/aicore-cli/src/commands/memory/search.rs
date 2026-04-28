use aicore_memory::{MemoryPermanence, MemorySource, MemoryType, SearchQuery};

use crate::config_store::{global_main_memory_scope, real_memory_kernel};
use crate::errors::memory_error;
use crate::names::{memory_permanence_name, memory_source_name, memory_type_name};
use crate::terminal::emit_cli_panel_body;

#[derive(Debug, Default)]
pub(crate) struct MemorySearchOptions {
    pub(crate) memory_type: Option<MemoryType>,
    pub(crate) source: Option<MemorySource>,
    pub(crate) permanence: Option<MemoryPermanence>,
    pub(crate) limit: Option<usize>,
}

pub(crate) fn parse_memory_search_options(args: &[String]) -> Result<MemorySearchOptions, String> {
    let mut options = MemorySearchOptions::default();
    let mut index = 0usize;

    while index < args.len() {
        match args[index].as_str() {
            "--type" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "缺少 --type 参数值。".to_string())?;
                options.memory_type = Some(parse_memory_type_filter(value)?);
                index += 2;
            }
            "--source" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "缺少 --source 参数值。".to_string())?;
                options.source = Some(parse_memory_source_filter(value)?);
                index += 2;
            }
            "--permanence" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "缺少 --permanence 参数值。".to_string())?;
                options.permanence = Some(parse_memory_permanence_filter(value)?);
                index += 2;
            }
            "--limit" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "缺少 --limit 参数值。".to_string())?;
                let parsed = value
                    .parse::<usize>()
                    .map_err(|_| "--limit 必须是正整数。".to_string())?;
                if parsed == 0 {
                    return Err("--limit 必须是正整数。".to_string());
                }
                options.limit = Some(parsed);
                index += 2;
            }
            other => return Err(format!("未知参数：{other}")),
        }
    }

    Ok(options)
}

pub(crate) fn print_memory_search(query: &str, options: MemorySearchOptions) -> Result<(), String> {
    let kernel = real_memory_kernel()?;
    let results = kernel
        .search(SearchQuery {
            text: query.to_string(),
            scope: Some(global_main_memory_scope()),
            memory_type: options.memory_type,
            source: options.source,
            permanence: options.permanence,
            limit: options.limit,
        })
        .map_err(memory_error)?;

    let mut lines = Vec::new();
    if results.is_empty() {
        lines.push("- 无匹配记忆".to_string());
    } else {
        for result in results {
            let record = result.record;
            lines.push(format!(
                "- {} [{}] {}",
                record.memory_id,
                memory_type_name(&record.memory_type),
                record.content
            ));
            lines.push(format!("  source: {}", memory_source_name(&record.source)));
            lines.push(format!(
                "  permanence: {}",
                memory_permanence_name(&record.permanence)
            ));
            lines.push(format!("  score: {}", result.score));
            lines.push(format!("  matched: {}", result.matched_fields.join(",")));
        }
    }

    emit_cli_panel_body("记忆搜索：", &lines.join("\n"));

    Ok(())
}

fn parse_memory_type_filter(value: &str) -> Result<MemoryType, String> {
    match value {
        "core" => Ok(MemoryType::Core),
        "working" => Ok(MemoryType::Working),
        "status" => Ok(MemoryType::Status),
        "decision" => Ok(MemoryType::Decision),
        _ => Err(format!("无效的 --type：{value}")),
    }
}

fn parse_memory_source_filter(value: &str) -> Result<MemorySource, String> {
    match value {
        "user_explicit" => Ok(MemorySource::UserExplicit),
        "user_correction" => Ok(MemorySource::UserCorrection),
        "assistant_summary" => Ok(MemorySource::AssistantSummary),
        "rule_based_agent" => Ok(MemorySource::RuleBasedAgent),
        _ => Err(format!("无效的 --source：{value}")),
    }
}

fn parse_memory_permanence_filter(value: &str) -> Result<MemoryPermanence, String> {
    match value {
        "standard" => Ok(MemoryPermanence::Standard),
        "permanent" => Ok(MemoryPermanence::Permanent),
        _ => Err(format!("无效的 --permanence：{value}")),
    }
}
