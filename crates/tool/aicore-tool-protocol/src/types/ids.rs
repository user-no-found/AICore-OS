use aicore_foundation::AicoreError;
use serde::{Deserialize, Serialize};

macro_rules! tool_id {
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

tool_id!(ToolId, "tool id");
tool_id!(ToolModuleId, "tool module id");
tool_id!(ToolVersion, "tool version");
tool_id!(ToolCallId, "tool call id");
tool_id!(ToolSchemaHash, "tool schema hash");
tool_id!(ToolArgsDigest, "tool args digest");
tool_id!(ToolRegistryRevision, "tool registry revision");
tool_id!(ToolNoticeId, "tool notice id");
tool_id!(SandboxProfileId, "sandbox profile id");

fn is_valid_token(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_'))
}
