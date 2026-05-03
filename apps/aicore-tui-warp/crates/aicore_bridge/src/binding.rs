use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

use crate::env::{binding_from_launcher_env, export_binding_to_env};
use crate::layout::ensure_instance_layout;
use crate::metadata::parse_instance_metadata;
use crate::token::{sanitize_token, stable_workspace_hash};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AicoreWarpBinding {
    pub instance_id: String,
    pub instance_kind: String,
    pub workspace_root: PathBuf,
    pub instance_root: PathBuf,
}

pub fn bind_current_instance() -> Result<AicoreWarpBinding> {
    if let Some(binding) = binding_from_launcher_env()? {
        export_binding_to_env(&binding);
        return Ok(binding);
    }

    let cwd = std::env::current_dir().context("read current directory")?;
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| cwd.clone());
    bind_instance_for_paths(&cwd, &home)
}

pub fn bind_instance_for_paths(cwd: &Path, home: &Path) -> Result<AicoreWarpBinding> {
    if !cwd.is_absolute() {
        bail!("cwd must be absolute: {}", cwd.display());
    }
    if !home.is_absolute() {
        bail!("home must be absolute: {}", home.display());
    }

    let binding = if cwd == home {
        global_main_binding(home)
    } else if let Some(workspace_root) = find_workspace_root(cwd, home) {
        workspace_binding(workspace_root)?
    } else if cwd.starts_with(home) {
        workspace_binding(cwd.to_path_buf())?
    } else {
        let workspace_root = find_workspace_root_outside_home(cwd).unwrap_or_else(|| cwd.into());
        workspace_binding(workspace_root)?
    };

    ensure_instance_layout(&binding)?;
    export_binding_to_env(&binding);
    Ok(binding)
}

fn global_main_binding(home: &Path) -> AicoreWarpBinding {
    AicoreWarpBinding {
        instance_id: "global-main".to_string(),
        instance_kind: "global-main".to_string(),
        workspace_root: home.to_path_buf(),
        instance_root: home.join(".aicore"),
    }
}

fn workspace_binding(workspace_root: PathBuf) -> Result<AicoreWarpBinding> {
    let instance_root = workspace_root.join(".aicore");
    let instance_id = workspace_instance_id(&workspace_root, &instance_root)?;
    Ok(AicoreWarpBinding {
        instance_id,
        instance_kind: "workspace".to_string(),
        workspace_root,
        instance_root,
    })
}

fn workspace_instance_id(workspace_root: &Path, instance_root: &Path) -> Result<String> {
    let instance_toml = instance_root.join("instance.toml");
    if instance_toml.exists() {
        let contents = fs::read_to_string(&instance_toml)
            .with_context(|| format!("read {}", instance_toml.display()))?;
        let metadata = parse_instance_metadata(&contents)?;
        if let Some(instance_kind) = metadata.instance_kind {
            if instance_kind != "workspace" {
                bail!("workspace instance metadata has invalid instance_kind: {instance_kind}");
            }
        }
        if let Some(instance_id) = metadata.instance_id {
            if instance_id == "global-main" {
                bail!("workspace instance metadata cannot use global-main");
            }
            return Ok(instance_id);
        }
        bail!("workspace instance metadata missing instance_id");
    }

    let name = workspace_root
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("workspace");
    let sanitized = sanitize_token(name);
    let hash = stable_workspace_hash(workspace_root);
    Ok(format!("workspace.{sanitized}.{hash}"))
}

fn find_workspace_root(cwd: &Path, home: &Path) -> Option<PathBuf> {
    let mut current = Some(cwd);
    while let Some(path) = current {
        if path == home {
            return None;
        }
        if path.join(".aicore").is_dir() {
            return Some(path.to_path_buf());
        }
        current = path.parent();
    }
    None
}

fn find_workspace_root_outside_home(cwd: &Path) -> Option<PathBuf> {
    let mut current = Some(cwd);
    while let Some(path) = current {
        if path.join(".aicore").is_dir() {
            return Some(path.to_path_buf());
        }
        current = path.parent();
    }
    None
}
