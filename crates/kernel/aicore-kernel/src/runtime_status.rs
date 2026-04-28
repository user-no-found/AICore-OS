use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::path::Path;

use aicore_foundation::AicoreLayout;

use crate::{
    InstalledManifestRegistry, KernelHandlerError, KernelHandlerResult, KernelInvocationEnvelope,
    KernelRouteRuntimeOutput,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeStatusSnapshot {
    pub global_root: String,
    pub foundation_installed: bool,
    pub kernel_installed: bool,
    pub contract_version: String,
    pub manifest_count: usize,
    pub capability_count: usize,
    pub event_ledger_path: String,
    pub bin_path: String,
    pub bin_path_status: String,
    pub foundation_runtime_binary_path: String,
    pub foundation_runtime_binary_installed: bool,
    pub kernel_runtime_binary_path: String,
    pub kernel_runtime_binary_installed: bool,
    pub kernel_invocation_path: String,
}

impl RuntimeStatusSnapshot {
    pub fn load(layout: &AicoreLayout) -> Self {
        Self::load_with_invocation_path(layout, "in_process_test_only")
    }

    pub fn load_with_invocation_path(layout: &AicoreLayout, invocation_path: &str) -> Self {
        let foundation_installed = layout.runtime_foundation_root.join("install.toml").exists();
        let kernel_installed = layout.runtime_kernel_root.join("install.toml").exists();
        let foundation_runtime_binary_path = layout.bin_root.join("aicore-foundation");
        let kernel_runtime_binary_path = layout.bin_root.join("aicore-kernel");
        let contract_version = read_toml_string_key(
            &layout.runtime_kernel_root.join("version.toml"),
            "contract_version",
        )
        .unwrap_or_else(|| "-".to_string());
        let manifest_registry = InstalledManifestRegistry::load_from_dir(&layout.manifests_root)
            .unwrap_or_else(|_| InstalledManifestRegistry::from_manifests(Vec::new()));

        Self {
            global_root: layout.state_root.display().to_string(),
            foundation_installed,
            kernel_installed,
            contract_version,
            manifest_count: manifest_registry.manifest_count(),
            capability_count: manifest_registry.capability_count(),
            event_ledger_path: layout
                .kernel_state_root
                .join("event-ledger.jsonl")
                .display()
                .to_string(),
            bin_path: layout.bin_root.display().to_string(),
            bin_path_status: bin_path_status(&layout.bin_root).to_string(),
            foundation_runtime_binary_installed: foundation_runtime_binary_path.exists(),
            foundation_runtime_binary_path: foundation_runtime_binary_path.display().to_string(),
            kernel_runtime_binary_installed: kernel_runtime_binary_path.exists(),
            kernel_runtime_binary_path: kernel_runtime_binary_path.display().to_string(),
            kernel_invocation_path: invocation_path.to_string(),
        }
    }

    pub fn public_fields(&self) -> BTreeMap<String, String> {
        BTreeMap::from([
            ("global_root".to_string(), self.global_root.clone()),
            (
                "foundation_installed".to_string(),
                yes_no(self.foundation_installed).to_string(),
            ),
            (
                "kernel_installed".to_string(),
                yes_no(self.kernel_installed).to_string(),
            ),
            (
                "contract_version".to_string(),
                self.contract_version.clone(),
            ),
            (
                "manifest_count".to_string(),
                self.manifest_count.to_string(),
            ),
            (
                "capability_count".to_string(),
                self.capability_count.to_string(),
            ),
            (
                "event_ledger_path".to_string(),
                self.event_ledger_path.clone(),
            ),
            ("bin_path".to_string(), self.bin_path.clone()),
            ("bin_path_status".to_string(), self.bin_path_status.clone()),
            (
                "foundation_runtime_binary".to_string(),
                installed_missing(self.foundation_runtime_binary_installed).to_string(),
            ),
            (
                "foundation_runtime_binary_path".to_string(),
                self.foundation_runtime_binary_path.clone(),
            ),
            (
                "kernel_runtime_binary".to_string(),
                installed_missing(self.kernel_runtime_binary_installed).to_string(),
            ),
            (
                "kernel_runtime_binary_path".to_string(),
                self.kernel_runtime_binary_path.clone(),
            ),
            (
                "kernel_invocation_path".to_string(),
                self.kernel_invocation_path.clone(),
            ),
        ])
    }

    pub fn summary(&self) -> String {
        format!(
            "global root={} | foundation installed={} | kernel installed={} | manifest count={} | capability count={} | bin path status={} | kernel invocation path={}",
            self.global_root,
            yes_no(self.foundation_installed),
            yes_no(self.kernel_installed),
            self.manifest_count,
            self.capability_count,
            self.bin_path_status,
            self.kernel_invocation_path
        )
    }

    pub fn into_handler_result(self) -> KernelHandlerResult {
        KernelHandlerResult::structured("runtime.status", self.public_fields(), self.summary())
    }
}

pub fn runtime_status_handler_for_layout(
    layout: AicoreLayout,
) -> impl Fn(
    &KernelInvocationEnvelope,
    &KernelRouteRuntimeOutput,
) -> Result<KernelHandlerResult, KernelHandlerError>
+ Send
+ Sync
+ 'static {
    move |_envelope, _route| Ok(RuntimeStatusSnapshot::load(&layout).into_handler_result())
}

pub fn runtime_status_handler_for_layout_with_invocation_path(
    layout: AicoreLayout,
    invocation_path: impl Into<String>,
) -> impl Fn(
    &KernelInvocationEnvelope,
    &KernelRouteRuntimeOutput,
) -> Result<KernelHandlerResult, KernelHandlerError>
+ Send
+ Sync
+ 'static {
    let invocation_path = invocation_path.into();
    move |_envelope, _route| {
        Ok(
            RuntimeStatusSnapshot::load_with_invocation_path(&layout, &invocation_path)
                .into_handler_result(),
        )
    }
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

fn installed_missing(value: bool) -> &'static str {
    if value { "installed" } else { "missing" }
}

fn bin_path_status(bin_path: &Path) -> &'static str {
    if !bin_path.exists() {
        return "missing";
    }
    let path_env = std::env::var_os("PATH").unwrap_or_default();
    let active = std::env::split_paths(&OsString::from(path_env)).any(|entry| entry == bin_path);
    if active {
        "active"
    } else {
        "exists_not_in_path"
    }
}

fn read_toml_string_key(path: &Path, key: &str) -> Option<String> {
    let content = fs::read_to_string(path).ok()?;
    content.lines().find_map(|line| {
        let (current_key, value) = line.split_once('=')?;
        if current_key.trim() != key {
            return None;
        }
        Some(value.trim().trim_matches('"').to_string())
    })
}
