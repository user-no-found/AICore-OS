use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::binding::AicoreWarpBinding;

const DEFAULT_INSTANCE_SOUL: &str = "You are AICore OS current active turn's Agent Runtime.\n";

pub(crate) fn ensure_instance_layout(binding: &AicoreWarpBinding) -> Result<()> {
    fs::create_dir_all(&binding.instance_root)
        .with_context(|| format!("create {}", binding.instance_root.display()))?;
    for dir in [
        "sessions", "events", "memory", "prompts", "runtime", "bindings", "config", "registry",
        "cache", "logs", "tmp", "skills", "tools", "team",
    ] {
        fs::create_dir_all(binding.instance_root.join(dir))
            .with_context(|| format!("create {}", binding.instance_root.join(dir).display()))?;
    }

    write_if_missing(
        &binding.instance_root.join("instance.toml"),
        &format!(
            "instance_id = \"{}\"\ninstance_kind = \"{}\"\n",
            binding.instance_id, binding.instance_kind
        ),
    )?;
    write_if_missing(
        &binding.instance_root.join("soul.md"),
        DEFAULT_INSTANCE_SOUL,
    )?;
    if binding.instance_kind == "global-main" {
        write_if_missing(&binding.instance_root.join("user_profile.md"), "")?;
    } else {
        ensure_workspace_gitignore(&binding.workspace_root)?;
    }

    Ok(())
}

fn write_if_missing(path: &Path, content: &str) -> Result<()> {
    if path.exists() {
        return Ok(());
    }
    fs::write(path, content).with_context(|| format!("write {}", path.display()))
}

fn ensure_workspace_gitignore(workspace_root: &Path) -> Result<()> {
    let gitignore_path = workspace_root.join(".gitignore");
    let existing = if gitignore_path.exists() {
        fs::read_to_string(&gitignore_path)
            .with_context(|| format!("read {}", gitignore_path.display()))?
    } else {
        String::new()
    };

    if existing.lines().any(|line| line.trim() == ".aicore/") {
        return Ok(());
    }

    let mut updated = existing;
    if !updated.is_empty() && !updated.ends_with('\n') {
        updated.push('\n');
    }
    updated.push_str(".aicore/\n");
    fs::write(&gitignore_path, updated)
        .with_context(|| format!("write {}", gitignore_path.display()))
}
