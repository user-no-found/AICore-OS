use aicore_control::default_control_plane;
use aicore_runtime::default_runtime;

fn main() {
    let control_plane = default_control_plane();
    let runtime = default_runtime();
    let surface = control_plane.default_kernel_surface();
    let control_summary = control_plane.summary();
    let runtime_summary = runtime.summary();
    let main_instance = control_plane.main_instance_summary();

    println!("AICore TUI 骨架");
    println!("当前仅提供最小终端摘要视图。");
    println!();
    println!("主实例: {}", main_instance.id);
    println!("组件数: {}", control_summary.component_count);
    println!("会话 ID: {}", runtime_summary.conversation_id);
    println!("消息数: {}", runtime_summary.event_count);
    println!("工具数: {}", surface.tools.len());
    println!("记忆提案数: {}", surface.memories.len());
    println!("说明: 后续在此基础上接入真实 TUI 交互。");
}
