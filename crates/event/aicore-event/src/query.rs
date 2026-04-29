use aicore_foundation::{AicoreError, AicoreResult, InstanceId, Timestamp};
use serde::{Deserialize, Serialize};

use crate::envelope::EventEnvelope;
use crate::lifecycle::RetentionClass;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct EventQueryRequest {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub instance_id: Option<InstanceId>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub event_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub subject_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub subject_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub source_component: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub correlation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub retention_class: Option<RetentionClass>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub occurred_after: Option<Timestamp>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub occurred_before: Option<Timestamp>,

    pub summary_only: bool,
    pub limit: u32,

    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cursor: Option<String>,
}

impl EventQueryRequest {
    pub fn new() -> Self {
        Self {
            instance_id: None,
            event_type: None,
            subject_type: None,
            subject_id: None,
            source_component: None,
            correlation_id: None,
            retention_class: None,
            occurred_after: None,
            occurred_before: None,
            summary_only: true,
            limit: 20,
            cursor: None,
        }
    }

    pub fn with_instance_id(mut self, id: InstanceId) -> Self {
        self.instance_id = Some(id);
        self
    }

    pub fn with_event_type(mut self, value: impl Into<String>) -> Self {
        self.event_type = Some(value.into());
        self
    }

    pub fn with_limit(mut self, limit: u32) -> Self {
        self.limit = limit;
        self
    }

    pub fn with_cursor(mut self, cursor: impl Into<String>) -> Self {
        self.cursor = Some(cursor.into());
        self
    }

    pub fn with_summary_only(mut self, value: bool) -> Self {
        self.summary_only = value;
        self
    }

    pub fn validate(&self) -> AicoreResult<()> {
        if self.limit == 0 || self.limit > 1000 {
            return Err(AicoreError::InvalidState(format!(
                "query limit must be between 1 and 1000, got {}",
                self.limit
            )));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventQueryResponse {
    pub events: Vec<EventEnvelope>,
    pub total_matched: u64,

    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub next_cursor: Option<String>,
}

impl EventQueryResponse {
    pub fn empty() -> Self {
        Self {
            events: Vec::new(),
            total_matched: 0,
            next_cursor: None,
        }
    }

    pub fn with_events(events: Vec<EventEnvelope>, total_matched: u64) -> Self {
        Self {
            events,
            total_matched,
            next_cursor: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventGetRequest {
    pub event_id: String,
    pub summary_only: bool,
}

impl EventGetRequest {
    pub fn new(event_id: impl Into<String>) -> Self {
        Self {
            event_id: event_id.into(),
            summary_only: true,
        }
    }

    pub fn with_full_evidence(mut self) -> Self {
        self.summary_only = false;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventGetResponse {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub event: Option<EventEnvelope>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_request_defaults() {
        let req = EventQueryRequest::new();
        assert!(req.summary_only);
        assert_eq!(req.limit, 20);
        assert!(req.instance_id.is_none());
    }

    #[test]
    fn query_request_builder() {
        let req = EventQueryRequest::new()
            .with_instance_id(InstanceId::global_main())
            .with_event_type("memory.remembered")
            .with_limit(50)
            .with_cursor("cursor.01");

        assert_eq!(req.limit, 50);
        assert_eq!(req.cursor, Some("cursor.01".to_string()));
        assert_eq!(req.event_type, Some("memory.remembered".to_string()));
    }

    #[test]
    fn query_request_rejects_zero_limit() {
        let req = EventQueryRequest::new().with_limit(0);
        assert!(req.validate().is_err());
    }

    #[test]
    fn query_request_rejects_too_large_limit() {
        let req = EventQueryRequest::new().with_limit(1001);
        assert!(req.validate().is_err());
    }

    #[test]
    fn query_response_empty() {
        let resp = EventQueryResponse::empty();
        assert!(resp.events.is_empty());
        assert_eq!(resp.total_matched, 0);
    }

    #[test]
    fn get_request_defaults_summary_only() {
        let req = EventGetRequest::new("evt.01");
        assert!(req.summary_only);
    }

    #[test]
    fn get_request_full_evidence() {
        let req = EventGetRequest::new("evt.01").with_full_evidence();
        assert!(!req.summary_only);
    }
}
