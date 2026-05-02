use super::*;
use aicore_foundation::{InstanceId, SessionId, Timestamp};
use aicore_session::{ApprovalId, MessageId, PendingInputId, TurnId};

const FORBIDDEN_FIELDS: &[&str] = &[
    "raw_provider_request",
    "raw_provider_response",
    "raw_tool_input",
    "raw_tool_output",
    "raw_stdout",
    "raw_stderr",
    "raw_memory_content",
    "raw_prompt",
    "secret",
    "token",
    "api_key",
    "cookie",
    "credential",
    "authorization",
    "password",
];

#[test]
fn ids_follow_foundation_token_rules() {
    assert_eq!(
        IoClientId::new("client.tui.1").unwrap().as_str(),
        "client.tui.1"
    );
    assert_eq!(IoConnectionId::new("conn-1").unwrap().as_str(), "conn-1");
    assert_eq!(IoRequestId::new("req_1").unwrap().as_str(), "req_1");
    assert_eq!(IoEventId::new("event.1").unwrap().as_str(), "event.1");
    assert_eq!(IoSubscriptionId::new("sub.1").unwrap().as_str(), "sub.1");
    assert!(IoRequestId::new("bad/id").is_err());
}

#[test]
fn stream_cursor_is_opaque() {
    let cursor = IoStreamCursor::new("ledger:turn.1:msg.2").unwrap();
    assert_eq!(cursor.as_str(), "ledger:turn.1:msg.2");
    assert!(cursor.parse_seq().is_none());
    assert!(IoStreamCursor::new("").is_err());
}

#[test]
fn enums_serialize_as_contract_values() {
    assert_eq!(
        serde_json::to_string(&IoClientKind::Tui).unwrap(),
        "\"tui\""
    );
    assert_eq!(
        serde_json::to_string(&IoAttachMode::Observe).unwrap(),
        "\"observe\""
    );
    assert_eq!(
        serde_json::to_string(&IoInputKind::ApprovalResponse).unwrap(),
        "\"approval_response\""
    );
    assert_eq!(
        serde_json::to_string(&IoOutputKind::AssistantDelta).unwrap(),
        "\"assistant_delta\""
    );
    assert_eq!(
        serde_json::to_string(&IoOutputKind::AssistantFinal).unwrap(),
        "\"assistant_final\""
    );
    assert_eq!(
        serde_json::to_string(&IoDeliveryMode::Live).unwrap(),
        "\"live\""
    );
    assert_eq!(
        serde_json::to_string(&IoStreamStatus::StaleCursor).unwrap(),
        "\"stale_cursor\""
    );
}

#[test]
fn bind_attach_detach_requests_carry_ownership_fields() {
    let bind = BindInstanceRequest {
        request_id: IoRequestId::new("req.bind").unwrap(),
        client_kind: IoClientKind::Cli,
        attach_mode: IoAttachMode::Bind,
        instance_id: Some(InstanceId::global_main()),
        workspace_hint: None,
        created_at: Timestamp::from_unix_millis(1),
        correlation_id: Some("corr.1".to_string()),
        causation_id: None,
    };
    let attach = AttachInstanceRequest {
        request_id: IoRequestId::new("req.attach").unwrap(),
        instance_id: InstanceId::global_main(),
        client_id: IoClientId::new("client.cli").unwrap(),
        client_kind: IoClientKind::Cli,
        attach_mode: IoAttachMode::Attach,
        from_cursor: None,
        created_at: Timestamp::from_unix_millis(2),
        correlation_id: Some("corr.1".to_string()),
        causation_id: Some("req.bind".to_string()),
    };
    let detach = DetachInstanceRequest {
        request_id: IoRequestId::new("req.detach").unwrap(),
        instance_id: InstanceId::global_main(),
        client_id: IoClientId::new("client.cli").unwrap(),
        connection_id: IoConnectionId::new("conn.cli").unwrap(),
        created_at: Timestamp::from_unix_millis(3),
        correlation_id: Some("corr.1".to_string()),
        causation_id: Some("req.attach".to_string()),
    };

    assert_eq!(bind.client_kind, IoClientKind::Cli);
    assert_eq!(attach.instance_id.as_str(), "global-main");
    assert_eq!(detach.connection_id.as_str(), "conn.cli");
}

#[test]
fn submit_input_expresses_user_stop_and_approval() {
    let user = input(IoInputKind::UserMessage, Some("hello"));
    let stop = input(IoInputKind::StopRequest, None);
    let approval = input(IoInputKind::ApprovalResponse, Some("approved"));

    assert_eq!(user.input.input_kind, IoInputKind::UserMessage);
    assert_eq!(stop.input.input_kind, IoInputKind::StopRequest);
    assert_eq!(approval.input.input_kind, IoInputKind::ApprovalResponse);
}

#[test]
fn output_event_distinguishes_delta_and_final() {
    let delta = output(IoOutputKind::AssistantDelta, Some("partial"));
    let final_event = output(IoOutputKind::AssistantFinal, Some("done"));

    assert_eq!(delta.output_kind, IoOutputKind::AssistantDelta);
    assert_eq!(final_event.output_kind, IoOutputKind::AssistantFinal);
    assert_ne!(delta.output_kind, final_event.output_kind);
}

#[test]
fn current_snapshot_expresses_turn_pending_and_approval_state() {
    let idle = snapshot(None, None, None);
    let active = snapshot(
        Some(TurnId::new("turn.1").unwrap()),
        Some(VisiblePendingInputSummary {
            pending_input_id: PendingInputId::new("pending.1").unwrap(),
            summary_zh: Some("待确认输入".to_string()),
            created_at: Timestamp::from_unix_millis(5),
            requires_confirmation: true,
        }),
        Some(VisibleApprovalSummary {
            approval_id: ApprovalId::new("approval.1").unwrap(),
            turn_id: Some(TurnId::new("turn.1").unwrap()),
            summary_zh: Some("审批请求".to_string()),
            created_at: Timestamp::from_unix_millis(6),
        }),
    );

    assert!(idle.active_turn_id.is_none());
    assert_eq!(active.active_turn_id.as_ref().unwrap().as_str(), "turn.1");
    assert!(active.pending_input.is_some());
    assert!(active.pending_approval.is_some());
}

#[test]
fn event_envelope_carries_required_protocol_metadata() {
    let envelope = IoEventEnvelope {
        instance_id: InstanceId::global_main(),
        event_id: IoEventId::new("event.1").unwrap(),
        client_id: Some(IoClientId::new("client.tui").unwrap()),
        connection_id: Some(IoConnectionId::new("conn.tui").unwrap()),
        session_id: Some(SessionId::new("sess.1").unwrap()),
        turn_id: Some(TurnId::new("turn.1").unwrap()),
        output: output(IoOutputKind::Snapshot, None),
        cursor: IoStreamCursor::new("cursor.1").unwrap(),
        delivery_mode: IoDeliveryMode::Snapshot,
        created_at: Timestamp::from_unix_millis(7),
        correlation_id: Some("corr.1".to_string()),
        causation_id: Some("req.1".to_string()),
    };

    assert_eq!(envelope.instance_id.as_str(), "global-main");
    assert_eq!(envelope.event_id.as_str(), "event.1");
    assert_eq!(envelope.cursor.as_str(), "cursor.1");
    assert_eq!(envelope.created_at.unix_millis(), 7);
}

#[test]
fn protocol_error_is_structured_and_serializable() {
    let error = IoProtocolError {
        code: IoProtocolErrorCode::StaleCursor,
        message_zh: Some("游标已过期".to_string()),
        summary_en: Some("cursor is stale".to_string()),
        retryable: true,
    };
    let response = SubmitInputResponse {
        request_id: IoRequestId::new("req.error").unwrap(),
        instance_id: InstanceId::global_main(),
        status: IoSubmissionStatus::Rejected,
        receipt: None,
        error: Some(error),
        created_at: Timestamp::from_unix_millis(10),
        correlation_id: Some("corr.1".to_string()),
        causation_id: Some("req.input".to_string()),
    };

    let encoded = serde_json::to_string(&response).unwrap();
    assert!(encoded.contains("stale_cursor"));
    let decoded: SubmitInputResponse = serde_json::from_str(&encoded).unwrap();
    assert_eq!(
        decoded.error.unwrap().code,
        IoProtocolErrorCode::StaleCursor
    );
}

#[test]
fn responses_round_trip_through_serde() {
    let response = AttachInstanceResponse {
        request_id: IoRequestId::new("req.attach").unwrap(),
        instance_id: InstanceId::global_main(),
        client_id: IoClientId::new("client.cli").unwrap(),
        connection_id: IoConnectionId::new("conn.cli").unwrap(),
        subscription_id: IoSubscriptionId::new("sub.cli").unwrap(),
        status: IoClientStatus::Attached,
        snapshot: snapshot(None, None, None),
        cursor: IoStreamCursor::new("cursor.attach").unwrap(),
        created_at: Timestamp::from_unix_millis(8),
        correlation_id: None,
        causation_id: None,
    };

    let encoded = serde_json::to_string(&response).unwrap();
    let decoded: AttachInstanceResponse = serde_json::from_str(&encoded).unwrap();

    assert_eq!(decoded, response);
}

#[test]
fn public_structures_do_not_expose_forbidden_fields() {
    let public_names = [
        "BindInstanceRequest request_id client_kind attach_mode instance_id workspace_hint created_at correlation_id causation_id",
        "AttachInstanceRequest request_id instance_id client_id client_kind attach_mode from_cursor created_at correlation_id causation_id",
        "SubmitInputRequest request_id instance_id client_id connection_id session_id turn_id input created_at correlation_id causation_id",
        "IoInputEnvelope input_id input_kind content summary_zh metadata redaction_applied",
        "IoOutputEvent output_kind message_id session_id turn_id content summary_zh summary_en metadata redaction_applied",
        "IoEventEnvelope instance_id event_id client_id connection_id session_id turn_id output cursor delivery_mode created_at correlation_id causation_id",
        "CurrentSnapshot instance_id session_id active_turn_id clients pending_input pending_approval recent_message_cursor stream_status recovery_notice",
    ]
    .join("\n");

    for forbidden in FORBIDDEN_FIELDS {
        assert!(
            !public_names.contains(forbidden),
            "forbidden field leaked: {forbidden}"
        );
    }
}

#[test]
fn contract_has_no_query_or_execution_type_names() {
    let public_names = [
        "BindInstanceRequest AttachInstanceRequest DetachInstanceRequest SubmitInputRequest StopTurnRequest",
        "AcknowledgeEventRequest GetCurrentSnapshotRequest InstanceIoGateway InstanceIoReader InstanceIoWriter",
        "IoInputEnvelope IoOutputEvent IoEventEnvelope CurrentSnapshot VisibleClientSummary",
    ]
    .join(" ");

    for forbidden in [
        "Query",
        "EventQuery",
        "ProviderRuntime",
        "ToolRuntime",
        "TeamRuntime",
        "MemoryProposalRuntime",
    ] {
        assert!(
            !public_names.contains(forbidden),
            "unexpected execution/query type: {forbidden}"
        );
    }
}

fn input(kind: IoInputKind, content: Option<&str>) -> SubmitInputRequest {
    SubmitInputRequest {
        request_id: IoRequestId::new("req.input").unwrap(),
        instance_id: InstanceId::global_main(),
        client_id: IoClientId::new("client.cli").unwrap(),
        connection_id: IoConnectionId::new("conn.cli").unwrap(),
        session_id: Some(SessionId::new("sess.1").unwrap()),
        turn_id: None,
        input: IoInputEnvelope {
            input_id: IoRequestId::new("input.1").unwrap(),
            input_kind: kind,
            content: content.map(str::to_string),
            summary_zh: None,
            metadata: None,
            redaction_applied: false,
        },
        created_at: Timestamp::from_unix_millis(4),
        correlation_id: Some("corr.1".to_string()),
        causation_id: None,
    }
}

fn output(kind: IoOutputKind, content: Option<&str>) -> IoOutputEvent {
    IoOutputEvent {
        output_kind: kind,
        message_id: Some(MessageId::new("msg.1").unwrap()),
        session_id: Some(SessionId::new("sess.1").unwrap()),
        turn_id: Some(TurnId::new("turn.1").unwrap()),
        content: content.map(str::to_string),
        summary_zh: None,
        summary_en: None,
        metadata: None,
        redaction_applied: false,
    }
}

fn snapshot(
    active_turn_id: Option<TurnId>,
    pending_input: Option<VisiblePendingInputSummary>,
    pending_approval: Option<VisibleApprovalSummary>,
) -> CurrentSnapshot {
    CurrentSnapshot {
        instance_id: InstanceId::global_main(),
        session_id: Some(SessionId::new("sess.1").unwrap()),
        active_turn_id,
        visible_turns: vec![VisibleTurnSummary {
            turn_id: TurnId::new("turn.1").unwrap(),
            status: "active".to_string(),
            summary_zh: None,
            updated_at: Timestamp::from_unix_millis(9),
        }],
        connected_clients: vec![VisibleClientSummary {
            client_id: IoClientId::new("client.cli").unwrap(),
            client_kind: IoClientKind::Cli,
            status: IoClientStatus::Attached,
            connected_at: Timestamp::from_unix_millis(1),
            last_seen_at: Timestamp::from_unix_millis(9),
        }],
        pending_input,
        pending_approval,
        recent_message_cursor: Some(IoStreamCursor::new("cursor.1").unwrap()),
        stream_status: IoStreamStatus::Open,
        recovery_notice: None,
    }
}
