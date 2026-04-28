use crate::config_store::real_config_store;
use crate::errors::map_runtime_load_error;
use crate::terminal::{cli_row, emit_cli_panel};

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
