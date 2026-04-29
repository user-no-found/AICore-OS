use aicore_foundation::{ComponentId, EventId, InstanceId, InvocationId, Timestamp};
use serde::{Deserialize, Serialize};

use crate::lifecycle::{EventStatus, EventVisibility, ReplayPolicy, RetentionClass};
use crate::tag::EventTagSet;
use crate::validation;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventEnvelope {
    pub event_id: EventId,
    pub event_type: String,
    pub schema_version: String,
    pub occurred_at: Timestamp,
    pub recorded_at: Timestamp,
    pub source_component: ComponentId,
    pub source_instance: InstanceId,
    pub subject_type: String,
    pub subject_id: String,
    pub summary: String,
    pub retention_class: RetentionClass,

    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub correlation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub causation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub invocation_id: Option<InvocationId>,
    #[serde(skip_serializing_if = "EventTagSet::is_empty", default)]
    pub tag_set: EventTagSet,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub evidence_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub payload_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub redaction_level: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub visibility: Option<EventVisibility>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub status: Option<EventStatus>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub replay_policy: Option<ReplayPolicy>,
}

impl EventEnvelope {
    pub fn builder(
        event_id: EventId,
        event_type: impl Into<String>,
        occurred_at: Timestamp,
        source_component: ComponentId,
        source_instance: InstanceId,
        subject_type: impl Into<String>,
        subject_id: impl Into<String>,
        summary: impl Into<String>,
        retention_class: RetentionClass,
    ) -> EventEnvelopeBuilder {
        EventEnvelopeBuilder::new(
            event_id,
            event_type,
            occurred_at,
            source_component,
            source_instance,
            subject_type,
            subject_id,
            summary,
            retention_class,
        )
    }

    pub fn validate(&self) -> Result<(), aicore_foundation::AicoreError> {
        validation::validate_event_type(&self.event_type)?;
        validation::validate_schema_version(&self.schema_version)?;
        validation::validate_summary(&self.summary)?;
        validation::validate_subject_type(&self.subject_type)?;
        validation::validate_subject_id(&self.subject_id)?;
        if let Some(ref r) = self.evidence_ref {
            validation::validate_ref(r, "evidence_ref")?;
        }
        if let Some(ref r) = self.payload_ref {
            validation::validate_ref(r, "payload_ref")?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct EventEnvelopeBuilder {
    event_id: EventId,
    event_type: String,
    schema_version: String,
    occurred_at: Timestamp,
    recorded_at: Timestamp,
    source_component: ComponentId,
    source_instance: InstanceId,
    subject_type: String,
    subject_id: String,
    summary: String,
    retention_class: RetentionClass,
    correlation_id: Option<String>,
    causation_id: Option<String>,
    invocation_id: Option<InvocationId>,
    tag_set: EventTagSet,
    evidence_ref: Option<String>,
    payload_ref: Option<String>,
    redaction_level: Option<String>,
    visibility: Option<EventVisibility>,
    status: Option<EventStatus>,
    replay_policy: Option<ReplayPolicy>,
}

impl EventEnvelopeBuilder {
    pub fn new(
        event_id: EventId,
        event_type: impl Into<String>,
        occurred_at: Timestamp,
        source_component: ComponentId,
        source_instance: InstanceId,
        subject_type: impl Into<String>,
        subject_id: impl Into<String>,
        summary: impl Into<String>,
        retention_class: RetentionClass,
    ) -> Self {
        Self {
            event_id,
            event_type: event_type.into(),
            schema_version: "1.0".to_string(),
            occurred_at,
            recorded_at: occurred_at,
            source_component,
            source_instance,
            subject_type: subject_type.into(),
            subject_id: subject_id.into(),
            summary: summary.into(),
            retention_class,
            correlation_id: None,
            causation_id: None,
            invocation_id: None,
            tag_set: EventTagSet::new(),
            evidence_ref: None,
            payload_ref: None,
            redaction_level: None,
            visibility: None,
            status: None,
            replay_policy: None,
        }
    }

    pub fn schema_version(mut self, value: impl Into<String>) -> Self {
        self.schema_version = value.into();
        self
    }

    pub fn recorded_at(mut self, value: Timestamp) -> Self {
        self.recorded_at = value;
        self
    }

    pub fn correlation_id(mut self, value: impl Into<String>) -> Self {
        self.correlation_id = Some(value.into());
        self
    }

    pub fn causation_id(mut self, value: impl Into<String>) -> Self {
        self.causation_id = Some(value.into());
        self
    }

    pub fn invocation_id(mut self, value: InvocationId) -> Self {
        self.invocation_id = Some(value);
        self
    }

    pub fn tag_set(mut self, value: EventTagSet) -> Self {
        self.tag_set = value;
        self
    }

    pub fn evidence_ref(mut self, value: impl Into<String>) -> Self {
        self.evidence_ref = Some(value.into());
        self
    }

    pub fn payload_ref(mut self, value: impl Into<String>) -> Self {
        self.payload_ref = Some(value.into());
        self
    }

    pub fn redaction_level(mut self, value: impl Into<String>) -> Self {
        self.redaction_level = Some(value.into());
        self
    }

    pub fn visibility(mut self, value: EventVisibility) -> Self {
        self.visibility = Some(value);
        self
    }

    pub fn status(mut self, value: EventStatus) -> Self {
        self.status = Some(value);
        self
    }

    pub fn replay_policy(mut self, value: ReplayPolicy) -> Self {
        self.replay_policy = Some(value);
        self
    }

    pub fn build(self) -> Result<EventEnvelope, aicore_foundation::AicoreError> {
        let envelope = EventEnvelope {
            event_id: self.event_id,
            event_type: self.event_type,
            schema_version: self.schema_version,
            occurred_at: self.occurred_at,
            recorded_at: self.recorded_at,
            source_component: self.source_component,
            source_instance: self.source_instance,
            subject_type: self.subject_type,
            subject_id: self.subject_id,
            summary: self.summary,
            retention_class: self.retention_class,
            correlation_id: self.correlation_id,
            causation_id: self.causation_id,
            invocation_id: self.invocation_id,
            tag_set: self.tag_set,
            evidence_ref: self.evidence_ref,
            payload_ref: self.payload_ref,
            redaction_level: self.redaction_level,
            visibility: self.visibility,
            status: self.status,
            replay_policy: self.replay_policy,
        };
        envelope.validate()?;
        Ok(envelope)
    }
}
