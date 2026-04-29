use aicore_terminal::{TerminalConfig, TerminalMode};

use crate::commands::kernel::{adopt_readonly, emit_local_direct_json};
use crate::config_store::{load_real_auth_pool, real_config_store};
use crate::errors::map_runtime_load_error;
use crate::names::auth_kind_name;
use crate::terminal::{cli_row, emit_cli_panel};

#[derive(Debug, Clone)]
pub(crate) struct ModelShowReport {
    pub(crate) fields: serde_json::Value,
}

impl ModelShowReport {
    pub(crate) fn summary(&self) -> String {
        "实例模型配置读取完成".to_string()
    }

    pub(crate) fn fields(&self) -> serde_json::Value {
        self.fields.clone()
    }
}

pub(crate) fn build_model_show_report() -> Result<ModelShowReport, String> {
    let store = real_config_store()?;
    let runtime = store
        .load_instance_runtime("global-main")
        .map_err(map_runtime_load_error)?;
    let auth_pool = load_real_auth_pool(&store)?;
    let primary_auth = auth_pool
        .available_entries()
        .into_iter()
        .find(|entry| entry.auth_ref == runtime.primary.auth_ref);
    let (provider, provider_kind) = primary_auth
        .map(|entry| (entry.provider.as_str(), auth_kind_name(&entry.kind)))
        .unwrap_or(("-", "-"));

    let (fallback, fallback_auth_ref, fallback_model) =
        runtime
            .fallback
            .as_ref()
            .map_or(("missing", "-", "-"), |fallback| {
                (
                    "configured",
                    fallback.auth_ref.as_str(),
                    fallback.model.as_str(),
                )
            });

    Ok(ModelShowReport {
        fields: serde_json::json!({
            "operation": "model.show",
            "runtime_config_present": "true",
            "instance": runtime.instance_id,
            "primary_auth_ref": runtime.primary.auth_ref.as_str(),
            "primary_model": runtime.primary.model,
            "provider": provider,
            "provider_kind": provider_kind,
            "fallback": fallback,
            "fallback_auth_ref": fallback_auth_ref,
            "fallback_model": fallback_model,
            "kernel_invocation_path": "binary"
        }),
    })
}

pub(crate) fn run_model_show_command(args: &[String]) -> i32 {
    adopt_readonly("model.show", args, || run_model_show_local_direct())
}

fn run_model_show_local_direct() -> i32 {
    match build_model_show_report() {
        Ok(report) => {
            if TerminalConfig::current().mode == TerminalMode::Json {
                emit_local_direct_json("model.show", true, report.fields());
                0
            } else {
                print_model_show_with_local_mark(&report);
                0
            }
        }
        Err(error) => {
            if TerminalConfig::current().mode == TerminalMode::Json {
                emit_local_direct_json("model.show", false, serde_json::json!({"error": error}));
            } else {
                eprintln!("配置命令失败：{error}");
            }
            1
        }
    }
}

fn print_model_show_with_local_mark(report: &ModelShowReport) {
    let instance = report
        .fields
        .get("instance")
        .and_then(|v| v.as_str())
        .unwrap_or("-");
    let primary_auth_ref = report
        .fields
        .get("primary_auth_ref")
        .and_then(|v| v.as_str())
        .unwrap_or("-");
    let primary_model = report
        .fields
        .get("primary_model")
        .and_then(|v| v.as_str())
        .unwrap_or("-");
    let fallback = report
        .fields
        .get("fallback")
        .and_then(|v| v.as_str())
        .unwrap_or("-");
    let fallback_auth_ref = report
        .fields
        .get("fallback_auth_ref")
        .and_then(|v| v.as_str())
        .unwrap_or("-");
    let fallback_model = report
        .fields
        .get("fallback_model")
        .and_then(|v| v.as_str())
        .unwrap_or("-");

    let mut rows = vec![
        cli_row("instance", instance),
        cli_row("primary auth_ref", primary_auth_ref),
        cli_row("primary model", primary_model),
    ];
    if fallback == "configured" {
        rows.push(cli_row("fallback auth_ref", fallback_auth_ref));
        rows.push(cli_row("fallback model", fallback_model));
    } else {
        rows.push(cli_row("fallback", "未配置"));
    }
    rows.push(cli_row("execution_path", "local_direct"));
    rows.push(cli_row("kernel_invocation_path", "not_used"));
    rows.push(cli_row("ledger_appended", "false"));
    rows.push(cli_row(
        "注意",
        "本次未经过 installed Kernel runtime binary，不写 kernel invocation ledger",
    ));
    emit_cli_panel("实例模型配置（local direct）", rows);
}
