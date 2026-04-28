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
