use aicore_foundation::AicoreError;
use serde::{Deserialize, Serialize};

macro_rules! io_id {
    ($name:ident, $label:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

io_id!(IoClientId, "io client id");
io_id!(IoConnectionId, "io connection id");
io_id!(IoRequestId, "io request id");
io_id!(IoEventId, "io event id");
io_id!(IoSubscriptionId, "io subscription id");

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IoStreamCursor(String);

impl IoStreamCursor {
    pub fn new(value: impl Into<String>) -> Result<Self, AicoreError> {
        let value = value.into();
        if !value.is_empty() && value.len() <= 512 {
            Ok(Self(value))
        } else {
            Err(AicoreError::InvalidId {
                kind: "io stream cursor".to_string(),
                value,
            })
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn parse_seq(&self) -> Option<u64> {
        None
    }
}

fn is_valid_token(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_'))
}
