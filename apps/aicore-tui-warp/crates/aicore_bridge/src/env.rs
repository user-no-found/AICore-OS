use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Result};

use crate::binding::AicoreWarpBinding;
use crate::token::is_valid_token;

pub(crate) fn binding_from_launcher_env() -> Result<Option<AicoreWarpBinding>> {
    let Some(instance_id) = std::env::var_os("AICORE_INSTANCE_ID") else {
        return Ok(None);
    };
    let instance_id = instance_id
        .into_string()
        .map_err(|_| anyhow!("AICORE_INSTANCE_ID must be valid UTF-8"))?;
    let instance_kind = required_env_string("AICORE_INSTANCE_KIND")?;
    let workspace_root = required_env_path("AICORE_WORKSPACE_ROOT")?;
    let instance_root = required_env_path("AICORE_INSTANCE_ROOT")?;

    validate_binding_env(
        &instance_id,
        &instance_kind,
        &workspace_root,
        &instance_root,
    )?;
    Ok(Some(AicoreWarpBinding {
        instance_id,
        instance_kind,
        workspace_root,
        instance_root,
    }))
}

pub(crate) fn export_binding_to_env(binding: &AicoreWarpBinding) {
    set_env("AICORE_TUI_SOURCE", "warp_fork");
    set_env("AICORE_INSTANCE_ID", &binding.instance_id);
    set_env("AICORE_INSTANCE_KIND", &binding.instance_kind);
    set_env("AICORE_WORKSPACE_ROOT", binding.workspace_root.as_os_str());
    set_env("AICORE_INSTANCE_ROOT", binding.instance_root.as_os_str());
}

fn required_env_string(key: &str) -> Result<String> {
    let value = std::env::var_os(key).ok_or_else(|| anyhow!("{key} is required"))?;
    value
        .into_string()
        .map_err(|_| anyhow!("{key} must be valid UTF-8"))
}

fn required_env_path(key: &str) -> Result<PathBuf> {
    let value = std::env::var_os(key).ok_or_else(|| anyhow!("{key} is required"))?;
    let path = PathBuf::from(value);
    if !path.is_absolute() {
        bail!("{key} must be absolute: {}", path.display());
    }
    Ok(path)
}

fn validate_binding_env(
    instance_id: &str,
    instance_kind: &str,
    workspace_root: &Path,
    instance_root: &Path,
) -> Result<()> {
    if !is_valid_token(instance_id) {
        bail!("AICORE_INSTANCE_ID is invalid: {instance_id}");
    }
    match instance_kind {
        "global-main" => {
            if instance_id != "global-main" {
                bail!("global-main binding must use instance_id global-main");
            }
        }
        "workspace" => {
            if instance_id == "global-main" {
                bail!("workspace binding cannot use global-main");
            }
        }
        _ => bail!("AICORE_INSTANCE_KIND is invalid: {instance_kind}"),
    }
    if !instance_root.starts_with(workspace_root) {
        bail!("AICORE_INSTANCE_ROOT must be inside AICORE_WORKSPACE_ROOT");
    }
    if instance_root.file_name().and_then(|value| value.to_str()) != Some(".aicore") {
        bail!("AICORE_INSTANCE_ROOT must point to .aicore");
    }
    Ok(())
}

fn set_env(key: &str, value: impl AsRef<std::ffi::OsStr>) {
    unsafe {
        std::env::set_var(key, value);
    }
}
