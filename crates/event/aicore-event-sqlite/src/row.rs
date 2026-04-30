use aicore_event::{
    EventEnvelope, EventStatus, EventTag, EventTagSet, EventVisibility, ReplayPolicy,
    RetentionClass,
};
use aicore_foundation::{AicoreError, ComponentId, EventId, InstanceId, InvocationId, Timestamp};

pub struct EventRow {
    pub event_id: String,
    pub event_type: String,
    pub schema_version: String,
    pub occurred_at: String,
    pub recorded_at: String,
    pub source_component: String,
    pub source_instance: String,
    pub subject_type: String,
    pub subject_id: String,
    pub summary: String,
    pub retention_class: String,
    pub correlation_id: Option<String>,
    pub causation_id: Option<String>,
    pub invocation_id: Option<String>,
    pub redaction_level: Option<String>,
    pub visibility: Option<String>,
    pub status: Option<String>,
    pub replay_policy: Option<String>,
}

pub fn event_row_from_envelope(envelope: &EventEnvelope) -> EventRow {
    EventRow {
        event_id: envelope.event_id.as_str().to_string(),
        event_type: envelope.event_type.clone(),
        schema_version: envelope.schema_version.clone(),
        occurred_at: timestamp_to_text(envelope.occurred_at),
        recorded_at: timestamp_to_text(envelope.recorded_at),
        source_component: envelope.source_component.as_str().to_string(),
        source_instance: envelope.source_instance.as_str().to_string(),
        subject_type: envelope.subject_type.clone(),
        subject_id: envelope.subject_id.clone(),
        summary: envelope.summary.clone(),
        retention_class: retention_class_to_text(&envelope.retention_class).to_string(),
        correlation_id: envelope.correlation_id.clone(),
        causation_id: envelope.causation_id.clone(),
        invocation_id: envelope
            .invocation_id
            .as_ref()
            .map(|value| value.as_str().to_string()),
        redaction_level: envelope.redaction_level.clone(),
        visibility: envelope.visibility.as_ref().map(visibility_to_text),
        status: envelope.status.as_ref().map(status_to_text),
        replay_policy: envelope.replay_policy.as_ref().map(replay_policy_to_text),
    }
}

pub fn event_from_row(
    row: EventRow,
    evidence_ref: Option<String>,
    payload_ref: Option<String>,
    tags: Vec<String>,
    confirmed_tags: Vec<String>,
) -> Result<EventEnvelope, AicoreError> {
    let mut tag_set = EventTagSet::new();
    for tag in tags {
        tag_set = tag_set.with_tag(EventTag::new(tag)?);
    }
    for tag in confirmed_tags {
        tag_set = tag_set.with_confirmed(EventTag::new(tag)?);
    }

    Ok(EventEnvelope {
        event_id: EventId::new(row.event_id)?,
        event_type: row.event_type,
        schema_version: row.schema_version,
        occurred_at: timestamp_from_text(&row.occurred_at)?,
        recorded_at: timestamp_from_text(&row.recorded_at)?,
        source_component: ComponentId::new(row.source_component)?,
        source_instance: InstanceId::new(row.source_instance)?,
        subject_type: row.subject_type,
        subject_id: row.subject_id,
        summary: row.summary,
        retention_class: retention_class_from_text(&row.retention_class)?,
        correlation_id: row.correlation_id,
        causation_id: row.causation_id,
        invocation_id: row.invocation_id.map(InvocationId::new).transpose()?,
        tag_set,
        evidence_ref,
        payload_ref,
        redaction_level: row.redaction_level,
        visibility: row
            .visibility
            .as_deref()
            .map(visibility_from_text)
            .transpose()?,
        status: row.status.as_deref().map(status_from_text).transpose()?,
        replay_policy: row
            .replay_policy
            .as_deref()
            .map(replay_policy_from_text)
            .transpose()?,
    })
}

pub fn timestamp_to_text(value: Timestamp) -> String {
    value.unix_millis().to_string()
}

fn timestamp_from_text(value: &str) -> Result<Timestamp, AicoreError> {
    let millis = value.parse::<u128>().map_err(|_| {
        AicoreError::InvalidState(format!("invalid timestamp text in sqlite row: {value}"))
    })?;
    Ok(Timestamp::from_unix_millis(millis))
}

fn retention_class_to_text(value: &RetentionClass) -> &'static str {
    match value {
        RetentionClass::Ephemeral => "ephemeral",
        RetentionClass::Transient30d => "transient_30d",
        RetentionClass::Summary180d => "summary_180d",
        RetentionClass::Durable => "durable",
        RetentionClass::AuditPinned => "audit_pinned",
        RetentionClass::NeedsReview => "needs_review",
        RetentionClass::Invalid => "invalid",
    }
}

fn retention_class_from_text(value: &str) -> Result<RetentionClass, AicoreError> {
    match value {
        "ephemeral" => Ok(RetentionClass::Ephemeral),
        "transient_30d" => Ok(RetentionClass::Transient30d),
        "summary_180d" => Ok(RetentionClass::Summary180d),
        "durable" => Ok(RetentionClass::Durable),
        "audit_pinned" => Ok(RetentionClass::AuditPinned),
        "needs_review" => Ok(RetentionClass::NeedsReview),
        "invalid" => Ok(RetentionClass::Invalid),
        _ => Err(AicoreError::InvalidState(format!(
            "unknown retention_class in sqlite row: {value}"
        ))),
    }
}

fn visibility_to_text(value: &EventVisibility) -> String {
    match value {
        EventVisibility::System => "system",
        EventVisibility::User => "user",
        EventVisibility::Instance => "instance",
        EventVisibility::GlobalMain => "global_main",
    }
    .to_string()
}

fn visibility_from_text(value: &str) -> Result<EventVisibility, AicoreError> {
    match value {
        "system" => Ok(EventVisibility::System),
        "user" => Ok(EventVisibility::User),
        "instance" => Ok(EventVisibility::Instance),
        "global_main" => Ok(EventVisibility::GlobalMain),
        _ => Err(AicoreError::InvalidState(format!(
            "unknown visibility in sqlite row: {value}"
        ))),
    }
}

fn status_to_text(value: &EventStatus) -> String {
    match value {
        EventStatus::Recorded => "recorded",
        EventStatus::Compressed => "compressed",
        EventStatus::Expired => "expired",
        EventStatus::Invalid => "invalid",
    }
    .to_string()
}

fn status_from_text(value: &str) -> Result<EventStatus, AicoreError> {
    match value {
        "recorded" => Ok(EventStatus::Recorded),
        "compressed" => Ok(EventStatus::Compressed),
        "expired" => Ok(EventStatus::Expired),
        "invalid" => Ok(EventStatus::Invalid),
        _ => Err(AicoreError::InvalidState(format!(
            "unknown status in sqlite row: {value}"
        ))),
    }
}

fn replay_policy_to_text(value: &ReplayPolicy) -> String {
    match value {
        ReplayPolicy::Replayable => "replayable",
        ReplayPolicy::HistoryOnly => "history_only",
        ReplayPolicy::NotReplayable => "not_replayable",
    }
    .to_string()
}

fn replay_policy_from_text(value: &str) -> Result<ReplayPolicy, AicoreError> {
    match value {
        "replayable" => Ok(ReplayPolicy::Replayable),
        "history_only" => Ok(ReplayPolicy::HistoryOnly),
        "not_replayable" => Ok(ReplayPolicy::NotReplayable),
        _ => Err(AicoreError::InvalidState(format!(
            "unknown replay_policy in sqlite row: {value}"
        ))),
    }
}
