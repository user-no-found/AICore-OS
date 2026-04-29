use aicore_terminal::{TerminalConfig, TerminalMode};

use crate::commands::kernel::{adopt_readonly, emit_local_direct_json};
use crate::config_store::{load_real_services, real_config_store};
use crate::names::{service_mode_name, service_role_name};
use crate::terminal::{cli_row, emit_cli_panel};

#[derive(Debug, Clone)]
pub(crate) struct ServiceListReport {
    pub(crate) service_count: usize,
    pub(crate) services: serde_json::Value,
}

impl ServiceListReport {
    pub(crate) fn summary(&self) -> String {
        format!("服务角色配置读取完成：{} 个角色", self.service_count)
    }

    pub(crate) fn fields(&self) -> serde_json::Value {
        serde_json::json!({
            "operation": "service.list",
            "service_count": self.service_count.to_string(),
            "services": self.services.to_string(),
            "kernel_invocation_path": "binary"
        })
    }
}

pub(crate) fn build_service_list_report() -> Result<ServiceListReport, String> {
    let store = real_config_store()?;
    let services = load_real_services(&store)?;
    let entries = services
        .profiles
        .into_iter()
        .map(|profile| {
            let mode = service_mode_name(&profile.mode);
            serde_json::json!({
                "role": service_role_name(&profile.role),
                "mode": mode,
                "auth_ref": profile
                    .auth_ref
                    .as_ref()
                    .map(|auth_ref| auth_ref.as_str())
                    .unwrap_or("-"),
                "model": profile.model.as_deref().unwrap_or("-"),
                "enabled": (mode != "disabled"),
                "configured": (profile.auth_ref.is_some() || profile.model.is_some())
            })
        })
        .collect::<Vec<_>>();

    Ok(ServiceListReport {
        service_count: entries.len(),
        services: serde_json::Value::Array(entries),
    })
}

pub(crate) fn run_service_list_command(args: &[String]) -> i32 {
    adopt_readonly("service.list", args, || run_service_list_local_direct())
}

fn run_service_list_local_direct() -> i32 {
    match build_service_list_report() {
        Ok(report) => {
            if TerminalConfig::current().mode == TerminalMode::Json {
                emit_local_direct_json("service.list", true, report.fields());
                0
            } else {
                print_service_list_with_local_mark(&report);
                0
            }
        }
        Err(error) => {
            if TerminalConfig::current().mode == TerminalMode::Json {
                emit_local_direct_json("service.list", false, serde_json::json!({"error": error}));
            } else {
                eprintln!("配置命令失败：{error}");
            }
            1
        }
    }
}

fn print_service_list_with_local_mark(_report: &ServiceListReport) {
    let store = match real_config_store() {
        Ok(store) => store,
        Err(error) => {
            eprintln!("配置命令失败：{error}");
            return;
        }
    };
    let services = match load_real_services(&store) {
        Ok(services) => services,
        Err(error) => {
            eprintln!("配置命令失败：{error}");
            return;
        }
    };
    let mut rows = Vec::new();
    for profile in services.profiles {
        let role = service_role_name(&profile.role);
        rows.push(cli_row(
            format!("{role} mode"),
            service_mode_name(&profile.mode),
        ));
        if let Some(auth_ref) = profile.auth_ref {
            rows.push(cli_row(format!("{role} auth_ref"), auth_ref.as_str()));
        }
        if let Some(model) = profile.model {
            rows.push(cli_row(format!("{role} model"), model));
        }
    }
    rows.push(cli_row("execution_path", "local_direct"));
    rows.push(cli_row("kernel_invocation_path", "not_used"));
    rows.push(cli_row("ledger_appended", "false"));
    rows.push(cli_row(
        "注意",
        "本次未经过 installed Kernel runtime binary，不写 kernel invocation ledger",
    ));
    emit_cli_panel("服务角色配置（local direct）", rows);
}
