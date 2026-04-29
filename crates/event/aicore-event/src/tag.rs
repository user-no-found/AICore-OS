use aicore_foundation::{AicoreError, AicoreResult};
use serde::{Deserialize, Serialize};

fn is_valid_tag(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_' | ':'))
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventTag(String);

impl EventTag {
    pub fn new(value: impl Into<String>) -> AicoreResult<Self> {
        let value = value.into();
        if is_valid_tag(&value) {
            Ok(Self(value))
        } else {
            Err(AicoreError::InvalidId {
                kind: "event tag".to_string(),
                value,
            })
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct EventTagSet {
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub tags: Vec<EventTag>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub confirmed: Vec<EventTag>,
}

impl EventTagSet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_tag(mut self, tag: EventTag) -> Self {
        self.tags.push(tag);
        self
    }

    pub fn with_confirmed(mut self, tag: EventTag) -> Self {
        self.confirmed.push(tag);
        self
    }

    pub fn is_empty(&self) -> bool {
        self.tags.is_empty() && self.confirmed.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_valid_tag() {
        let tag = EventTag::new("error").unwrap();
        assert_eq!(tag.as_str(), "error");
    }

    #[test]
    fn accepts_tag_with_colon() {
        let tag = EventTag::new("severity:high").unwrap();
        assert_eq!(tag.as_str(), "severity:high");
    }

    #[test]
    fn rejects_empty_tag() {
        let err = EventTag::new("").expect_err("should reject");
        assert!(err.to_string().contains("event tag"));
    }

    #[test]
    fn rejects_tag_with_slash() {
        let err = EventTag::new("bad/tag").expect_err("should reject");
        assert!(err.to_string().contains("event tag"));
    }

    #[test]
    fn tag_set_builder() {
        let set = EventTagSet::new()
            .with_tag(EventTag::new("error").unwrap())
            .with_confirmed(EventTag::new("fixed").unwrap());
        assert_eq!(set.tags.len(), 1);
        assert_eq!(set.confirmed.len(), 1);
    }
}
