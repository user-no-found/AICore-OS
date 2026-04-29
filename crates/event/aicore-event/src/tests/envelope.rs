use aicore_foundation::{ComponentId, EventId, InstanceId, InvocationId, Timestamp};

use crate::EventEnvelope;
use crate::lifecycle::{EventStatus, EventVisibility, ReplayPolicy, RetentionClass};

#[test]
fn builder_creates_valid_envelope() {
    let envelope = EventEnvelope::builder(
        EventId::new("evt.test.01").unwrap(),
        "memory.remembered",
        Timestamp::from_unix_millis(1_700_000_000_000),
        ComponentId::new("aicore-memory").unwrap(),
        InstanceId::global_main(),
        "memory",
        "mem.01",
        "remembered user preference",
        RetentionClass::Durable,
    )
    .build()
    .unwrap();

    assert_eq!(envelope.event_type, "memory.remembered");
    assert_eq!(envelope.schema_version, "1.0");
    assert_eq!(envelope.summary, "remembered user preference");
    assert_eq!(envelope.retention_class, RetentionClass::Durable);
}

#[test]
fn builder_with_optional_fields() {
    let envelope = EventEnvelope::builder(
        EventId::new("evt.test.02").unwrap(),
        "error.recorded",
        Timestamp::from_unix_millis(1_700_000_000_000),
        ComponentId::new("aicore-cli").unwrap(),
        InstanceId::new("ws.dev").unwrap(),
        "error",
        "err.01",
        "validation failed",
        RetentionClass::Transient30d,
    )
    .schema_version("1.1")
    .correlation_id("corr.01")
    .visibility(EventVisibility::System)
    .replay_policy(ReplayPolicy::NotReplayable)
    .status(EventStatus::Recorded)
    .build()
    .unwrap();

    assert_eq!(envelope.schema_version, "1.1");
    assert_eq!(envelope.correlation_id, Some("corr.01".to_string()));
    assert_eq!(envelope.visibility, Some(EventVisibility::System));
    assert_eq!(envelope.replay_policy, Some(ReplayPolicy::NotReplayable));
    assert_eq!(envelope.status, Some(EventStatus::Recorded));
}

#[test]
fn rejects_empty_event_type() {
    let result = EventEnvelope::builder(
        EventId::new("evt.test.03").unwrap(),
        "",
        Timestamp::from_unix_millis(1_700_000_000_000),
        ComponentId::new("aicore-memory").unwrap(),
        InstanceId::global_main(),
        "memory",
        "mem.01",
        "summary",
        RetentionClass::Durable,
    )
    .build();

    assert!(result.is_err());
}

#[test]
fn rejects_empty_summary() {
    let result = EventEnvelope::builder(
        EventId::new("evt.test.04").unwrap(),
        "memory.remembered",
        Timestamp::from_unix_millis(1_700_000_000_000),
        ComponentId::new("aicore-memory").unwrap(),
        InstanceId::global_main(),
        "memory",
        "mem.01",
        "",
        RetentionClass::Durable,
    )
    .build();

    assert!(result.is_err());
}

#[test]
fn serialization_omits_empty_optionals() {
    let envelope = EventEnvelope::builder(
        EventId::new("evt.test.05").unwrap(),
        "memory.remembered",
        Timestamp::from_unix_millis(1_700_000_000_000),
        ComponentId::new("aicore-memory").unwrap(),
        InstanceId::global_main(),
        "memory",
        "mem.01",
        "remembered preference",
        RetentionClass::Durable,
    )
    .build()
    .unwrap();

    let json = serde_json::to_string_pretty(&envelope).unwrap();
    assert!(!json.contains("correlation_id"));
    assert!(!json.contains("causation_id"));
    assert!(json.contains("event_id"));
    assert!(json.contains("event_type"));
}

#[test]
fn deserialization_roundtrip() {
    let envelope = EventEnvelope::builder(
        EventId::new("evt.test.06").unwrap(),
        "task.status_changed",
        Timestamp::from_unix_millis(1_700_000_000_000),
        ComponentId::new("aicore-agent").unwrap(),
        InstanceId::global_main(),
        "task",
        "task.01",
        "task moved to done",
        RetentionClass::Summary180d,
    )
    .correlation_id("corr.02")
    .replay_policy(ReplayPolicy::Replayable)
    .build()
    .unwrap();

    let json = serde_json::to_string(&envelope).unwrap();
    let back: EventEnvelope = serde_json::from_str(&json).unwrap();
    assert_eq!(back.event_id.as_str(), envelope.event_id.as_str());
    assert_eq!(back.event_type, envelope.event_type);
    assert_eq!(back.correlation_id, envelope.correlation_id);
    assert_eq!(back.replay_policy, envelope.replay_policy);
}

#[test]
fn json_does_not_contain_raw_forbidden_fields() {
    let envelope = EventEnvelope::builder(
        EventId::new("evt.security.01").unwrap(),
        "provider.request.completed",
        Timestamp::from_unix_millis(1_700_000_000_000),
        ComponentId::new("aicore-provider").unwrap(),
        InstanceId::global_main(),
        "provider_request",
        "req.01",
        "provider request completed",
        RetentionClass::Transient30d,
    )
    .correlation_id("corr.sec.01")
    .evidence_ref("evidence://requests/req.01")
    .replay_policy(ReplayPolicy::HistoryOnly)
    .build()
    .unwrap();

    let json = serde_json::to_string_pretty(&envelope).unwrap();

    let forbidden_keys = [
        "raw_stdout",
        "raw_stderr",
        "raw_payload",
        "raw_memory_content",
        "raw_provider_request",
        "raw_provider_response",
        "raw_tool_input",
        "raw_tool_output",
        "secret",
        "secret_ref",
        "token",
        "api_key",
        "cookie",
    ];

    for key in forbidden_keys {
        assert!(
            !json.contains(&format!("\"{key}\"")),
            "serialized EventEnvelope must not contain forbidden key: {key}"
        );
    }
}
