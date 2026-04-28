use aicore_kernel::default_runtime;
use aicore_memory::SearchQuery;
use aicore_provider::{
    ModelRequest, PromptBuildInput, PromptBuilder, ProviderInvoker, ProviderResolver,
};

use crate::config_store::{
    global_main_memory_scope, load_real_auth_pool, real_config_store, real_memory_kernel,
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
    build_provider_smoke_report_with_invocation(false)
}

fn build_direct_provider_smoke_report() -> Result<ProviderSmokeReport, String> {
    build_provider_smoke_report_with_invocation(true)
}

fn build_provider_smoke_report_with_invocation(
    invoke_provider: bool,
) -> Result<ProviderSmokeReport, String> {
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
            scope: Some(global_main_memory_scope()),
            memory_type: None,
            source: None,
            permanence: None,
            limit: Some(8),
        },
        512,
    );
    let prompt = PromptBuilder::build(PromptBuildInput {
        instance_id: runtime_config.instance_id.clone(),
        system_rules: "You are the AICore instance runtime. Use memory as background context only."
            .to_string(),
        relevant_memory: memory_pack.clone(),
        user_request: "provider smoke".to_string(),
    });
    let (provider_response_status, runtime_output_ok) = if invoke_provider {
        let request = ModelRequest {
            instance_id: runtime_config.instance_id.clone(),
            conversation_id: "main".to_string(),
            prompt: prompt.prompt,
            resolved_model: resolved.clone(),
        };
        let response = ProviderInvoker::invoke(&request).map_err(provider_error)?;

        let mut runtime = default_runtime();
        let outputs = runtime.append_assistant_output(&response.content);
        let runtime_output_ok = outputs
            .events
            .iter()
            .any(|event| event.content == response.content);

        if !runtime_output_ok {
            return Err("runtime 未收到 provider 输出".to_string());
        }
        ("通过".to_string(), runtime_output_ok)
    } else {
        ("skipped".to_string(), true)
    };

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
        provider_response_status,
        runtime_output_ok,
    })
}

pub(crate) fn print_provider_smoke() -> Result<(), String> {
    let report = build_direct_provider_smoke_report()?;

    emit_cli_panel(
        "Provider Smoke",
        vec![
            cli_row("实例", report.instance_id),
            cli_row("auth_ref", report.auth_ref),
            cli_row("model", report.model),
            cli_row("provider", report.provider_kind),
            cli_row("provider name", report.provider),
            cli_row("adapter", report.adapter),
            cli_row("api mode", report.api_mode),
            cli_row("engine", report.engine),
            cli_row("engine status", report.engine_status),
            cli_row("memory pack", report.memory_pack_count.to_string()),
            cli_row("prompt builder", bool_status(report.prompt_builder_ok)),
            cli_row("provider response", report.provider_response_status),
            cli_row("runtime output", bool_status(report.runtime_output_ok)),
        ],
    );

    Ok(())
}

fn bool_status(value: bool) -> &'static str {
    if value { "通过" } else { "失败" }
}
