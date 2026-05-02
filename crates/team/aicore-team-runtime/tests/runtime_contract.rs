use aicore_foundation::{InstanceId, SessionId, Timestamp, TurnId};
use aicore_model_protocol::ModelId;
use aicore_team_protocol::*;
use aicore_team_runtime::*;
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

fn team_run_id() -> TeamRunId {
    TeamRunId::new("teamrun.1").unwrap()
}

fn agent_id(n: u8) -> TeamAgentId {
    TeamAgentId::new(format!("teamagent.{n}")).unwrap()
}

fn tool_id() -> ToolId {
    ToolId::new("tool.git.status").unwrap()
}

fn model_id() -> ModelId {
    ModelId::new("mock.model").unwrap()
}

fn policy() -> TeamPolicy {
    TeamPolicy {
        max_team_agents_per_turn: 4,
        max_concurrent_team_agents: 2,
        max_spawn_depth: 1,
        allowed_models: vec![model_id()],
        tool_snapshot: vec![tool_id()],
        parent_deadline: Some(now(100)),
    }
}

fn budget() -> TeamBudget {
    TeamBudget {
        max_input_tokens: 1000,
        max_output_tokens: 500,
        max_messages: 8,
        deadline_at: Some(now(90)),
    }
}

fn new_runtime() -> InMemoryTeamRuntime {
    let mut runtime = InMemoryTeamRuntime::new(policy());
    runtime
        .create_team_context(
            instance_id(),
            session_id(),
            turn_id(),
            team_run_id(),
            channel_id(),
            TeamAgentId::new("main.agent").unwrap(),
            budget(),
            now(1),
        )
        .unwrap();
    runtime
}

fn spawn_request(n: u8) -> TeamSpawnRequest {
    TeamSpawnRequest {
        team_agent_id: agent_id(n),
        parent_instance_id: instance_id(),
        parent_session_id: session_id(),
        parent_turn_id: turn_id(),
        team_channel_id: channel_id(),
        role_name: format!("reviewer.{n}"),
        task: "Review scoped changes.".to_string(),
        model: model_id(),
        instructions: "Use only provided summaries.".to_string(),
        allowed_tools: vec![tool_id()],
        communication_scope: TeamCommunicationScope::MainVisible,
        output_contract: "Return findings and risks.".to_string(),
        budget: Some(budget()),
        deadline: Some(now(80)),
        spawn_depth: 1,
        created_at: now(2),
    }
}

fn team_message() -> TeamMessage {
    TeamMessage {
        message_id: TeamMessageId::new("teammsg.1").unwrap(),
        team_channel_id: channel_id(),
        sender_agent_id: agent_id(1),
        recipient_agent_id: None,
        kind: TeamMessageKind::Finding,
        communication_scope: TeamCommunicationScope::TeamVisible,
        summary_en: "Found a boundary issue.".to_string(),
        summary_zh: Some("发现边界问题。".to_string()),
        source_refs: vec!["crates/team".to_string()],
        created_at: now(3),
        seq: 0,
    }
}

fn team_result() -> TeamResult {
    TeamResult {
        result_id: TeamResultId::new("teamresult.1").unwrap(),
        team_agent_id: agent_id(1),
        status: TeamResultStatus::Submitted,
        summary_en: "Review complete.".to_string(),
        summary_zh: Some("复核完成。".to_string()),
        findings: vec![TeamFinding {
            kind: TeamMessageKind::Finding,
            summary_en: "No runtime integration.".to_string(),
            summary_zh: None,
            source_refs: vec!["tests".to_string()],
            created_at: now(4),
        }],
        risks: vec!["mock only".to_string()],
        confidence: 80,
        source_refs: vec!["tests".to_string()],
        created_at: now(4),
    }
}

#[test]
fn create_team_context_success() {
    let runtime = new_runtime();
    let context = runtime.get_team_context().unwrap();
    assert_eq!(context.status, TeamRunStatus::Running);
    assert_eq!(context.parent_turn_id, turn_id());
    assert_eq!(context.team_channel_id, channel_id());
}

#[test]
fn spawn_team_agent_success() {
    let mut runtime = new_runtime();
    let outcome = runtime.spawn_team_agent(spawn_request(1)).unwrap();
    assert_eq!(outcome.agent.status, TeamAgentStatus::Running);
    assert_eq!(runtime.get_team_context().unwrap().agents.len(), 1);
}

#[test]
fn spawn_validation_rejects_depth_agent_concurrency_model_tool_and_budget() {
    let mut runtime = new_runtime();
    let mut request = spawn_request(1);
    request.spawn_depth = 2;
    assert_eq!(
        runtime.spawn_team_agent(request).unwrap_err(),
        TeamSpawnFailureCode::SpawnDepthExceeded
    );

    let mut request = spawn_request(1);
    request.model = ModelId::new("other.model").unwrap();
    assert_eq!(
        runtime.spawn_team_agent(request).unwrap_err(),
        TeamSpawnFailureCode::InvalidModel
    );

    let mut request = spawn_request(1);
    request.allowed_tools = vec![ToolId::new("tool.unknown").unwrap()];
    assert_eq!(
        runtime.spawn_team_agent(request).unwrap_err(),
        TeamSpawnFailureCode::ToolNotAllowed
    );

    let mut request = spawn_request(1);
    request.budget = None;
    assert_eq!(
        runtime.spawn_team_agent(request).unwrap_err(),
        TeamSpawnFailureCode::BudgetMissing
    );

    let mut runtime = new_runtime();
    runtime.spawn_team_agent(spawn_request(1)).unwrap();
    runtime.spawn_team_agent(spawn_request(2)).unwrap();
    assert_eq!(
        runtime.spawn_team_agent(spawn_request(3)).unwrap_err(),
        TeamSpawnFailureCode::ConcurrencyLimit
    );

    runtime.mark_agent_completed(&agent_id(1)).unwrap();
    runtime.mark_agent_completed(&agent_id(2)).unwrap();
    runtime.spawn_team_agent(spawn_request(3)).unwrap();
    runtime.mark_agent_completed(&agent_id(3)).unwrap();
    runtime.spawn_team_agent(spawn_request(4)).unwrap();
    runtime.mark_agent_completed(&agent_id(4)).unwrap();
    assert_eq!(
        runtime.spawn_team_agent(spawn_request(5)).unwrap_err(),
        TeamSpawnFailureCode::TooManyAgents
    );
}

#[test]
fn append_team_message_and_reject_after_channel_closed() {
    let mut runtime = new_runtime();
    runtime.spawn_team_agent(spawn_request(1)).unwrap();
    let message = runtime.append_team_message(team_message()).unwrap();
    assert_eq!(message.seq, 1);
    assert_eq!(runtime.list_team_messages().len(), 1);

    runtime
        .stop_team_run(TeamStopRequest {
            team_run_id: team_run_id(),
            requested_at: now(5),
        })
        .unwrap();
    assert_eq!(
        runtime.append_team_message(team_message()).unwrap_err(),
        TeamRuntimeError::ChannelClosed
    );
}

#[test]
fn submit_result_and_late_result_after_stop() {
    let mut runtime = new_runtime();
    runtime.spawn_team_agent(spawn_request(1)).unwrap();
    let result = runtime.submit_team_result(team_result()).unwrap();
    assert_eq!(result.status, TeamResultStatus::Accepted);
    assert_eq!(runtime.list_team_results().len(), 1);

    runtime
        .stop_team_run(TeamStopRequest {
            team_run_id: team_run_id(),
            requested_at: now(5),
        })
        .unwrap();
    let late = runtime.submit_team_result(team_result()).unwrap();
    assert_eq!(late.status, TeamResultStatus::RejectedTurnStopped);
    assert_eq!(runtime.list_team_results().len(), 1);
}

#[test]
fn stop_team_run_closes_channel_and_stops_agents() {
    let mut runtime = new_runtime();
    runtime.spawn_team_agent(spawn_request(1)).unwrap();
    let outcome = runtime
        .stop_team_run(TeamStopRequest {
            team_run_id: team_run_id(),
            requested_at: now(5),
        })
        .unwrap();
    assert_eq!(outcome.status, TeamRunStatus::Stopped);
    assert_eq!(runtime.channel_state().status, TeamChannelStatus::Closed);
    assert!(
        runtime
            .get_team_context()
            .unwrap()
            .agents
            .iter()
            .all(|agent| agent.status == TeamAgentStatus::Destroyed)
    );
}

#[test]
fn destroy_after_stop_prevents_spawn_message_and_result() {
    let mut runtime = new_runtime();
    runtime.spawn_team_agent(spawn_request(1)).unwrap();
    runtime
        .stop_team_run(TeamStopRequest {
            team_run_id: team_run_id(),
            requested_at: now(5),
        })
        .unwrap();
    let summary = runtime.destroy_team_run(now(6)).unwrap();
    assert_eq!(summary.status, TeamRunStatus::Destroyed);
    assert_eq!(summary.destroyed_agents, 1);
    assert_eq!(
        runtime.spawn_team_agent(spawn_request(2)).unwrap_err(),
        TeamSpawnFailureCode::ChannelClosed
    );
    assert_eq!(
        runtime.append_team_message(team_message()).unwrap_err(),
        TeamRuntimeError::Destroyed
    );
    assert_eq!(
        runtime.submit_team_result(team_result()).unwrap().status,
        TeamResultStatus::RejectedChannelClosed
    );
}

#[test]
fn runtime_source_has_no_provider_tool_memory_or_query_execution_entrypoints() {
    let symbols = exported_runtime_symbols();
    for word in [
        "provider_live_call",
        "http",
        "sdk",
        "shell_execute",
        "file_execute",
        "browser_execute",
        "mcp_execute",
        "memory_write",
        "memory_propose",
        "query",
        "event_query",
        "session_ledger_write",
        "agent_runtime_attach",
    ] {
        assert!(
            !symbols.iter().any(|symbol| symbol.contains(word)),
            "unexpected runtime symbol: {word}"
        );
    }
}
