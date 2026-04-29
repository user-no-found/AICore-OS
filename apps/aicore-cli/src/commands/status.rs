use aicore_foundation::AicoreLayout;
use aicore_kernel::{InstalledManifestRegistry, default_control_plane, default_runtime};
use aicore_terminal::{TerminalConfig, TerminalMode};

use crate::commands::kernel::{adopt_readonly, emit_local_direct_json};
use crate::terminal::{cli_row, emit_cli_panel, emit_cli_panel_body};

#[derive(Debug, Clone)]
pub(crate) struct CliStatusReport {
    pub(crate) app: String,
    pub(crate) contract_version: String,
    pub(crate) runtime_root: String,
    pub(crate) foundation_installed: bool,
    pub(crate) kernel_installed: bool,
    pub(crate) manifest_count: usize,
    pub(crate) capability_count: usize,
    pub(crate) bin_path_status: String,
    pub(crate) main_instance: String,
    pub(crate) component_count: usize,
    pub(crate) instance_count: usize,
    pub(crate) runtime_binding: String,
}

impl CliStatusReport {
    pub(crate) fn summary(&self) -> String {
        format!(
            "AICore CLI 状态读取完成：{} components / {} instances",
            self.component_count, self.instance_count
        )
    }

    pub(crate) fn fields(&self) -> serde_json::Value {
        serde_json::json!({
            "operation": "cli.status",
            "app": self.app,
            "contract_version": self.contract_version,
            "runtime_root": self.runtime_root,
            "foundation_installed": yes_no(self.foundation_installed),
            "kernel_installed": yes_no(self.kernel_installed),
            "manifest_count": self.manifest_count.to_string(),
            "capability_count": self.capability_count.to_string(),
            "kernel_invocation_path": "binary",
            "bin_path_status": self.bin_path_status,
            "main_instance": self.main_instance,
            "component_count": self.component_count.to_string(),
            "instance_count": self.instance_count.to_string(),
            "runtime": self.runtime_binding
        })
    }

    pub(crate) fn into_summary_and_fields(self) -> (String, serde_json::Value) {
        (self.summary(), self.fields())
    }
}

#[derive(Debug, Clone)]
pub(crate) struct InstanceListReport {
    pub(crate) instance_count: usize,
    pub(crate) entries: Vec<InstanceListEntry>,
}

#[derive(Debug, Clone)]
pub(crate) struct InstanceListEntry {
    pub(crate) instance_id: String,
    pub(crate) kind: String,
    pub(crate) workspace_root: String,
    pub(crate) status: String,
    pub(crate) active: bool,
    pub(crate) configured: bool,
}

impl InstanceListReport {
    pub(crate) fn summary(&self) -> String {
        format!("实例列表读取完成：{} 个实例", self.instance_count)
    }

    pub(crate) fn lines(&self) -> Vec<String> {
        self.entries
            .iter()
            .map(|entry| {
                format!(
                    "- {} [{}] {}",
                    entry.instance_id, entry.kind, entry.workspace_root
                )
            })
            .collect()
    }

    pub(crate) fn fields(&self) -> serde_json::Value {
        let entries = self
            .entries
            .iter()
            .map(|entry| {
                serde_json::json!({
                    "instance_id": entry.instance_id,
                    "kind": entry.kind,
                    "workspace_root": entry.workspace_root,
                    "status": entry.status,
                    "active": entry.active,
                    "configured": entry.configured
                })
            })
            .collect::<Vec<_>>();
        serde_json::json!({
            "operation": "instance.list",
            "instance_count": self.instance_count.to_string(),
            "instances": serde_json::to_string(&entries).expect("instance entries should encode"),
            "kernel_invocation_path": "binary"
        })
    }

    pub(crate) fn into_summary_and_fields(self) -> (String, serde_json::Value) {
        (self.summary(), self.fields())
    }
}

pub(crate) fn build_cli_status_report() -> CliStatusReport {
    let control_plane = default_control_plane();
    let runtime = default_runtime();
    let control_summary = control_plane.summary();
    let main_instance = control_plane.main_instance_summary();
    let runtime_summary = runtime.summary();
    let layout = AicoreLayout::from_system_home();
    let registry = InstalledManifestRegistry::load_from_dir(&layout.manifests_root)
        .unwrap_or_else(|_| InstalledManifestRegistry::from_manifests(Vec::new()));

    CliStatusReport {
        app: "aicore-cli".to_string(),
        contract_version: read_contract_version(&layout),
        runtime_root: layout.state_root.display().to_string(),
        foundation_installed: layout.runtime_foundation_root.join("install.toml").exists(),
        kernel_installed: layout.runtime_kernel_root.join("install.toml").exists(),
        manifest_count: registry.manifest_count(),
        capability_count: registry.capability_count(),
        bin_path_status: bin_path_status(&layout),
        main_instance: main_instance.id.as_str().to_string(),
        component_count: control_summary.component_count,
        instance_count: control_summary.instance_count,
        runtime_binding: format!(
            "{}/{}",
            runtime_summary.instance_id, runtime_summary.conversation_id
        ),
    }
}

pub(crate) fn build_instance_list_report() -> InstanceListReport {
    let control_plane = default_control_plane();
    let entries = control_plane
        .instance_registry()
        .list()
        .into_iter()
        .map(|instance| {
            let kind = match instance.kind {
                aicore_kernel::InstanceKind::GlobalMain => "global_main",
                aicore_kernel::InstanceKind::Workspace => "workspace",
            };
            InstanceListEntry {
                instance_id: instance.id.as_str().to_string(),
                kind: kind.to_string(),
                workspace_root: instance.workspace_root.display().to_string(),
                status: "configured".to_string(),
                active: matches!(instance.kind, aicore_kernel::InstanceKind::GlobalMain),
                configured: true,
            }
        })
        .collect::<Vec<_>>();
    InstanceListReport {
        instance_count: entries.len(),
        entries,
    }
}

fn read_contract_version(layout: &AicoreLayout) -> String {
    std::fs::read_to_string(layout.runtime_kernel_root.join("version.toml"))
        .ok()
        .and_then(|content| {
            content.lines().find_map(|line| {
                let (key, value) = line.split_once('=')?;
                if key.trim() != "contract_version" {
                    return None;
                }
                Some(value.trim().trim_matches('"').to_string())
            })
        })
        .unwrap_or_else(|| "kernel.app.v1".to_string())
}

fn bin_path_status(layout: &AicoreLayout) -> String {
    if !layout.bin_root.exists() {
        return "missing".to_string();
    }
    let path_env = std::env::var_os("PATH").unwrap_or_default();
    let active = std::env::split_paths(&std::ffi::OsString::from(path_env))
        .any(|entry| entry == layout.bin_root);
    if active {
        "active".to_string()
    } else {
        "exists_not_in_path".to_string()
    }
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

pub(crate) fn run_status_command(args: &[String]) -> i32 {
    crate::commands::kernel::adopt_readonly("cli.status", args, || run_status_local_direct())
}

fn run_status_local_direct() -> i32 {
    let report = build_cli_status_report();
    if aicore_terminal::TerminalConfig::current().mode == aicore_terminal::TerminalMode::Json {
        crate::commands::kernel::emit_local_direct_json("cli.status", true, report.fields());
        0
    } else {
        print_status_with_local_mark(&report);
        0
    }
}

fn print_status_with_local_mark(report: &CliStatusReport) {
    emit_cli_panel(
        "AICore CLI (local direct)",
        vec![
            cli_row("主实例", report.main_instance.clone()),
            cli_row("组件数量", report.component_count.to_string()),
            cli_row("实例数量", report.instance_count.to_string()),
            cli_row("Runtime", report.runtime_binding.clone()),
            cli_row("execution_path", "local_direct"),
            cli_row("kernel_invocation_path", "not_used"),
            cli_row("ledger_appended", "false"),
            cli_row(
                "注意",
                "本次未经过 installed Kernel runtime binary，不写 kernel invocation ledger",
            ),
        ],
    );
}

pub(crate) fn run_instance_list_command(args: &[String]) -> i32 {
    adopt_readonly("instance.list", args, || run_instance_list_local_direct())
}

fn run_instance_list_local_direct() -> i32 {
    let report = build_instance_list_report();
    if TerminalConfig::current().mode == TerminalMode::Json {
        emit_local_direct_json("instance.list", true, report.fields());
        0
    } else {
        print_instance_list_with_local_mark(&report);
        0
    }
}

fn print_instance_list_with_local_mark(report: &InstanceListReport) {
    let mut lines = report.lines();
    lines.push(String::new());
    lines.push("---".to_string());
    lines.push("execution_path：local_direct".to_string());
    lines.push("kernel_invocation_path：not_used".to_string());
    lines.push("ledger_appended：false".to_string());
    lines.push(
        "注意：本次未经过 installed Kernel runtime binary，不写 kernel invocation ledger"
            .to_string(),
    );
    emit_cli_panel_body("实例列表（local direct）：", &lines.join("\n"));
}
