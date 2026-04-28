use std::{fs, path::PathBuf};

use aicore_auth::{AuthCapability, AuthEntry, AuthKind, AuthRef, GlobalAuthPool, SecretRef};

use super::{
    ConfigPaths, ConfigStore, GlobalServiceProfiles, InstanceRuntimeConfig, ModelBinding,
    ProviderProfileOverride, ProviderProfilesConfig, ServiceProfile, ServiceProfileMode,
    ServiceRole,
};

mod paths;
mod providers;
mod store;
mod types;
mod validation;

fn temp_root(name: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("aicore-config-tests-{name}"));
    if root.exists() {
        fs::remove_dir_all(&root).expect("temp dir should be removable");
    }
    root
}

fn auth_pool() -> GlobalAuthPool {
    GlobalAuthPool::new(vec![
        AuthEntry {
            auth_ref: AuthRef::new("auth.openrouter.main"),
            provider: "openrouter".to_string(),
            kind: AuthKind::ApiKey,
            secret_ref: SecretRef::new("secret://auth.openrouter.main"),
            capabilities: vec![AuthCapability::Chat],
            enabled: true,
        },
        AuthEntry {
            auth_ref: AuthRef::new("auth.openai.backup"),
            provider: "openai".to_string(),
            kind: AuthKind::ApiKey,
            secret_ref: SecretRef::new("secret://auth.openai.backup"),
            capabilities: vec![AuthCapability::Chat],
            enabled: true,
        },
    ])
}
