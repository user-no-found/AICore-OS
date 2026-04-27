#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedactedText {
    safe_text: String,
}

impl RedactedText {
    pub fn new(safe_text: impl Into<String>) -> Self {
        Self {
            safe_text: safe_text.into(),
        }
    }

    pub fn as_str(&self) -> &str {
        &self.safe_text
    }
}

pub fn redact_secret(label: impl AsRef<str>) -> RedactedText {
    RedactedText::new(format!("[redacted:{}]", label.as_ref()))
}

#[cfg(test)]
mod tests {
    use super::redact_secret;

    #[test]
    fn redaction_does_not_expose_secret_value() {
        let secret = "sk-live-secret-value";
        let redacted = redact_secret("api_key");

        assert_eq!(redacted.as_str(), "[redacted:api_key]");
        assert!(!redacted.as_str().contains(secret));
    }
}
