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

pub(crate) fn print_provider_smoke() -> Result<(), String> {
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

    emit_cli_panel(
        "Provider Smoke",
        vec![
            cli_row("实例", runtime_config.instance_id),
            cli_row("auth_ref", resolved.auth_ref.as_str()),
            cli_row("model", resolved.model),
            cli_row("provider", provider_kind_name(&resolved.kind)),
            cli_row("provider name", resolved.provider),
            cli_row("adapter", resolved.runtime.adapter_id),
            cli_row("api mode", resolved.runtime.api_mode.as_str()),
            cli_row("engine", resolved.runtime.engine_id),
            cli_row(
                "engine status",
                provider_availability_name(&resolved.availability),
            ),
            cli_row("memory pack", memory_pack.len().to_string()),
            cli_row("prompt builder", "通过"),
            cli_row("provider response", "通过"),
            cli_row("runtime output", "通过"),
        ],
    );

    Ok(())
}
