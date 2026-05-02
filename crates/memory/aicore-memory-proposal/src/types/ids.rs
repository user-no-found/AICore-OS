use aicore_foundation::AicoreError;
use serde::{Deserialize, Serialize};

macro_rules! memory_id {
    ($name:ident, $label:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, AicoreError> {
                let value = value.into();
                if is_valid_id(&value) {
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

memory_id!(MemoryProposalId, "memory proposal id");
memory_id!(MemoryReviewId, "memory review id");
memory_id!(MemoryDecisionId, "memory decision id");
memory_id!(MemoryWriteRequestId, "memory write request id");
memory_id!(MemoryRecordRef, "memory record ref");

fn is_valid_id(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_'))
}
