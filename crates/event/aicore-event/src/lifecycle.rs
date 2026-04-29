use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetentionClass {
    Ephemeral,
    Transient30d,
    Summary180d,
    Durable,
    AuditPinned,
    NeedsReview,
    Invalid,
}

impl RetentionClass {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ephemeral => "ephemeral",
            Self::Transient30d => "transient_30d",
            Self::Summary180d => "summary_180d",
            Self::Durable => "durable",
            Self::AuditPinned => "audit_pinned",
            Self::NeedsReview => "needs_review",
            Self::Invalid => "invalid",
        }
    }

    pub fn default_for_error_index() -> Self {
        Self::Transient30d
    }

    pub fn default_for_fix_index() -> Self {
        Self::Transient30d
    }

    pub fn default_for_unclassified() -> Self {
        Self::NeedsReview
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReplayPolicy {
    Replayable,
    HistoryOnly,
    NotReplayable,
}

impl ReplayPolicy {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Replayable => "replayable",
            Self::HistoryOnly => "history_only",
            Self::NotReplayable => "not_replayable",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventStatus {
    Recorded,
    Compressed,
    Expired,
    Invalid,
}

impl EventStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Recorded => "recorded",
            Self::Compressed => "compressed",
            Self::Expired => "expired",
            Self::Invalid => "invalid",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventVisibility {
    System,
    User,
    Instance,
    GlobalMain,
}

impl EventVisibility {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::System => "system",
            Self::User => "user",
            Self::Instance => "instance",
            Self::GlobalMain => "global_main",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retention_class_roundtrip() {
        let all = [
            RetentionClass::Ephemeral,
            RetentionClass::Transient30d,
            RetentionClass::Summary180d,
            RetentionClass::Durable,
            RetentionClass::AuditPinned,
            RetentionClass::NeedsReview,
            RetentionClass::Invalid,
        ];
        for rc in all {
            let json = serde_json::to_string(&rc).unwrap();
            let back: RetentionClass = serde_json::from_str(&json).unwrap();
            assert_eq!(back, rc);
        }
    }

    #[test]
    fn replay_policy_roundtrip() {
        let all = [
            ReplayPolicy::Replayable,
            ReplayPolicy::HistoryOnly,
            ReplayPolicy::NotReplayable,
        ];
        for rp in all {
            let json = serde_json::to_string(&rp).unwrap();
            let back: ReplayPolicy = serde_json::from_str(&json).unwrap();
            assert_eq!(back, rp);
        }
    }

    #[test]
    fn event_status_roundtrip() {
        let all = [
            EventStatus::Recorded,
            EventStatus::Compressed,
            EventStatus::Expired,
            EventStatus::Invalid,
        ];
        for es in all {
            let json = serde_json::to_string(&es).unwrap();
            let back: EventStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(back, es);
        }
    }

    #[test]
    fn event_visibility_roundtrip() {
        let all = [
            EventVisibility::System,
            EventVisibility::User,
            EventVisibility::Instance,
            EventVisibility::GlobalMain,
        ];
        for ev in all {
            let json = serde_json::to_string(&ev).unwrap();
            let back: EventVisibility = serde_json::from_str(&json).unwrap();
            assert_eq!(back, ev);
        }
    }

    #[test]
    fn retention_defaults() {
        assert_eq!(
            RetentionClass::default_for_error_index(),
            RetentionClass::Transient30d
        );
        assert_eq!(
            RetentionClass::default_for_fix_index(),
            RetentionClass::Transient30d
        );
        assert_eq!(
            RetentionClass::default_for_unclassified(),
            RetentionClass::NeedsReview
        );
    }
}
