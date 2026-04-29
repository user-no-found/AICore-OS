use aicore_kernel::{
    GatewaySource, InterruptMode, OutputTarget, TransportEnvelope, default_runtime,
};
use aicore_terminal::{TerminalConfig, TerminalMode};

use crate::commands::kernel::{adopt_readonly, emit_local_direct_json};
use crate::names::{delivery_identity_name, output_target_name};
use crate::terminal::emit_cli_panel_body;

#[derive(Debug, Clone)]
pub(crate) struct RuntimeSmokeReport {
    pub(crate) lines: Vec<String>,
    pub(crate) cli_decision: String,
    pub(crate) event_count: usize,
    pub(crate) cli_output_target: String,
    pub(crate) cli_delivery_identity: String,
    pub(crate) external_output_target: String,
    pub(crate) external_delivery_identity: String,
    pub(crate) follow_output_target: String,
    pub(crate) follow_delivery_identity: String,
}

impl RuntimeSmokeReport {
    pub(crate) fn summary(&self) -> String {
        "Runtime smoke 只读检查完成".to_string()
    }

    pub(crate) fn fields(&self) -> serde_json::Value {
        serde_json::json!({
            "operation": "runtime.smoke",
            "runtime_root": aicore_foundation::AicoreLayout::from_system_home().state_root.display().to_string(),
            "foundation_runtime_binary": runtime_binary_status("aicore-foundation"),
            "kernel_runtime_binary": runtime_binary_status("aicore-kernel"),
            "manifests_present": aicore_foundation::AicoreLayout::from_system_home().manifests_root.exists().to_string(),
            "ledger_present": aicore_foundation::AicoreLayout::from_system_home().kernel_state_root.join("invocation-ledger.jsonl").exists().to_string(),
            "status": "ok",
            "warning_count": "0",
            "diagnostics": self.lines.join(" | "),
            "cli_decision": self.cli_decision,
            "event_count": self.event_count.to_string(),
            "cli_output_target": self.cli_output_target,
            "cli_delivery_identity": self.cli_delivery_identity,
            "external_output_target": self.external_output_target,
            "external_delivery_identity": self.external_delivery_identity,
            "follow_output_target": self.follow_output_target,
            "follow_delivery_identity": self.follow_delivery_identity,
            "kernel_invocation_path": "binary"
        })
    }

    pub(crate) fn into_summary_and_fields(self) -> (String, serde_json::Value) {
        (self.summary(), self.fields())
    }
}

pub(crate) fn build_runtime_smoke_report() -> RuntimeSmokeReport {
    let mut cli_runtime = default_runtime();
    let cli_ingress = cli_runtime.handle_ingress(
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
    let cli_routed = cli_runtime.append_assistant_output("reply");
    let cli_first = cli_routed
        .events
        .first()
        .expect("runtime smoke must have at least one output");

    let mut external_runtime = default_runtime();
    external_runtime.handle_ingress(
        TransportEnvelope {
            source: GatewaySource::External,
            platform: Some("feishu".to_string()),
            target_id: Some("chat-1".to_string()),
            sender_id: Some("user-1".to_string()),
            is_group: true,
            mentioned_bot: true,
        },
        "hello from external",
        InterruptMode::Queue,
    );
    let external_routed = external_runtime.append_assistant_output("reply external");
    let external_origin = external_routed
        .events
        .iter()
        .find(|event| event.target == OutputTarget::Origin)
        .expect("external origin smoke must include origin output");

    let mut follow_runtime = default_runtime();
    follow_runtime.follow_external(TransportEnvelope {
        source: GatewaySource::External,
        platform: Some("feishu".to_string()),
        target_id: Some("chat-2".to_string()),
        sender_id: Some("user-2".to_string()),
        is_group: true,
        mentioned_bot: true,
    });
    let follow_routed = follow_runtime.append_assistant_output("reply followed");
    let followed_external = follow_routed
        .events
        .iter()
        .find(|event| event.target == OutputTarget::FollowedExternal)
        .expect("follow smoke must include followed external output");

    let cli_output_target = output_target_name(&cli_first.target).to_string();
    let cli_delivery_identity = delivery_identity_name(&cli_first.identity).to_string();
    let external_output_target = output_target_name(&external_origin.target).to_string();
    let external_delivery_identity = delivery_identity_name(&external_origin.identity).to_string();
    let follow_output_target = output_target_name(&followed_external.target).to_string();
    let follow_delivery_identity = delivery_identity_name(&followed_external.identity).to_string();
    let lines = vec![
        "CLI 场景：".to_string(),
        format!("  接收决策：{:?}", cli_ingress.decision),
        format!("  账本消息数：{}", cli_runtime.summary().event_count),
        format!("  输出目标：{cli_output_target}"),
        format!("  投递身份：{cli_delivery_identity}"),
        "External Origin 场景：".to_string(),
        format!("  输出目标：{external_output_target}"),
        format!("  投递身份：{external_delivery_identity}"),
        "Follow 场景：".to_string(),
        format!("  输出目标：{follow_output_target}"),
        format!("  投递身份：{follow_delivery_identity}"),
    ];

    RuntimeSmokeReport {
        lines,
        cli_decision: format!("{:?}", cli_ingress.decision),
        event_count: cli_runtime.summary().event_count,
        cli_output_target,
        cli_delivery_identity,
        external_output_target,
        external_delivery_identity,
        follow_output_target,
        follow_delivery_identity,
    }
}

fn runtime_binary_status(binary_name: &str) -> &'static str {
    let layout = aicore_foundation::AicoreLayout::from_system_home();
    if layout.bin_root.join(binary_name).exists() {
        "installed"
    } else {
        "missing"
    }
}

pub(crate) fn run_runtime_smoke_command(args: &[String]) -> i32 {
    adopt_readonly("runtime.smoke", args, || run_runtime_smoke_local_direct())
}

fn run_runtime_smoke_local_direct() -> i32 {
    let report = build_runtime_smoke_report();
    if TerminalConfig::current().mode == TerminalMode::Json {
        emit_local_direct_json("runtime.smoke", true, report.fields());
        0
    } else {
        print_runtime_smoke_with_local_mark(&report);
        0
    }
}

fn print_runtime_smoke_with_local_mark(report: &RuntimeSmokeReport) {
    let mut lines = report.lines.clone();
    lines.push(String::new());
    lines.push("---".to_string());
    lines.push("execution_path：local_direct".to_string());
    lines.push("kernel_invocation_path：not_used".to_string());
    lines.push("ledger_appended：false".to_string());
    lines.push(
        "注意：本次未经过 installed Kernel runtime binary，不写 kernel invocation ledger"
            .to_string(),
    );
    emit_cli_panel_body("Runtime Smoke（local direct）：", &lines.join("\n"));
}
