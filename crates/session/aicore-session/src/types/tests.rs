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
fn public_structures_do_not_expose_forbidden_raw_fields() {
    let public_names = [
        "SessionRecord session_id title status created_at updated_at metadata",
        "TurnRecord turn_id session_id turn_seq status started_at finished_at",
        "MessageRecord message_id session_id turn_id message_seq kind content created_at metadata",
        "ControlEventRecord event_id instance_id turn_id event_seq event_type detail created_at",
        "LedgerWriteRecord write_id instance_id turn_id write_seq write_type target_table target_id created_at",
        "InstanceRuntimeState instance_id active_session_id active_turn_id pending_input_id pending_approval_id last_message_seq last_control_event_seq last_write_seq runtime_status dirty_shutdown recovery_required updated_at",
    ]
    .join("\n");

    for forbidden in FORBIDDEN_FIELDS {
        assert!(
            !public_names.contains(forbidden),
            "forbidden field leaked: {forbidden}"
        );
    }
}
