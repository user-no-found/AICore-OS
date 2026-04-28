use aicore_kernel::{
    GatewaySource, InterruptMode, OutputTarget, TransportEnvelope, default_runtime,
};

use crate::names::{delivery_identity_name, output_target_name};
use crate::terminal::emit_cli_panel_body;

pub(crate) fn print_runtime_smoke() {
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

    let body = vec![
        "CLI 场景：".to_string(),
        format!("  接收决策：{:?}", cli_ingress.decision),
        format!("  账本消息数：{}", cli_runtime.summary().event_count),
        format!("  输出目标：{}", output_target_name(&cli_first.target)),
        format!(
            "  投递身份：{}",
            delivery_identity_name(&cli_first.identity)
        ),
        "External Origin 场景：".to_string(),
        format!(
            "  输出目标：{}",
            output_target_name(&external_origin.target)
        ),
        format!(
            "  投递身份：{}",
            delivery_identity_name(&external_origin.identity)
        ),
        "Follow 场景：".to_string(),
        format!(
            "  输出目标：{}",
            output_target_name(&followed_external.target)
        ),
        format!(
            "  投递身份：{}",
            delivery_identity_name(&followed_external.identity)
        ),
    ];

    emit_cli_panel_body("Runtime Smoke：", &body.join("\n"));
}
