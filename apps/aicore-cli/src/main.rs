use aicore_control::default_control_plane;
use aicore_surface::default_kernel_surface;

fn main() {
    let control_plane = default_control_plane();
    let surface = default_kernel_surface();
    let summary = control_plane.summary();

    println!("AICore CLI 骨架");
    println!("用于读取当前系统摘要。");
    println!();
    println!("组件数: {}", summary.component_count);
    println!("实例数: {}", summary.instance_count);
    println!("记忆提案数: {}", surface.memories.len());
    println!("技能记录数: {}", surface.skills.len());
    println!("工具数: {}", surface.tools.len());
    println!("自进化提案数: {}", surface.evolution_proposals.len());
}
