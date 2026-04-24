use crate::AicoreError;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ComponentId(String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InstanceId(String);

impl ComponentId {
    pub fn new(value: impl Into<String>) -> Result<Self, AicoreError> {
        let value = value.into();
        if is_valid_token(&value) {
            Ok(Self(value))
        } else {
            Err(AicoreError::InvalidComponentId(value))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl InstanceId {
    pub fn new(value: impl Into<String>) -> Result<Self, AicoreError> {
        let value = value.into();
        if is_valid_token(&value) {
            Ok(Self(value))
        } else {
            Err(AicoreError::InvalidInstanceId(value))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn global_main() -> Self {
        Self("global-main".to_string())
    }
}

fn is_valid_token(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_'))
}

#[cfg(test)]
mod tests {
    use super::{ComponentId, InstanceId};

    #[test]
    fn accepts_safe_component_id() {
        let id = ComponentId::new("ui.tui").expect("component id should be valid");
        assert_eq!(id.as_str(), "ui.tui");
    }

    #[test]
    fn rejects_unsafe_instance_id() {
        let error = InstanceId::new("bad/id").expect_err("instance id should be rejected");
        assert_eq!(error.to_string(), "invalid instance id: bad/id");
    }
}
