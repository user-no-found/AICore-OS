mod runtime_status;

use aicore_foundation::AicoreLayout;
use aicore_kernel::default_control_plane;
use aicore_kernel::default_runtime;
use aicore_terminal::{Block, Document, TerminalConfig, render_document};
use runtime_status::GlobalRuntimeStatus;

fn main() {
    let layout = AicoreLayout::from_system_home();
    let global_runtime_status = GlobalRuntimeStatus::load(&layout);
    let control_plane = default_control_plane();
    let runtime = default_runtime();
    let control_summary = control_plane.summary();
    let main_instance = control_plane.main_instance_summary();
    let runtime_summary = runtime.summary();

    let body = [
        format!("主实例：{}", main_instance.id),
        format!("主实例工作目录：{}", main_instance.workspace_root),
        format!("主实例状态目录：{}", main_instance.state_root),
        format!("组件数量：{}", control_summary.component_count),
        format!("实例数量：{}", control_summary.instance_count),
        format!(
            "Runtime：{}/{}",
            runtime_summary.instance_id, runtime_summary.conversation_id
        ),
        String::new(),
        global_runtime_status.render_body(),
    ]
    .join("\n");

    print!(
        "{}",
        render_document(
            &Document::new(vec![Block::panel("AICore OS", &body)]),
            &TerminalConfig::current()
        )
    );
}
