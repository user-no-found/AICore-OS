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

pub(crate) fn print_model_show() -> Result<(), String> {
    let store = real_config_store()?;
    let runtime = store
        .load_instance_runtime("global-main")
        .map_err(map_runtime_load_error)?;

    let mut rows = vec![
        cli_row("instance", runtime.instance_id),
        cli_row("primary auth_ref", runtime.primary.auth_ref.as_str()),
        cli_row("primary model", runtime.primary.model),
    ];
    if let Some(fallback) = runtime.fallback {
        rows.push(cli_row("fallback auth_ref", fallback.auth_ref.as_str()));
        rows.push(cli_row("fallback model", fallback.model));
    } else {
        rows.push(cli_row("fallback", "未配置"));
    }
    emit_cli_panel("实例模型配置", rows);

    Ok(())
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
