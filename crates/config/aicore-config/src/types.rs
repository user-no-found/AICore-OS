use std::path::PathBuf;

use aicore_auth::AuthRef;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceRole {
    MemoryExtractor,
    MemoryCurator,
    MemoryDreamer,
    EvolutionProposer,
    EvolutionReviewer,
    Search,
    Tts,
    ImageGeneration,
    VideoGeneration,
    Vision,
    Reranker,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceProfileMode {
    InheritInstance,
    Explicit,
    Disabled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceProfile {
    pub role: ServiceRole,
    pub mode: ServiceProfileMode,
    pub auth_ref: Option<AuthRef>,
    pub model: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobalServiceProfiles {
    pub profiles: Vec<ServiceProfile>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelBinding {
    pub auth_ref: AuthRef,
    pub model: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceRuntimeConfig {
    pub instance_id: String,
    pub primary: ModelBinding,
    pub fallback: Option<ModelBinding>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigPaths {
    pub root: PathBuf,
    pub auth_toml: PathBuf,
    pub services_toml: PathBuf,
    pub providers_toml: PathBuf,
    pub instances_dir: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigStore {
    pub paths: ConfigPaths,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    Io(String),
    Parse(String),
    Validation(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderProfileOverride {
    pub provider_id: String,
    pub base_url: Option<String>,
    pub api_mode: Option<String>,
    pub engine_id: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderProfilesConfig {
    pub profiles: Vec<ProviderProfileOverride>,
}
