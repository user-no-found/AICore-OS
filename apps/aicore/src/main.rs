use std::env;

use aicore_control::{
    default_control_plane, ControlPlane, EvolutionTargetView, KernelSurface,
    MemoryProposalTypeView, SkillScopeView,
};
use aicore_runtime::{default_runtime, GatewaySource, OutputTarget};

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let control_plane = default_control_plane();
    let kernel_surface = control_plane.default_kernel_surface();

    match args.as_slice() {
        [] => print_help(),
        [cmd] if cmd == "main" => print_main_instance(&control_plane),
        [group, action] if group == "app" && action == "list" => print_app_list(&control_plane),
        [group, action] if group == "instance" && action == "list" => {
            print_instance_list(&control_plane)
        }
        [group, action] if group == "evolution" && action == "list" => {
            print_evolution_list(&kernel_surface)
        }
        [group, action] if group == "memory" && action == "list" => print_memory_list(&kernel_surface),
        [group, action] if group == "skill" && action == "list" => print_skill_list(&kernel_surface),
        [group, action] if group == "tool" && action == "list" => print_tool_list(&kernel_surface),
        [group, action] if group == "runtime" && action == "demo" => print_runtime_demo(),
        _ => {
            eprintln!("未知命令。");
            eprintln!();
            print_help();
            std::process::exit(1);
        }
    }
}

fn print_help() {
    println!("AICore OS");
    println!("统一入口命令。");
    println!();
    println!("可用命令：");
    println!("  aicore app list        查看已知组件");
    println!("  aicore instance list   查看已知实例");
    println!("  aicore main            查看主实例");
    println!("  aicore evolution list  查看自进化提案");
    println!("  aicore memory list     查看记忆提案");
    println!("  aicore skill list      查看技能记录");
    println!("  aicore tool list       查看已知工具");
    println!("  aicore runtime demo    查看实例运行时最小闭环");
}

fn print_app_list(control_plane: &ControlPlane) {
    println!("已知组件：");
    for app in control_plane.app_summaries() {
        println!("- {} [{}] {}", app.id, app.kind, app.description_zh);
    }
}

fn print_instance_list(control_plane: &ControlPlane) {
    println!("已知实例：");
    for instance in control_plane.instance_registry().list() {
        println!(
            "- {} [{}] {}",
            instance.id, instance.kind, instance.workspace_root
        );
    }
}

fn print_main_instance(control_plane: &ControlPlane) {
    let instance = control_plane.main_instance_summary();

    println!("主实例：");
    println!("  ID: {}", instance.id);
    println!("  类型: {}", instance.kind);
    println!("  工作区: {}", instance.workspace_root);
}

fn print_tool_list(surface: &KernelSurface) {
    println!("已知工具：");
    for tool in &surface.tools {
        println!(
            "- {} [{}] {} (rev {})",
            tool.id, tool.toolset, tool.display_name_zh, tool.revision
        );
    }
}

fn print_memory_list(surface: &KernelSurface) {
    println!("记忆提案：");
    for proposal in &surface.memories {
        println!(
            "- {} [{}] {}",
            proposal.id,
            memory_type_name(&proposal.memory_type),
            proposal.normalized_memory
        );
    }
}

fn print_evolution_list(surface: &KernelSurface) {
    println!("自进化提案：");
    for proposal in &surface.evolution_proposals {
        println!(
            "- {} [{}] discussion={}",
            proposal.id,
            evolution_target_name(&proposal.target),
            proposal.requires_user_discussion
        );
    }
}

fn print_skill_list(surface: &KernelSurface) {
    println!("技能记录：");
    for skill in &surface.skills {
        println!(
            "- {} [{}] {}",
            skill.id,
            skill_scope_name(&skill.scope),
            skill.owner
        );
    }
}

fn print_runtime_demo() {
    let mut runtime = default_runtime();
    let output = runtime.handle_user_input(GatewaySource::Cli, "demo");
    let summary = runtime.summary();

    println!("实例运行时演示：");
    println!("  实例: {}", summary.instance_id);
    println!("  会话: {}", summary.conversation_id);
    println!("  账本消息数: {}", summary.event_count);
    println!("  输出目标: {}", output_target_name(&output.target));
    println!("  输出内容: {}", output.content);
}

fn memory_type_name(memory_type: &MemoryProposalTypeView) -> &'static str {
    match memory_type {
        MemoryProposalTypeView::Core => "core",
        MemoryProposalTypeView::Permanent => "permanent",
        MemoryProposalTypeView::Working => "working",
    }
}

fn skill_scope_name(scope: &SkillScopeView) -> &'static str {
    match scope {
        SkillScopeView::Builtin => "builtin",
        SkillScopeView::Global => "global",
        SkillScopeView::GlobalMainPrivate => "global-main-private",
        SkillScopeView::Instance => "instance",
        SkillScopeView::Task => "task",
    }
}

fn evolution_target_name(target: &EvolutionTargetView) -> &'static str {
    match target {
        EvolutionTargetView::Tool => "tool",
        EvolutionTargetView::Prompt => "prompt",
        EvolutionTargetView::Skill => "skill",
        EvolutionTargetView::Soul => "soul",
        EvolutionTargetView::SecurityPolicy => "security-policy",
    }
}

fn output_target_name(target: &OutputTarget) -> &'static str {
    match target {
        OutputTarget::ActiveView => "active-view",
        OutputTarget::ExternalReply => "external-reply",
    }
}
