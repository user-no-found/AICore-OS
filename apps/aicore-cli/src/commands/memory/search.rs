use aicore_memory::{MemoryPermanence, MemorySource, MemoryType};
use aicore_terminal::{TerminalConfig, TerminalMode};

use crate::commands::kernel::adoption::extract_local_flag;
use crate::commands::kernel::{emit_local_direct_json, print_kernel_invoke_readonly};
use crate::commands::memory::report::build_memory_search_report;
use crate::names::{memory_permanence_name, memory_source_name, memory_type_name};
use crate::terminal::emit_cli_panel_body;

#[derive(Clone, Debug, Default)]
pub(crate) struct MemorySearchOptions {
    pub(crate) memory_type: Option<MemoryType>,
    pub(crate) source: Option<MemorySource>,
    pub(crate) permanence: Option<MemoryPermanence>,
    pub(crate) limit: Option<usize>,
}

impl MemorySearchOptions {
    pub(crate) fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "type": self.memory_type.as_ref().map(memory_type_name),
            "source": self.source.as_ref().map(memory_source_name),
            "permanence": self.permanence.as_ref().map(memory_permanence_name),
            "limit": self.limit
        })
    }
}

pub(crate) fn run_memory_search_command(query: &str, args: &[String]) -> i32 {
    let (is_local, stripped) = extract_local_flag(args);
    if is_local {
        run_memory_search_local_direct(query, &stripped)
    } else {
        let mut invoke_args = vec![query.to_string()];
        invoke_args.extend_from_slice(&stripped);
        print_kernel_invoke_readonly("memory.search", &invoke_args)
    }
}

fn run_memory_search_local_direct(query: &str, args: &[String]) -> i32 {
    match parse_memory_search_options(args)
        .and_then(|options| build_memory_search_report(query, options))
    {
        Ok((_, fields)) => {
            if TerminalConfig::current().mode == TerminalMode::Json {
                emit_local_direct_json("memory.search", true, fields);
                0
            } else {
                print_memory_search_with_local_mark(query, &fields);
                0
            }
        }
        Err(error) => {
            if TerminalConfig::current().mode == TerminalMode::Json {
                emit_local_direct_json("memory.search", false, serde_json::json!({"error": error}));
            } else {
                eprintln!("记忆命令失败：{error}");
            }
            1
        }
    }
}

fn print_memory_search_with_local_mark(query: &str, fields: &serde_json::Value) {
    let mut lines = Vec::new();
    let result_count = fields
        .get("result_count")
        .and_then(|v| v.as_str())
        .unwrap_or("0")
        .parse::<usize>()
        .unwrap_or(0);
    if result_count == 0 {
        lines.push("- 无匹配记忆".to_string());
    } else if let Some(results_str) = fields.get("results").and_then(|v| v.as_str()) {
        if let Ok(results) = serde_json::from_str::<Vec<serde_json::Value>>(results_str) {
            for result in results {
                lines.push(format!(
                    "- {} [{}] {}",
                    field_str(&result, "memory_id"),
                    field_str(&result, "memory_type"),
                    field_str(&result, "content")
                ));
                lines.push(format!("  source: {}", field_str(&result, "source")));
                lines.push(format!(
                    "  permanence: {}",
                    field_str(&result, "permanence")
                ));
                lines.push(format!("  score: {}", field_str(&result, "score")));
                lines.push(format!(
                    "  matched: {}",
                    result
                        .get("matched_fields")
                        .and_then(|m| m.as_array())
                        .map(|arr| arr
                            .iter()
                            .filter_map(|v| v.as_str())
                            .collect::<Vec<_>>()
                            .join(","))
                        .unwrap_or_default()
                ));
            }
        }
    }
    lines.push("- execution_path: local_direct".to_string());
    lines.push("- kernel_invocation_path: not_used".to_string());
    lines.push("- ledger_appended: false".to_string());
    lines.push(
        "- 注意：本次未经过 installed Kernel runtime binary，不写 kernel invocation ledger"
            .to_string(),
    );
    emit_cli_panel_body(
        &format!("记忆搜索（local direct）：query = {query}"),
        &lines.join("\n"),
    );
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

fn field_str(value: &serde_json::Value, key: &str) -> String {
    value
        .get(key)
        .and_then(|v| v.as_str())
        .unwrap_or("<none>")
        .to_string()
}

pub(crate) fn parse_memory_type_filter(value: &str) -> Result<MemoryType, String> {
    match value {
        "core" => Ok(MemoryType::Core),
        "working" => Ok(MemoryType::Working),
        "status" => Ok(MemoryType::Status),
        "decision" => Ok(MemoryType::Decision),
        _ => Err(format!("无效的 --type：{value}")),
    }
}

pub(crate) fn parse_memory_source_filter(value: &str) -> Result<MemorySource, String> {
    match value {
        "user_explicit" => Ok(MemorySource::UserExplicit),
        "user_correction" => Ok(MemorySource::UserCorrection),
        "assistant_summary" => Ok(MemorySource::AssistantSummary),
        "rule_based_agent" => Ok(MemorySource::RuleBasedAgent),
        _ => Err(format!("无效的 --source：{value}")),
    }
}

pub(crate) fn parse_memory_permanence_filter(value: &str) -> Result<MemoryPermanence, String> {
    match value {
        "standard" => Ok(MemoryPermanence::Standard),
        "permanent" => Ok(MemoryPermanence::Permanent),
        _ => Err(format!("无效的 --permanence：{value}")),
    }
}
