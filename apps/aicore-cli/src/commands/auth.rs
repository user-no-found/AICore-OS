use crate::config_store::{load_real_auth_pool, real_config_store};
use crate::names::{auth_capability_name, auth_kind_name, secret_config_status};
use crate::terminal::{cli_row, emit_cli_panel};

pub(crate) fn print_auth_list() -> Result<(), String> {
    let store = real_config_store()?;
    let auth_pool = load_real_auth_pool(&store)?;

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
    emit_cli_panel("认证池", rows);

    Ok(())
}
