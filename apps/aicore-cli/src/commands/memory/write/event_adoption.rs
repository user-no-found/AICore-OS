use aicore_event::{
    EventEnvelope, EventStatus, EventTag, EventTagSet, EventVisibility, EventWriter, ReplayPolicy,
    RetentionClass,
};
use aicore_event_sqlite::SqliteEventStore;
use aicore_foundation::{AicoreClock, ComponentId, EventId, InstanceId, SystemClock};

use crate::config_store::real_event_store_db_path;

const SOURCE_COMPONENT: &str = "aicore-memory";
const REDACTION_LEVEL: &str = "summary";

pub(crate) enum MemoryBusinessEventKind {
    Remembered,
    ProposalAccepted,
    ProposalRejected,
}

pub(crate) struct EventRecordingOutcome {
    recorded: bool,
    write_status: &'static str,
    event_id: Option<String>,
    error_code: Option<&'static str>,
}

impl EventRecordingOutcome {
    pub(crate) fn apply_to_fields(&self, fields: &mut serde_json::Value) {
        let object = fields
            .as_object_mut()
            .expect("memory write fields should be object");
        object.insert(
            "event_recorded".to_string(),
            serde_json::Value::String(self.recorded.to_string()),
        );
        object.insert(
            "event_write_status".to_string(),
            serde_json::Value::String(self.write_status.to_string()),
        );
        if let Some(event_id) = &self.event_id {
            object.insert(
                "event_id".to_string(),
                serde_json::Value::String(event_id.clone()),
            );
        }
        if let Some(error_code) = self.error_code {
            object.insert(
                "event_error_code".to_string(),
                serde_json::Value::String(error_code.to_string()),
            );
        }
    }
}

pub(crate) fn skipped_event_recording() -> EventRecordingOutcome {
    EventRecordingOutcome {
        recorded: false,
        write_status: "skipped",
        event_id: None,
        error_code: None,
    }
}

pub(crate) fn record_memory_business_event(
    kind: MemoryBusinessEventKind,
    subject_id: &str,
) -> EventRecordingOutcome {
    let event_id = match EventId::new(format!(
        "evt.{}.{}",
        kind.event_type(),
        sanitize_subject_token(subject_id)
    )) {
        Ok(event_id) => event_id,
        Err(_) => return failed_outcome("event_id_invalid"),
    };

    let event_store_path = match real_event_store_db_path() {
        Ok(path) => path,
        Err(_) => return failed_outcome("event_store_path_failed"),
    };

    let store = match SqliteEventStore::open(&event_store_path, &InstanceId::global_main()) {
        Ok(store) => store,
        Err(_) => return failed_outcome("event_store_open_failed"),
    };

    let envelope = match build_envelope(&kind, subject_id, &event_id) {
        Ok(envelope) => envelope,
        Err(_) => return failed_outcome("event_build_failed"),
    };

    match store.write(&envelope) {
        Ok(()) => EventRecordingOutcome {
            recorded: true,
            write_status: "recorded",
            event_id: Some(event_id.as_str().to_string()),
            error_code: None,
        },
        Err(_) => failed_outcome("event_store_write_failed"),
    }
}

fn build_envelope(
    kind: &MemoryBusinessEventKind,
    subject_id: &str,
    event_id: &EventId,
) -> Result<EventEnvelope, aicore_foundation::AicoreError> {
    let tags = EventTagSet::new()
        .with_tag(EventTag::new("source:memory")?)
        .with_tag(EventTag::new(kind.operation_tag())?)
        .with_tag(EventTag::new("event:business")?)
        .with_tag(EventTag::new("retention:transient_30d")?);

    EventEnvelope::builder(
        event_id.clone(),
        kind.event_type(),
        SystemClock.now(),
        ComponentId::new(SOURCE_COMPONENT)?,
        InstanceId::global_main(),
        kind.subject_type(),
        subject_id.to_string(),
        kind.summary(),
        RetentionClass::Transient30d,
    )
    .tag_set(tags)
    .redaction_level(REDACTION_LEVEL)
    .visibility(EventVisibility::Instance)
    .status(EventStatus::Recorded)
    .replay_policy(ReplayPolicy::HistoryOnly)
    .build()
}

fn sanitize_subject_token(value: &str) -> String {
    let token: String = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_') {
                ch
            } else {
                '_'
            }
        })
        .take(96)
        .collect();

    if token.is_empty() {
        "unknown".to_string()
    } else {
        token
    }
}

fn failed_outcome(error_code: &'static str) -> EventRecordingOutcome {
    EventRecordingOutcome {
        recorded: false,
        write_status: "failed",
        event_id: None,
        error_code: Some(error_code),
    }
}

impl MemoryBusinessEventKind {
    fn event_type(&self) -> &'static str {
        match self {
            Self::Remembered => "memory.remembered",
            Self::ProposalAccepted => "memory.proposal.accepted",
            Self::ProposalRejected => "memory.proposal.rejected",
        }
    }

    fn subject_type(&self) -> &'static str {
        match self {
            Self::Remembered => "memory_record",
            Self::ProposalAccepted | Self::ProposalRejected => "memory_proposal",
        }
    }

    fn summary(&self) -> &'static str {
        match self {
            Self::Remembered => "memory record created",
            Self::ProposalAccepted => "memory proposal accepted",
            Self::ProposalRejected => "memory proposal rejected",
        }
    }

    fn operation_tag(&self) -> &'static str {
        match self {
            Self::Remembered => "operation:memory.remember",
            Self::ProposalAccepted => "operation:memory.accept",
            Self::ProposalRejected => "operation:memory.reject",
        }
    }
}
