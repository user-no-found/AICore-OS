use aicore_foundation::{AicoreError, AicoreResult};

pub const MAX_SUMMARY_LEN: usize = 1024;
pub const MAX_SUBJECT_TYPE_LEN: usize = 128;
pub const MAX_SUBJECT_ID_LEN: usize = 256;
pub const MAX_SCHEMA_VERSION_LEN: usize = 64;
pub const MAX_EVENT_TYPE_LEN: usize = 128;

fn is_safe_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_' | ':'))
}

pub fn validate_event_type(value: &str) -> AicoreResult<()> {
    if value.is_empty() {
        return Err(AicoreError::InvalidId {
            kind: "event type".to_string(),
            value: value.to_string(),
        });
    }
    if value.len() > MAX_EVENT_TYPE_LEN {
        return Err(AicoreError::InvalidId {
            kind: "event type".to_string(),
            value: format!("too long: {} chars", value.len()),
        });
    }
    if !value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_' | ':'))
    {
        return Err(AicoreError::InvalidId {
            kind: "event type".to_string(),
            value: value.to_string(),
        });
    }
    Ok(())
}

pub fn validate_schema_version(value: &str) -> AicoreResult<()> {
    if value.is_empty() || value.len() > MAX_SCHEMA_VERSION_LEN {
        return Err(AicoreError::InvalidId {
            kind: "schema version".to_string(),
            value: value.to_string(),
        });
    }
    Ok(())
}

pub fn validate_summary(value: &str) -> AicoreResult<()> {
    if value.is_empty() {
        return Err(AicoreError::InvalidId {
            kind: "summary".to_string(),
            value: "empty".to_string(),
        });
    }
    if value.len() > MAX_SUMMARY_LEN {
        return Err(AicoreError::InvalidId {
            kind: "summary".to_string(),
            value: format!("too long: {} chars", value.len()),
        });
    }
    Ok(())
}

pub fn validate_subject_type(value: &str) -> AicoreResult<()> {
    if value.is_empty() || value.len() > MAX_SUBJECT_TYPE_LEN || !is_safe_token(value) {
        return Err(AicoreError::InvalidId {
            kind: "subject type".to_string(),
            value: value.to_string(),
        });
    }
    Ok(())
}

pub fn validate_subject_id(value: &str) -> AicoreResult<()> {
    if value.is_empty() || value.len() > MAX_SUBJECT_ID_LEN {
        return Err(AicoreError::InvalidId {
            kind: "subject id".to_string(),
            value: value.to_string(),
        });
    }
    Ok(())
}

pub fn validate_ref(value: &str, kind: &str) -> AicoreResult<()> {
    if value.is_empty() || value.len() > 512 {
        return Err(AicoreError::InvalidId {
            kind: kind.to_string(),
            value: value.to_string(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_type_valid() {
        assert!(validate_event_type("memory.remembered").is_ok());
    }

    #[test]
    fn event_type_rejects_empty() {
        assert!(validate_event_type("").is_err());
    }

    #[test]
    fn event_type_rejects_slash() {
        assert!(validate_event_type("bad/type").is_err());
    }

    #[test]
    fn summary_rejects_empty() {
        assert!(validate_summary("").is_err());
    }

    #[test]
    fn summary_rejects_too_long() {
        let long = "a".repeat(MAX_SUMMARY_LEN + 1);
        assert!(validate_summary(&long).is_err());
    }

    #[test]
    fn subject_type_rejects_empty() {
        assert!(validate_subject_type("").is_err());
    }

    #[test]
    fn subject_id_rejects_empty() {
        assert!(validate_subject_id("").is_err());
    }

    #[test]
    fn ref_rejects_empty() {
        assert!(validate_ref("", "evidence_ref").is_err());
    }
}
