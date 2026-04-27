use aicore_kernel::default_control_plane;
use aicore_kernel::default_runtime;

fn main() {
    let control_plane = default_control_plane();
    let runtime = default_runtime();
    let control_summary = control_plane.summary();
    let main_instance = control_plane.main_instance_summary();
    let runtime_summary = runtime.summary();

    println!("AICore OS");
    println!("主实例：{}", main_instance.id);
    println!("主实例工作目录：{}", main_instance.workspace_root);
    println!("主实例状态目录：{}", main_instance.state_root);
    println!("组件数量：{}", control_summary.component_count);
    println!("实例数量：{}", control_summary.instance_count);
    println!(
        "Runtime：{}/{}",
        runtime_summary.instance_id, runtime_summary.conversation_id
    );
}
