use aicore_foundation::AicoreError;
use serde::{Deserialize, Serialize};

macro_rules! skill_id {
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

skill_id!(SkillId, "skill id");
skill_id!(SkillVersion, "skill version");
skill_id!(SkillRegistryRevision, "skill registry revision");
skill_id!(SkillNoticeId, "skill notice id");
skill_id!(SkillContextId, "skill context id");

fn is_valid_token(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_'))
}
