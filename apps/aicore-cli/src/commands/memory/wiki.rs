use std::{fs, path::PathBuf};

use aicore_memory::{MemoryPaths, ProjectionState};
use aicore_terminal::{Block, Document};

use crate::config_store::real_memory_paths;
use crate::errors::memory_error;
use crate::terminal::emit_document;

pub(crate) fn print_memory_wiki_index() -> Result<(), String> {
    print_memory_wiki_page("index")
}

pub(crate) fn print_memory_wiki_page(page: &str) -> Result<(), String> {
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

    emit_document(Document::new(vec![
        Block::panel("记忆 Wiki Projection：", &metadata.join("\n")),
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

fn resolve_memory_wiki_page(
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
