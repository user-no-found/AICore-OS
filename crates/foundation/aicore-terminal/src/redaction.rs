use crate::sanitize::sanitize_text;

pub fn redact_text(value: &str) -> String {
    value
        .split_inclusive(char::is_whitespace)
        .map(redact_token)
        .collect()
}

pub fn safe_text(value: &str) -> String {
    redact_text(&sanitize_text(value))
}

fn redact_token(token: &str) -> String {
    let trimmed = token.trim_end_matches(char::is_whitespace);
    let suffix = &token[trimmed.len()..];
    let lower = trimmed.to_ascii_lowercase();
    if lower.contains("sk-")
        || lower.contains("secret_ref")
        || lower.contains("credential_lease_ref")
        || lower.contains("api_key")
    {
        format!("[REDACTED]{suffix}")
    } else {
        token.to_string()
    }
}
