#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AuthRef(String);

impl AuthRef {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SecretRef(String);

impl SecretRef {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthKind {
    ApiKey,
    OAuth,
    Session,
    Token,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthCapability {
    Chat,
    Vision,
    Search,
    Embedding,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthEntry {
    pub auth_ref: AuthRef,
    pub provider: String,
    pub kind: AuthKind,
    pub secret_ref: SecretRef,
    pub capabilities: Vec<AuthCapability>,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobalAuthPool {
    entries: Vec<AuthEntry>,
}

impl GlobalAuthPool {
    pub fn new(entries: Vec<AuthEntry>) -> Self {
        let mut deduped = Vec::new();

        for entry in entries {
            if deduped
                .iter()
                .any(|existing: &AuthEntry| existing.auth_ref == entry.auth_ref)
            {
                continue;
            }
            deduped.push(entry);
        }

        Self { entries: deduped }
    }

    pub fn entries(&self) -> &[AuthEntry] {
        &self.entries
    }

    pub fn available_entries(&self) -> Vec<&AuthEntry> {
        self.entries.iter().filter(|entry| entry.enabled).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{AuthCapability, AuthEntry, AuthKind, AuthRef, GlobalAuthPool, SecretRef};

    fn auth_entry(auth_ref: &str, enabled: bool) -> AuthEntry {
        AuthEntry {
            auth_ref: AuthRef::new(auth_ref),
            provider: "openrouter".to_string(),
            kind: AuthKind::ApiKey,
            secret_ref: SecretRef::new(format!("secret://{auth_ref}")),
            capabilities: vec![AuthCapability::Chat, AuthCapability::Vision],
            enabled,
        }
    }

    #[test]
    fn can_add_auth_entry_to_global_auth_pool() {
        let pool = GlobalAuthPool::new(vec![auth_entry("auth.openrouter.main", true)]);

        assert_eq!(pool.entries().len(), 1);
        assert_eq!(pool.entries()[0].auth_ref.as_str(), "auth.openrouter.main");
    }

    #[test]
    fn rejects_duplicate_auth_ref() {
        let pool = GlobalAuthPool::new(vec![
            auth_entry("auth.openrouter.main", true),
            auth_entry("auth.openrouter.main", true),
        ]);

        assert_eq!(pool.available_entries().len(), 1);
    }

    #[test]
    fn disabled_auth_is_not_available() {
        let pool = GlobalAuthPool::new(vec![
            auth_entry("auth.openrouter.main", true),
            auth_entry("auth.openrouter.backup", false),
        ]);

        assert_eq!(pool.available_entries().len(), 1);
        assert_eq!(
            pool.available_entries()[0].auth_ref.as_str(),
            "auth.openrouter.main"
        );
    }

    #[test]
    fn secret_ref_is_not_plaintext_secret() {
        let entry = auth_entry("auth.openrouter.main", true);

        assert_eq!(entry.secret_ref.as_str(), "secret://auth.openrouter.main");
        assert_ne!(entry.secret_ref.as_str(), "sk-live-secret-value");
    }
}
