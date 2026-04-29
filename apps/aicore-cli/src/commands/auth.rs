use aicore_terminal::{TerminalConfig, TerminalMode};

use crate::commands::kernel::{adopt_readonly, emit_local_direct_json};
use crate::config_store::{load_real_auth_pool, real_config_store};
use crate::names::{auth_capability_name, auth_kind_name, secret_config_status};
use crate::terminal::{cli_row, emit_cli_panel};

#[derive(Debug, Clone)]
pub(crate) struct AuthListReport {
    pub(crate) auth_count: usize,
    pub(crate) entries: serde_json::Value,
}

impl AuthListReport {
    pub(crate) fn summary(&self) -> String {
        format!("认证池读取完成：{} 条 auth_ref", self.auth_count)
    }

    pub(crate) fn fields(&self) -> serde_json::Value {
        serde_json::json!({
            "operation": "auth.list",
            "auth_count": self.auth_count.to_string(),
            "entries": self.entries.to_string(),
            "kernel_invocation_path": "binary"
        })
    }
}

pub(crate) fn build_auth_list_report() -> Result<AuthListReport, String> {
    let store = real_config_store()?;
    let auth_pool = load_real_auth_pool(&store)?;
    let entries = auth_pool
        .available_entries()
        .into_iter()
        .map(|entry| {
            serde_json::json!({
                "auth_ref": entry.auth_ref.as_str(),
                "provider": entry.provider,
                "kind": auth_kind_name(&entry.kind),
                "enabled": entry.enabled,
                "capabilities": entry
                    .capabilities
                    .iter()
                    .map(auth_capability_name)
                    .collect::<Vec<_>>(),
                "secret": secret_config_status(&entry.secret_ref)
            })
        })
        .collect::<Vec<_>>();

    Ok(AuthListReport {
        auth_count: entries.len(),
        entries: serde_json::Value::Array(entries),
    })
}

pub(crate) fn run_auth_list_command(args: &[String]) -> i32 {
    adopt_readonly("auth.list", args, || run_auth_list_local_direct())
}

fn run_auth_list_local_direct() -> i32 {
    match build_auth_list_report() {
        Ok(report) => {
            if TerminalConfig::current().mode == TerminalMode::Json {
                emit_local_direct_json("auth.list", true, report.fields());
                0
            } else {
                print_auth_list_with_local_mark(&report);
                0
            }
        }
        Err(error) => {
            if TerminalConfig::current().mode == TerminalMode::Json {
                emit_local_direct_json("auth.list", false, serde_json::json!({"error": error}));
            } else {
                eprintln!("配置命令失败：{error}");
            }
            1
        }
    }
}

fn print_auth_list_with_local_mark(_report: &AuthListReport) {
    let store = match real_config_store() {
        Ok(store) => store,
        Err(error) => {
            eprintln!("配置命令失败：{error}");
            return;
        }
    };
    let auth_pool = match load_real_auth_pool(&store) {
        Ok(pool) => pool,
        Err(error) => {
            eprintln!("配置命令失败：{error}");
            return;
        }
    };
    let mut rows = Vec::new();
    for entry in auth_pool.available_entries() {
        rows.push(cli_row("auth_ref", entry.auth_ref.as_str()));
        rows.push(cli_row("provider", entry.provider.as_str()));
        rows.push(cli_row("kind", auth_kind_name(&entry.kind)));
        rows.push(cli_row("enabled", entry.enabled.to_string()));
        rows.push(cli_row(
            "capabilities",
            entry
                .capabilities
                .iter()
                .map(auth_capability_name)
                .collect::<Vec<_>>()
                .join(", "),
        ));
        rows.push(cli_row("secret", secret_config_status(&entry.secret_ref)));
    }
    rows.push(cli_row("execution_path", "local_direct"));
    rows.push(cli_row("kernel_invocation_path", "not_used"));
    rows.push(cli_row("ledger_appended", "false"));
    rows.push(cli_row(
        "注意",
        "本次未经过 installed Kernel runtime binary，不写 kernel invocation ledger",
    ));
    emit_cli_panel("认证池（local direct）", rows);
}
