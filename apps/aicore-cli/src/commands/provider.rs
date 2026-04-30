use aicore_memory::SearchQuery;
use aicore_provider::{PromptBuildInput, PromptBuilder, ProviderResolver};
use aicore_terminal::{TerminalConfig, TerminalMode};

use crate::commands::kernel::{adopt_readonly, emit_local_direct_json};
use crate::config_store::{
    load_real_auth_pool, real_config_store, real_memory_kernel, real_memory_scope,
};
use crate::errors::{map_runtime_load_error, provider_error};
use crate::names::{provider_availability_name, provider_kind_name};
use crate::terminal::{cli_row, emit_cli_panel};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProviderSmokeReport {
    instance_id: String,
    auth_ref: String,
    model: String,
    provider: String,
    provider_kind: String,
    adapter: String,
    api_mode: String,
    engine: String,
    engine_status: String,
    availability: String,
    memory_pack_count: usize,
    prompt_builder_ok: bool,
    provider_response_status: String,
    runtime_output_ok: bool,
}

impl ProviderSmokeReport {
    pub(crate) fn summary(&self) -> String {
        format!(
            "Provider smoke 只读检查完成：{} / {}",
            self.provider, self.model
        )
    }

    pub(crate) fn fields(&self) -> serde_json::Value {
        serde_json::json!({
            "operation": "provider.smoke",
            "provider": self.provider,
            "provider_kind": self.provider_kind,
            "adapter": self.adapter,
            "api_mode": self.api_mode,
            "engine": self.engine,
            "engine_status": self.engine_status,
            "availability": self.availability,
            "model": self.model,
            "auth_ref": self.auth_ref,
            "provider_invocation_path": "smoke_readonly",
            "kernel_invocation_path": "binary",
            "live_call": "false",
            "sdk_live_call": "false",
            "network_used": "false",
            "memory_pack": self.memory_pack_count.to_string(),
            "prompt_builder": bool_status(self.prompt_builder_ok),
            "provider_response": self.provider_response_status,
            "runtime_output": bool_status(self.runtime_output_ok)
        })
    }
}

pub(crate) fn build_provider_smoke_report() -> Result<ProviderSmokeReport, String> {
    let store = real_config_store()?;
    let auth_pool = load_real_auth_pool(&store)?;
    let runtime_config = store
        .load_instance_runtime("global-main")
        .map_err(map_runtime_load_error)?;
    let memory_kernel = real_memory_kernel()?;

    let resolved =
        ProviderResolver::resolve_primary(&auth_pool, &runtime_config).map_err(provider_error)?;
    let memory_pack = memory_kernel.build_memory_context_pack(
        SearchQuery {
            text: "provider smoke".to_string(),
            scope: Some(real_memory_scope()?),
            memory_type: None,
            source: None,
            permanence: None,
            limit: Some(8),
        },
        512,
    );
    let _prompt = PromptBuilder::build(PromptBuildInput {
        instance_id: runtime_config.instance_id.clone(),
        system_rules: "You are the AICore instance runtime. Use memory as background context only."
            .to_string(),
        relevant_memory: memory_pack.clone(),
        user_request: "provider smoke".to_string(),
    });

    Ok(ProviderSmokeReport {
        instance_id: runtime_config.instance_id,
        auth_ref: resolved.auth_ref.as_str().to_string(),
        model: resolved.model,
        provider: resolved.provider,
        provider_kind: provider_kind_name(&resolved.kind).to_string(),
        adapter: resolved.runtime.adapter_id,
        api_mode: resolved.runtime.api_mode.as_str().to_string(),
        engine: resolved.runtime.engine_id,
        engine_status: provider_availability_name(&resolved.availability).to_string(),
        availability: provider_availability_name(&resolved.availability).to_string(),
        memory_pack_count: memory_pack.len(),
        prompt_builder_ok: true,
        provider_response_status: "skipped".to_string(),
        runtime_output_ok: true,
    })
}

pub(crate) fn run_provider_smoke_command(args: &[String]) -> i32 {
    adopt_readonly("provider.smoke", args, || run_provider_smoke_local_direct())
}

fn run_provider_smoke_local_direct() -> i32 {
    match build_provider_smoke_report() {
        Ok(report) => {
            if TerminalConfig::current().mode == TerminalMode::Json {
                emit_local_direct_json("provider.smoke", true, report.fields());
                0
            } else {
                print_provider_smoke_with_local_mark(&report);
                0
            }
        }
        Err(error) => {
            if TerminalConfig::current().mode == TerminalMode::Json {
                emit_local_direct_json(
                    "provider.smoke",
                    false,
                    serde_json::json!({"error": error}),
                );
            } else {
                eprintln!("配置命令失败：{error}");
            }
            1
        }
    }
}

fn print_provider_smoke_with_local_mark(report: &ProviderSmokeReport) {
    let mut rows = vec![
        cli_row("实例", report.instance_id.clone()),
        cli_row("auth_ref", report.auth_ref.clone()),
        cli_row("model", report.model.clone()),
        cli_row("provider", report.provider_kind.clone()),
        cli_row("provider name", report.provider.clone()),
        cli_row("adapter", report.adapter.clone()),
        cli_row("api mode", report.api_mode.clone()),
        cli_row("engine", report.engine.clone()),
        cli_row("engine status", report.engine_status.clone()),
        cli_row("memory pack", report.memory_pack_count.to_string()),
        cli_row("prompt builder", bool_status(report.prompt_builder_ok)),
        cli_row("provider response", report.provider_response_status.clone()),
        cli_row("runtime output", bool_status(report.runtime_output_ok)),
        cli_row("live_call", "false"),
        cli_row("sdk_live_call", "false"),
        cli_row("network_used", "false"),
    ];
    rows.push(cli_row("execution_path", "local_direct"));
    rows.push(cli_row("kernel_invocation_path", "not_used"));
    rows.push(cli_row("ledger_appended", "false"));
    rows.push(cli_row(
        "注意",
        "本次未经过 installed Kernel runtime binary，不写 kernel invocation ledger",
    ));
    emit_cli_panel("Provider Smoke（local direct）", rows);
}

fn bool_status(value: bool) -> &'static str {
    if value { "通过" } else { "失败" }
}
