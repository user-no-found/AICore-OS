use aicore_kernel::{default_control_plane, default_runtime};

use crate::terminal::{cli_row, emit_cli_panel, emit_cli_panel_body};

pub(crate) fn print_status() {
    let control_plane = default_control_plane();
    let runtime = default_runtime();
    let control_summary = control_plane.summary();
    let main_instance = control_plane.main_instance_summary();
    let runtime_summary = runtime.summary();

    emit_cli_panel(
        "AICore CLI",
        vec![
            cli_row("主实例", main_instance.id.as_str()),
            cli_row("组件数量", control_summary.component_count.to_string()),
            cli_row("实例数量", control_summary.instance_count.to_string()),
            cli_row(
                "Runtime",
                format!(
                    "{}/{}",
                    runtime_summary.instance_id, runtime_summary.conversation_id
                ),
            ),
        ],
    );
}

pub(crate) fn print_instance_list() {
    let control_plane = default_control_plane();
    let mut lines = Vec::new();

    for instance in control_plane.instance_registry().list() {
        let kind = match instance.kind {
            aicore_kernel::InstanceKind::GlobalMain => "global_main",
            aicore_kernel::InstanceKind::Workspace => "workspace",
        };

        lines.push(format!(
            "- {} [{}] {}",
            instance.id.as_str(),
            kind,
            instance.workspace_root.display()
        ));
    }

    emit_cli_panel_body("实例列表：", &lines.join("\n"));
}
