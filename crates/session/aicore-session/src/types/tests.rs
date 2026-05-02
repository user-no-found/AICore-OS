use super::*;
use aicore_foundation::{InstanceId, SessionId, Timestamp};

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
    assert_eq!(TurnId::new("turn.001").unwrap().as_str(), "turn.001");
    assert_eq!(MessageId::new("msg-001").unwrap().as_str(), "msg-001");
    assert_eq!(
        ApprovalId::new("approval_001").unwrap().as_str(),
        "approval_001"
    );
    assert_eq!(
        PendingInputId::new("pending.001").unwrap().as_str(),
        "pending.001"
    );
    assert_eq!(
        ControlEventId::new("event.001").unwrap().as_str(),
        "event.001"
    );
    assert_eq!(
        LedgerWriteId::new("write.001").unwrap().as_str(),
        "write.001"
    );

    assert!(TurnId::new("bad/id").is_err());
    assert!(MessageId::new("").is_err());
}

#[test]
fn enums_serialize_as_snake_case_contract_values() {
    assert_eq!(
        serde_json::to_string(&MessageRole::Assistant).unwrap(),
        "\"assistant\""
    );
    assert_eq!(
        serde_json::to_string(&MessageKind::AssistantDelta).unwrap(),
        "\"assistant_delta\""
    );
    assert_eq!(
        serde_json::to_string(&MessageKind::AssistantFinal).unwrap(),
        "\"assistant_final\""
    );
    assert_eq!(
        serde_json::to_string(&ControlEventKind::RuntimeStateUpdated).unwrap(),
        "\"runtime_state_updated\""
    );
    assert_eq!(
        serde_json::to_string(&LedgerWriteKind::Insert).unwrap(),
        "\"insert\""
    );
    assert_eq!(
        serde_json::to_string(&ApprovalStatus::Stale).unwrap(),
        "\"stale\""
    );
    assert_eq!(
        serde_json::to_string(&PendingInputStatus::Replaced).unwrap(),
        "\"replaced\""
    );
    assert_eq!(
        serde_json::to_string(&ActiveTurnAcquireStatus::AlreadyActive).unwrap(),
        "\"already_active\""
    );
    assert_eq!(
        serde_json::to_string(&StopTurnStatus::NoActiveTurn).unwrap(),
        "\"no_active_turn\""
    );
    assert_eq!(
        serde_json::to_string(&ApprovalResponseStatus::RejectedAlreadyResolved).unwrap(),
        "\"rejected_already_resolved\""
    );
    assert_eq!(
        serde_json::to_string(&ApprovalScope::SingleToolCall).unwrap(),
        "\"single_tool_call\""
    );
    assert_eq!(TurnStatus::Stopped.as_str(), "stopped");
    assert_eq!(TurnStatus::Failed.as_str(), "failed");
    assert_eq!(
        ApprovalStatus::InvalidatedByStop.as_str(),
        "invalidated_by_stop"
    );
}

#[test]
fn records_round_trip_through_serde() {
    let record = MessageRecord {
        message_id: "msg.001".to_string(),
        session_id: "sess.001".to_string(),
        turn_id: Some("turn.001".to_string()),
        message_seq: 1,
        kind: MessageKind::User,
        content: "hello".to_string(),
        created_at: 10,
        metadata: Some(serde_json::json!({ "summary": "safe" })),
    };

    let encoded = serde_json::to_string(&record).unwrap();
    let decoded: MessageRecord = serde_json::from_str(&encoded).unwrap();

    assert_eq!(decoded, record);
}

#[test]
fn requests_use_foundation_id_and_timestamp_types() {
    let request = AppendMessageRequest {
        instance_id: InstanceId::global_main(),
        session_id: SessionId::new("sess.001").unwrap(),
        turn_id: Some("turn.001".to_string()),
        message_id: "msg.001".to_string(),
        message_seq: 1,
        kind: MessageKind::User,
        content: "hello".to_string(),
        created_at: Timestamp::from_unix_millis(42),
        metadata: None,
    };

    assert_eq!(request.instance_id.as_str(), "global-main");
    assert_eq!(request.session_id.as_str(), "sess.001");
    assert_eq!(request.created_at.unix_millis(), 42);
}

#[test]
fn active_turn_and_stop_contracts_round_trip() {
    let acquire = ActiveTurnAcquireRequest {
        instance_id: InstanceId::global_main(),
        session_id: SessionId::new("sess.ctl").unwrap(),
        turn_id: "turn.ctl".to_string(),
        requested_at: Timestamp::from_unix_millis(10),
    };
    let encoded = serde_json::to_string(&acquire).unwrap();
    let decoded: ActiveTurnAcquireRequest = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded.turn_id, "turn.ctl");

    let outcome = ActiveTurnAcquireOutcome {
        status: ActiveTurnAcquireStatus::Acquired,
        active_turn_id: Some("turn.ctl".to_string()),
        lock_version: 1,
    };
    let encoded = serde_json::to_string(&outcome).unwrap();
    let decoded: ActiveTurnAcquireOutcome = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded.status, ActiveTurnAcquireStatus::Acquired);
    assert_eq!(decoded.lock_version, 1);

    let stop = StopTurnRequest {
        instance_id: InstanceId::global_main(),
        requested_at: Timestamp::from_unix_millis(11),
    };
    let encoded = serde_json::to_string(&stop).unwrap();
    let decoded: StopTurnRequest = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded.instance_id.as_str(), "global-main");
}

#[test]
fn pending_and_approval_contracts_round_trip() {
    let pending = PendingInputRecord {
        pending_input_id: "pending.001".to_string(),
        instance_id: "global-main".to_string(),
        session_id: Some("sess.001".to_string()),
        turn_id: Some("turn.001".to_string()),
        content: "safe user text".to_string(),
        status: PendingInputStatus::Pending,
        created_at: 10,
        updated_at: 10,
    };
    let encoded = serde_json::to_string(&pending).unwrap();
    let decoded: PendingInputRecord = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded, pending);

    let approval = ApprovalRecord {
        approval_id: "approval.001".to_string(),
        instance_id: "global-main".to_string(),
        turn_id: "turn.001".to_string(),
        status: ApprovalStatus::Pending,
        scope: ApprovalScope::SingleToolCall,
        summary: "safe approval summary".to_string(),
        created_at: 20,
        resolved_at: None,
        resolved_response_id: None,
    };
    let encoded = serde_json::to_string(&approval).unwrap();
    let decoded: ApprovalRecord = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded, approval);

    let response = ApprovalResponseRecord {
        response_id: "approval.response.001".to_string(),
        approval_id: "approval.001".to_string(),
        instance_id: "global-main".to_string(),
        decision: ApprovalDecision::Approve,
        status: ApprovalResponseStatus::Accepted,
        responder_client_id: Some("client.001".to_string()),
        responder_client_kind: Some("tui".to_string()),
        responded_at: 30,
    };
    let encoded = serde_json::to_string(&response).unwrap();
    let decoded: ApprovalResponseRecord = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded, response);
}

#[test]
fn public_structures_do_not_expose_forbidden_raw_fields() {
    let public_names = [
        "SessionRecord session_id title status created_at updated_at metadata",
        "TurnRecord turn_id session_id turn_seq status started_at finished_at",
        "MessageRecord message_id session_id turn_id message_seq kind content created_at metadata",
        "ControlEventRecord event_id instance_id turn_id event_seq event_type detail created_at",
        "LedgerWriteRecord write_id instance_id turn_id write_seq write_type target_table target_id created_at",
        "InstanceRuntimeState instance_id active_session_id active_turn_id pending_input_id pending_approval_id last_message_seq last_control_event_seq last_write_seq runtime_status dirty_shutdown recovery_required updated_at",
        "PendingInputRecord pending_input_id instance_id session_id turn_id content status created_at updated_at",
        "ApprovalRecord approval_id instance_id turn_id status scope summary created_at resolved_at resolved_response_id",
        "ApprovalResponseRecord response_id approval_id instance_id decision status responder_client_id responder_client_kind responded_at",
    ]
    .join("\n");

    for forbidden in FORBIDDEN_FIELDS {
        assert!(
            !public_names.contains(forbidden),
            "forbidden field leaked: {forbidden}"
        );
    }
}
