mod runtime_status;

use aicore_foundation::AicoreLayout;
use aicore_kernel::default_control_plane;
use aicore_kernel::default_runtime;
use aicore_terminal::{Block, Document, TerminalConfig, TerminalMode, render_document};
use runtime_status::{
    invoke_runtime_status, kernel_invocation_result_json, runtime_status_rows, status_code,
};

fn main() {
    std::process::exit(run());
}

fn run() -> i32 {
    let layout = AicoreLayout::from_system_home();
    let runtime_status = invoke_runtime_status(&layout);
    if TerminalConfig::current().mode == TerminalMode::Json {
        emit_kernel_invocation_result_json(&runtime_status);
        return status_code(&runtime_status);
    }

    let control_plane = default_control_plane();
    let runtime = default_runtime();
    let control_summary = control_plane.summary();
    let main_instance = control_plane.main_instance_summary();
    let runtime_summary = runtime.summary();
    let mut rows = vec![
        ("主实例".to_string(), main_instance.id.to_string()),
        (
            "主实例工作目录".to_string(),
            main_instance.workspace_root.to_string(),
        ),
        (
            "主实例状态目录".to_string(),
            main_instance.state_root.to_string(),
        ),
        (
            "组件数量".to_string(),
            control_summary.component_count.to_string(),
        ),
        (
            "实例数量".to_string(),
            control_summary.instance_count.to_string(),
        ),
        (
            "Runtime".to_string(),
            format!(
                "{}/{}",
                runtime_summary.instance_id, runtime_summary.conversation_id
            ),
        ),
    ];
    rows.extend(runtime_status_rows(&runtime_status));
    let body = rows
        .into_iter()
        .map(|(key, value)| format!("{key}：{value}"))
        .collect::<Vec<_>>()
        .join("\n");
    let title = if runtime_status
        .get("status")
        .and_then(|value| value.as_str())
        == Some("completed")
    {
        "AICore OS"
    } else {
        "内核状态调用失败"
    };

    print!(
        "{}",
        render_document(
            &Document::new(vec![Block::panel(title, &body)]),
            &TerminalConfig::current()
        )
    );
    status_code(&runtime_status)
}

fn emit_kernel_invocation_result_json(output: &serde_json::Value) {
    let payload = kernel_invocation_result_json(output);
    let payload = serde_json::to_string(&payload).expect("kernel invocation result should encode");
    print!(
        "{}",
        render_document(
            &Document::new(vec![Block::structured_json(
                "kernel.invocation.result",
                &payload,
            )]),
            &TerminalConfig::current()
        )
    );
}
