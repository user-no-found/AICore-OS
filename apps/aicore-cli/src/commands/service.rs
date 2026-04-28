use crate::config_store::{load_real_services, real_config_store};
use crate::names::{service_mode_name, service_role_name};
use crate::terminal::{cli_row, emit_cli_panel};

pub(crate) fn print_service_list() -> Result<(), String> {
    let store = real_config_store()?;
    let services = load_real_services(&store)?;

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
    emit_cli_panel("服务角色配置", rows);

    Ok(())
}
