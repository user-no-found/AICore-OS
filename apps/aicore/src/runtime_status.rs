use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use aicore_foundation::AicoreLayout;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobalRuntimeStatus {
    pub global_root: PathBuf,
    pub foundation_installed: bool,
    pub kernel_installed: bool,
    pub contract_version: String,
    pub manifest_count: usize,
    pub capability_count: usize,
    pub event_ledger_path: PathBuf,
    pub bin_path: PathBuf,
    pub bin_path_status: BinPathStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinPathStatus {
    Active,
    ExistsNotInPath,
    Missing,
}

impl GlobalRuntimeStatus {
    pub fn load(layout: &AicoreLayout) -> Self {
        let foundation_installed = layout.runtime_foundation_root.join("install.toml").exists();
        let kernel_installed = layout.runtime_kernel_root.join("install.toml").exists();
        let contract_version = read_toml_string_key(
            &layout.runtime_kernel_root.join("version.toml"),
            "contract_version",
        )
        .unwrap_or_else(|| "-".to_string());
        let capability_count = read_toml_usize_key(
            &layout.runtime_kernel_root.join("capabilities.toml"),
            "capability_count",
        )
        .unwrap_or(0);

        Self {
            global_root: layout.state_root.clone(),
            foundation_installed,
            kernel_installed,
            contract_version,
            manifest_count: count_toml_files(&layout.manifests_root),
            capability_count,
            event_ledger_path: layout.kernel_state_root.join("event-ledger.jsonl"),
            bin_path: layout.bin_root.clone(),
            bin_path_status: bin_path_status(&layout.bin_root),
        }
    }

    pub fn render_body(&self) -> String {
        [
            format!("global root：{}", self.global_root.display()),
            format!(
                "foundation installed：{}",
                yes_no(self.foundation_installed)
            ),
            format!("kernel installed：{}", yes_no(self.kernel_installed)),
            format!("contract version：{}", self.contract_version),
            format!("manifest count：{}", self.manifest_count),
            format!("capability count：{}", self.capability_count),
            format!("event ledger：{}", self.event_ledger_path.display()),
            format!("bin path：{}", self.bin_path.display()),
            format!("bin path status：{}", self.bin_path_status.label()),
        ]
        .join("\n")
    }
}

impl BinPathStatus {
    fn label(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::ExistsNotInPath => "exists_not_in_path",
            Self::Missing => "missing",
        }
    }
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

fn count_toml_files(path: &Path) -> usize {
    fs::read_dir(path)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter(|entry| {
                    entry.path().extension().and_then(|value| value.to_str()) == Some("toml")
                })
                .count()
        })
        .unwrap_or(0)
}

fn bin_path_status(bin_path: &Path) -> BinPathStatus {
    if !bin_path.exists() {
        return BinPathStatus::Missing;
    }
    let path_env = std::env::var_os("PATH").unwrap_or_default();
    let active = std::env::split_paths(&OsString::from(path_env)).any(|entry| entry == bin_path);
    if active {
        BinPathStatus::Active
    } else {
        BinPathStatus::ExistsNotInPath
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

fn read_toml_usize_key(path: &Path, key: &str) -> Option<usize> {
    let content = fs::read_to_string(path).ok()?;
    content.lines().find_map(|line| {
        let (current_key, value) = line.split_once('=')?;
        if current_key.trim() != key {
            return None;
        }
        value.trim().parse().ok()
    })
}
