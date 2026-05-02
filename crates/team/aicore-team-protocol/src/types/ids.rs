use aicore_foundation::AicoreError;
use serde::{Deserialize, Serialize};

macro_rules! team_id {
    ($name:ident, $label:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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

team_id!(TeamRunId, "team run id");
team_id!(TeamChannelId, "team channel id");
team_id!(TeamAgentId, "team agent id");
team_id!(TeamMessageId, "team message id");
team_id!(TeamResultId, "team result id");
team_id!(TeamTaskId, "team task id");

fn is_valid_token(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_'))
}
