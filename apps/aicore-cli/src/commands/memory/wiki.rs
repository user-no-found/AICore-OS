use std::{fs, path::PathBuf};

use aicore_memory::{MemoryPaths, ProjectionState};
use aicore_terminal::{Block, Document, TerminalConfig, TerminalMode};

use crate::commands::kernel::adoption::extract_local_flag;
use crate::commands::kernel::{emit_local_direct_json, print_kernel_invoke_readonly};
use crate::commands::memory::report::{build_memory_wiki_page_report, build_memory_wiki_report};
use crate::config_store::real_memory_paths;
use crate::errors::memory_error;
use crate::terminal::emit_document;

pub(crate) fn run_memory_wiki_command(args: &[String]) -> i32 {
    let (is_local, stripped) = extract_local_flag(args);
    if stripped.is_empty() {
        if is_local {
            run_memory_wiki_index_local_direct()
        } else {
            print_kernel_invoke_readonly("memory.wiki", &[])
        }
    } else {
        let page = &stripped[0];
        if is_local {
            run_memory_wiki_page_local_direct(page)
        } else {
            print_kernel_invoke_readonly("memory.wiki_page", &[page.to_string()])
        }
    }
}

fn run_memory_wiki_index_local_direct() -> i32 {
    if TerminalConfig::current().mode == TerminalMode::Json {
        match build_memory_wiki_report() {
            Ok((_, fields)) => {
                emit_local_direct_json("memory.wiki", true, fields);
                0
            }
            Err(error) => {
                emit_local_direct_json("memory.wiki", false, serde_json::json!({"error": error}));
                1
            }
        }
    } else {
        match print_memory_wiki_index_with_local_mark() {
            Ok(()) => 0,
            Err(error) => {
                eprintln!("记忆命令失败：{error}");
                1
            }
        }
    }
}

fn run_memory_wiki_page_local_direct(page: &str) -> i32 {
    if TerminalConfig::current().mode == TerminalMode::Json {
        match build_memory_wiki_page_report(page) {
            Ok((_, fields)) => {
                emit_local_direct_json("memory.wiki_page", true, fields);
                0
            }
            Err(error) => {
                emit_local_direct_json(
                    "memory.wiki_page",
                    false,
                    serde_json::json!({"error": error}),
                );
                1
            }
        }
    } else {
        match print_memory_wiki_page_with_local_mark(page) {
            Ok(()) => 0,
            Err(error) => {
                eprintln!("记忆命令失败：{error}");
                1
            }
        }
    }
}

fn print_memory_wiki_index_with_local_mark() -> Result<(), String> {
    print_memory_wiki_page_with_local_mark_inner("index")
}

fn print_memory_wiki_page_with_local_mark(page: &str) -> Result<(), String> {
    print_memory_wiki_page_with_local_mark_inner(page)
}

fn print_memory_wiki_page_with_local_mark_inner(page: &str) -> Result<(), String> {
    let paths = real_memory_paths()?;
    let kernel = aicore_memory::MemoryKernel::open(paths.clone()).map_err(memory_error)?;
    let (page_name, page_path) = resolve_memory_wiki_page(&paths, page)?;

    if !page_path.exists() {
        return Err("缺少 Wiki Projection，请先写入记忆或重建 projection。".to_string());
    }

    let content = fs::read_to_string(&page_path)
        .map_err(|error| format!("无法读取 Wiki Projection {}: {error}", page_path.display()))?;

    let mut metadata = wiki_projection_status_lines(kernel.projection_state());
    metadata.push(format!("- page: {page_name}"));
    metadata.push(format!("- path: {}", page_path.display()));
    metadata.push("- execution_path: local_direct".to_string());
    metadata.push("- kernel_invocation_path: not_used".to_string());
    metadata.push("- ledger_appended: false".to_string());
    metadata.push(
        "- 注意：本次未经过 installed Kernel runtime binary，不写 kernel invocation ledger"
            .to_string(),
    );

    emit_document(Document::new(vec![
        Block::panel(
            "记忆 Wiki Projection（local direct）：",
            &metadata.join("\n"),
        ),
        Block::markdown(&content),
    ]));

    Ok(())
}

pub(crate) fn wiki_projection_status_lines(state: &ProjectionState) -> Vec<String> {
    let mut lines = Vec::new();
    if state.stale {
        lines.push("Projection 状态：stale".to_string());
    }
    if let Some(warning) = state.warning.as_deref() {
        lines.push(format!("Projection warning：{warning}"));
    }
    lines
}

pub(crate) fn resolve_memory_wiki_page(
    paths: &MemoryPaths,
    page: &str,
) -> Result<(&'static str, PathBuf), String> {
    if page.contains('/') || page.contains('\\') || page.contains("..") {
        return Err("不允许读取任意 Wiki 路径。".to_string());
    }

    let normalized = page.trim_end_matches(".md");

    match normalized {
        "index" => Ok(("index", paths.wiki_index_md.clone())),
        "core" => Ok(("core", paths.wiki_core_md.clone())),
        "decisions" => Ok(("decisions", paths.wiki_decisions_md.clone())),
        "status" => Ok(("status", paths.wiki_status_md.clone())),
        _ => Err(format!("未知 Wiki 页面：{page}")),
    }
}
