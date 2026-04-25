use aicore_control::default_control_plane;
use aicore_runtime::default_runtime;

fn main() {
    let control_plane = default_control_plane();
    let runtime = default_runtime();
    let main_instance = control_plane.main_instance_summary();
    let runtime_summary = runtime.summary();

    println!("AICore TUI");
    println!("当前模式：终端 AI 交互");
    println!();
    println!("状态栏：");
    println!(
        "  实例：{} | 会话：{} | 消息数：{}",
        main_instance.id, runtime_summary.conversation_id, runtime_summary.event_count
    );
    println!();
    println!("会话输出区：");
    println!("  暂无会话输出。");
    println!();
    println!("工具与任务事件区：");
    println!("  暂无工具调用。");
    println!();
    println!("输入栏：");
    println!("  > 在此输入当前实例会话命令或消息");
}
