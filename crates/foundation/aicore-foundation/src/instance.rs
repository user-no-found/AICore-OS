use std::fs;
use std::path::{Path, PathBuf};

use crate::{AicoreError, AicoreResult, InstanceId};

pub const DEFAULT_INSTANCE_SOUL: &str = "You are AICore OS current active turn's Agent Runtime.\n";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstanceKind {
    GlobalMain,
    Workspace,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceBinding {
    pub kind: InstanceKind,
    pub instance_id: InstanceId,
    pub root: PathBuf,
    pub workspace_root: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstancePaths {
    pub root: PathBuf,
    pub instance_toml: PathBuf,
    pub soul_md: PathBuf,
    pub user_profile_md: Option<PathBuf>,
    pub sessions_dir: PathBuf,
    pub events_dir: PathBuf,
    pub memory_dir: PathBuf,
    pub prompts_dir: PathBuf,
    pub runtime_dir: PathBuf,
    pub bindings_dir: PathBuf,
    pub config_dir: PathBuf,
    pub registry_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub logs_dir: PathBuf,
    pub tmp_dir: PathBuf,
    pub skills_dir: PathBuf,
    pub tools_dir: PathBuf,
    pub team_dir: PathBuf,
}

pub fn resolve_instance_for_cwd(cwd: &Path, home: &Path) -> AicoreResult<InstanceBinding> {
    if !cwd.is_absolute() {
        return Err(AicoreError::InvalidPath(format!(
            "cwd must be absolute: {}",
            cwd.display()
        )));
    }
    if !home.is_absolute() {
        return Err(AicoreError::InvalidPath(format!(
            "home must be absolute: {}",
            home.display()
        )));
    }
    if cwd == home {
        return Ok(global_main_binding(home)?);
    }

    if let Some(workspace_root) = find_workspace_root(cwd, home) {
        return Ok(workspace_binding(workspace_root)?);
    }

    if is_within(cwd, home) {
        return Ok(workspace_binding(cwd.to_path_buf())?);
    }

    let workspace_root = find_workspace_root_outside_home(cwd).unwrap_or_else(|| cwd.to_path_buf());
    workspace_binding(workspace_root)
}

pub fn instance_paths(binding: &InstanceBinding) -> InstancePaths {
    let user_profile_md = match binding.kind {
        InstanceKind::GlobalMain => Some(binding.root.join("user_profile.md")),
        InstanceKind::Workspace => None,
    };

    InstancePaths {
        root: binding.root.clone(),
        instance_toml: binding.root.join("instance.toml"),
        soul_md: binding.root.join("soul.md"),
        user_profile_md,
        sessions_dir: binding.root.join("sessions"),
        events_dir: binding.root.join("events"),
        memory_dir: binding.root.join("memory"),
        prompts_dir: binding.root.join("prompts"),
        runtime_dir: binding.root.join("runtime"),
        bindings_dir: binding.root.join("bindings"),
        config_dir: binding.root.join("config"),
        registry_dir: binding.root.join("registry"),
        cache_dir: binding.root.join("cache"),
        logs_dir: binding.root.join("logs"),
        tmp_dir: binding.root.join("tmp"),
        skills_dir: binding.root.join("skills"),
        tools_dir: binding.root.join("tools"),
        team_dir: binding.root.join("team"),
    }
}

pub fn ensure_instance_layout(binding: &InstanceBinding) -> AicoreResult<InstancePaths> {
    let paths = instance_paths(binding);

    fs::create_dir_all(&paths.root).map_err(io_error)?;
    for dir in [
        &paths.sessions_dir,
        &paths.events_dir,
        &paths.memory_dir,
        &paths.prompts_dir,
        &paths.runtime_dir,
        &paths.bindings_dir,
        &paths.config_dir,
        &paths.registry_dir,
        &paths.cache_dir,
        &paths.logs_dir,
        &paths.tmp_dir,
        &paths.skills_dir,
        &paths.tools_dir,
        &paths.team_dir,
    ] {
        fs::create_dir_all(dir).map_err(io_error)?;
    }

    write_if_missing(&paths.instance_toml, &render_instance_toml(binding))?;
    write_if_missing(&paths.soul_md, DEFAULT_INSTANCE_SOUL)?;
    if let Some(user_profile_md) = &paths.user_profile_md {
        write_if_missing(user_profile_md, "")?;
    }

    if let Some(workspace_root) = &binding.workspace_root {
        ensure_workspace_gitignore(workspace_root)?;
    }

    Ok(paths)
}

pub fn ensure_workspace_gitignore(workspace_root: &Path) -> AicoreResult<()> {
    let gitignore_path = workspace_root.join(".gitignore");
    let existing = if gitignore_path.exists() {
        fs::read_to_string(&gitignore_path).map_err(io_error)?
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
    fs::write(gitignore_path, updated).map_err(io_error)
}

fn global_main_binding(home: &Path) -> AicoreResult<InstanceBinding> {
    Ok(InstanceBinding {
        kind: InstanceKind::GlobalMain,
        instance_id: InstanceId::global_main(),
        root: home.join(".aicore"),
        workspace_root: None,
    })
}

fn workspace_binding(workspace_root: PathBuf) -> AicoreResult<InstanceBinding> {
    let root = workspace_root.join(".aicore");
    let instance_id = workspace_instance_id(&workspace_root, &root)?;
    Ok(InstanceBinding {
        kind: InstanceKind::Workspace,
        instance_id,
        root,
        workspace_root: Some(workspace_root),
    })
}

fn workspace_instance_id(workspace_root: &Path, instance_root: &Path) -> AicoreResult<InstanceId> {
    let instance_toml = instance_root.join("instance.toml");
    if instance_toml.exists() {
        let contents = fs::read_to_string(&instance_toml).map_err(io_error)?;
        if let Some(instance_id) = parse_instance_id(&contents)? {
            return Ok(instance_id);
        }
    }

    let name = workspace_root
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("workspace");
    let sanitized = sanitize_token(name);
    let hash = stable_workspace_hash(workspace_root);
    InstanceId::new(format!("workspace.{sanitized}.{hash}"))
}

fn sanitize_token(value: &str) -> String {
    let token: String = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_') {
                ch
            } else {
                '_'
            }
        })
        .collect();

    if token.is_empty() {
        "workspace".to_string()
    } else {
        token
    }
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

fn is_within(path: &Path, ancestor: &Path) -> bool {
    path == ancestor || path.starts_with(ancestor)
}

fn write_if_missing(path: &Path, content: &str) -> AicoreResult<()> {
    if path.exists() {
        return Ok(());
    }
    fs::write(path, content).map_err(io_error)
}

fn render_instance_toml(binding: &InstanceBinding) -> String {
    let kind = match binding.kind {
        InstanceKind::GlobalMain => "global-main",
        InstanceKind::Workspace => "workspace",
    };
    format!(
        "instance_id = \"{}\"\ninstance_kind = \"{}\"\n",
        binding.instance_id.as_str(),
        kind
    )
}

fn parse_instance_id(contents: &str) -> AicoreResult<Option<InstanceId>> {
    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if key.trim() != "instance_id" {
            continue;
        }
        let value = value.trim().trim_matches('"');
        return InstanceId::new(value.to_string()).map(Some);
    }
    Ok(None)
}

fn stable_workspace_hash(workspace_root: &Path) -> String {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;

    let mut hash = FNV_OFFSET;
    for byte in workspace_root.as_os_str().as_encoded_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    format!("{hash:016x}")
}

fn io_error(error: std::io::Error) -> AicoreError {
    AicoreError::Unavailable(error.to_string())
}
