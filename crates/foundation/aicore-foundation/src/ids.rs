use crate::AicoreError;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ComponentId(String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InstanceId(String);

macro_rules! safe_id {
    ($name:ident, $label:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, AicoreError> {
                let value = value.into();
                if is_valid_token(&value) {
                    Ok(Self(value))
                } else {
                    Err(AicoreError::InvalidId {
                        kind: $label.to_string(),
                        value,
                    })
                }
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

safe_id!(AppId, "app id");
safe_id!(CapabilityId, "capability id");
safe_id!(ContractId, "contract id");
safe_id!(RouteId, "route id");
safe_id!(InvocationId, "invocation id");
safe_id!(EventId, "event id");
safe_id!(SessionId, "session id");
safe_id!(ConversationId, "conversation id");
safe_id!(TurnId, "turn id");
safe_id!(TaskId, "task id");
safe_id!(WorkerId, "worker id");

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
    use super::{ComponentId, EventId, InstanceId, InvocationId};

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

    #[test]
    fn accepts_safe_invocation_id() {
        let id = InvocationId::new("invoke.main_01").expect("invocation id should be valid");
        assert_eq!(id.as_str(), "invoke.main_01");
    }

    #[test]
    fn rejects_unsafe_event_id() {
        let error = EventId::new("event/bad").expect_err("event id should be rejected");
        assert_eq!(error.to_string(), "invalid event id: event/bad");
    }
}
