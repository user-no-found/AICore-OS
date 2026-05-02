use aicore_foundation::{InstanceId, SessionId, Timestamp, TurnId};
use aicore_model_protocol::ModelId;
use aicore_team_protocol::*;
use aicore_tool_protocol::ToolId;

fn now(value: u128) -> Timestamp {
    Timestamp::from_unix_millis(value)
}

fn instance_id() -> InstanceId {
    InstanceId::new("workspace.demo").unwrap()
}

fn session_id() -> SessionId {
    SessionId::new("session.1").unwrap()
}

fn turn_id() -> TurnId {
    TurnId::new("turn.1").unwrap()
}

fn channel_id() -> TeamChannelId {
    TeamChannelId::new("teamchannel.1").unwrap()
}

fn agent_id() -> TeamAgentId {
    TeamAgentId::new("teamagent.1").unwrap()
}

fn tool_id() -> ToolId {
    ToolId::new("tool.git.status").unwrap()
}

fn budget() -> TeamBudget {
    TeamBudget {
        max_input_tokens: 1200,
        max_output_tokens: 800,
        max_messages: 8,
        deadline_at: Some(now(100)),
    }
}

fn context() -> TeamContext {
    TeamContext {
        parent_instance_id: instance_id(),
        parent_session_id: session_id(),
        parent_turn_id: turn_id(),
        team_run_id: TeamRunId::new("teamrun.1").unwrap(),
        team_channel_id: channel_id(),
        team_generation: 1,
        created_by_agent_id: TeamAgentId::new("main.agent").unwrap(),
        created_at: now(1),
        status: TeamRunStatus::Created,
        team_budget: budget(),
        spawn_depth_limit: 1,
        concurrency_limit: 2,
        agents: vec![],
    }
}

fn spawn_request() -> TeamSpawnRequest {
    TeamSpawnRequest {
        team_agent_id: agent_id(),
        parent_instance_id: instance_id(),
        parent_session_id: session_id(),
        parent_turn_id: turn_id(),
        team_channel_id: channel_id(),
        role_name: "reviewer".to_string(),
        task: "Review scoped changes.".to_string(),
        model: ModelId::new("mock.model").unwrap(),
        instructions: "Use only provided summaries.".to_string(),
        allowed_tools: vec![tool_id()],
        communication_scope: TeamCommunicationScope::MainVisible,
        output_contract: "Return findings and risks.".to_string(),
        budget: Some(budget()),
        deadline: Some(now(90)),
        spawn_depth: 1,
        created_at: now(2),
    }
}

#[test]
fn core_types_round_trip_through_json() {
    let ctx = context();
    let json = serde_json::to_string(&ctx).unwrap();
    let decoded: TeamContext = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded.parent_instance_id, instance_id());
    assert_eq!(decoded.parent_session_id, session_id());
    assert_eq!(decoded.parent_turn_id, turn_id());
}

#[test]
fn team_context_fields_are_complete() {
    let ctx = context();
    assert_eq!(ctx.team_run_id.as_str(), "teamrun.1");
    assert_eq!(ctx.team_channel_id, channel_id());
    assert_eq!(ctx.team_generation, 1);
    assert_eq!(ctx.created_by_agent_id.as_str(), "main.agent");
    assert_eq!(ctx.status, TeamRunStatus::Created);
    assert_eq!(ctx.spawn_depth_limit, 1);
    assert_eq!(ctx.concurrency_limit, 2);
}

#[test]
fn spawn_request_fields_are_complete() {
    let request = spawn_request();
    assert_eq!(request.parent_instance_id, instance_id());
    assert_eq!(request.parent_session_id, session_id());
    assert_eq!(request.parent_turn_id, turn_id());
    assert_eq!(request.team_channel_id, channel_id());
    assert_eq!(request.team_agent_id, agent_id());
    assert_eq!(request.role_name, "reviewer");
    assert_eq!(request.allowed_tools, vec![tool_id()]);
    assert_eq!(
        request.communication_scope,
        TeamCommunicationScope::MainVisible
    );
    assert_eq!(request.spawn_depth, 1);
    assert!(request.budget.is_some());
}

#[test]
fn enum_contract_values_are_legal() {
    assert_eq!(
        serde_json::to_string(&TeamRunStatus::Running).unwrap(),
        "\"running\""
    );
    assert_eq!(
        serde_json::to_string(&TeamAgentStatus::WaitingApproval).unwrap(),
        "\"waiting_approval\""
    );
    assert_eq!(
        serde_json::to_string(&TeamMessageKind::ResultSummary).unwrap(),
        "\"result_summary\""
    );
    assert_eq!(
        serde_json::to_string(&TeamResultStatus::LateIgnored).unwrap(),
        "\"late_ignored\""
    );
}

#[test]
fn team_agent_descriptor_has_no_persistent_identity_or_memory_fields() {
    let descriptor = TeamAgentDescriptor {
        team_agent_id: agent_id(),
        role_name: "reviewer".to_string(),
        task: "Review scoped changes.".to_string(),
        model: ModelId::new("mock.model").unwrap(),
        allowed_tools: vec![tool_id()],
        status: TeamAgentStatus::Created,
        spawn_depth: 1,
        created_at: now(3),
    };
    let json = serde_json::to_string(&descriptor).unwrap();
    for word in [
        "soul",
        "user_profile",
        "long_memory",
        "memory_namespace",
        "persistent_identity",
        "cross_turn",
    ] {
        assert!(!json.contains(word), "unexpected long-lived field: {word}");
    }
}

#[test]
fn no_raw_leak_guard() {
    let message = TeamMessage {
        message_id: TeamMessageId::new("teammsg.1").unwrap(),
        team_channel_id: channel_id(),
        sender_agent_id: agent_id(),
        recipient_agent_id: None,
        kind: TeamMessageKind::Finding,
        communication_scope: TeamCommunicationScope::TeamVisible,
        summary_en: "Found a boundary risk.".to_string(),
        summary_zh: Some("发现边界风险。".to_string()),
        source_refs: vec!["src/lib.rs".to_string()],
        created_at: now(4),
        seq: 1,
    };
    let json = serde_json::to_string(&message).unwrap();
    for word in [
        "raw_provider_payload",
        "raw_tool_output",
        "raw_memory_content",
        "hidden_reasoning",
        "full_prompt",
        "secret",
        "token",
        "api_key",
        "cookie",
        "credential",
        "authorization",
        "password",
    ] {
        assert!(!json.contains(word), "forbidden field leaked: {word}");
    }
}
