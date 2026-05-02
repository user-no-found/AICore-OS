use aicore_foundation::AicoreError;
use serde::{Deserialize, Serialize};

macro_rules! model_id {
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

model_id!(ModelRequestId, "model request id");
model_id!(ModelRunId, "model run id");
model_id!(ModelEventId, "model event id");
model_id!(ProviderId, "provider id");
model_id!(ModelId, "model id");
model_id!(PromptAssemblyId, "prompt assembly id");
model_id!(PromptModuleId, "prompt module id");

fn is_valid_token(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_'))
}
