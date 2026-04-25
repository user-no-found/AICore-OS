use aicore_control::default_control_plane;
use aicore_runtime::{
    DeliveryIdentity, GatewaySource, InterruptMode, OutputTarget, TransportEnvelope,
    default_runtime,
};

pub fn run_from_args(args: Vec<String>) -> i32 {
    match args.as_slice() {
        [cmd] if cmd == "status" => {
            print_status();
            0
        }
        [group, action] if group == "instance" && action == "list" => {
            print_instance_list();
            0
        }
        [group, action] if group == "runtime" && action == "smoke" => {
            print_runtime_smoke();
            0
        }
        _ => {
            eprintln!("未知命令。");
            eprintln!("可用命令：status | instance list | runtime smoke");
            1
        }
    }
}

fn print_status() {
    let control_plane = default_control_plane();
    let runtime = default_runtime();
    let control_summary = control_plane.summary();
    let main_instance = control_plane.main_instance_summary();
    let runtime_summary = runtime.summary();

    println!("AICore CLI");
    println!("主实例：{}", main_instance.id);
    println!("组件数量：{}", control_summary.component_count);
    println!("实例数量：{}", control_summary.instance_count);
    println!(
        "Runtime：{}/{}",
        runtime_summary.instance_id, runtime_summary.conversation_id
    );
}

fn print_instance_list() {
    let control_plane = default_control_plane();

    println!("实例列表：");
    for instance in control_plane.instance_registry().list() {
        let kind = match instance.kind {
            aicore_contracts::InstanceKind::GlobalMain => "global_main",
            aicore_contracts::InstanceKind::Workspace => "workspace",
        };

        println!(
            "- {} [{}] {}",
            instance.id.as_str(),
            kind,
            instance.workspace_root.display()
        );
    }
}

fn print_runtime_smoke() {
    let mut runtime = default_runtime();
    let ingress = runtime.handle_ingress(
        TransportEnvelope {
            source: GatewaySource::Cli,
            platform: None,
            target_id: None,
            sender_id: None,
            is_group: false,
            mentioned_bot: false,
        },
        "hello",
        InterruptMode::Queue,
    );
    let routed = runtime.append_assistant_output("reply");
    let first = routed
        .events
        .first()
        .expect("runtime smoke must have at least one output");

    println!("Runtime Smoke：");
    println!("接收决策：{:?}", ingress.decision);
    println!("账本消息数：{}", runtime.summary().event_count);
    println!("输出目标：{}", output_target_name(&first.target));
    println!("投递身份：{}", delivery_identity_name(&first.identity));
}

fn output_target_name(target: &OutputTarget) -> &'static str {
    match target {
        OutputTarget::Origin => "origin",
        OutputTarget::ActiveViews => "active-views",
        OutputTarget::FollowedExternal => "followed-external",
    }
}

fn delivery_identity_name(identity: &DeliveryIdentity) -> String {
    match identity {
        DeliveryIdentity::ActiveViews => "active-views".to_string(),
        DeliveryIdentity::External {
            platform,
            target_id,
        } => {
            format!("external:{platform}:{target_id}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::run_from_args;

    #[test]
    fn rejects_unknown_command() {
        assert_eq!(run_from_args(vec!["unknown".to_string()]), 1);
    }
}
