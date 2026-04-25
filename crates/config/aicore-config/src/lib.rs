use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use aicore_auth::{AuthCapability, AuthEntry, AuthKind, AuthRef, GlobalAuthPool, SecretRef};

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

impl ConfigPaths {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        let root = root.into();

        Self {
            auth_toml: root.join("auth.toml"),
            services_toml: root.join("services.toml"),
            instances_dir: root.join("instances"),
            root,
        }
    }

    pub fn runtime_toml_for(&self, instance_id: &str) -> PathBuf {
        self.instances_dir.join(instance_id).join("runtime.toml")
    }
}

impl ConfigStore {
    pub fn new(paths: ConfigPaths) -> Self {
        Self { paths }
    }

    pub fn ensure_default_files(&self) -> Result<(), ConfigError> {
        self.ensure_root_dirs()?;

        if !self.paths.auth_toml.exists() {
            self.write_file(
                &self.paths.auth_toml,
                &render_auth_pool(&GlobalAuthPool::new(Vec::new())),
            )?;
        }

        if !self.paths.services_toml.exists() {
            self.write_file(
                &self.paths.services_toml,
                &render_services(&GlobalServiceProfiles {
                    profiles: Vec::new(),
                }),
            )?;
        }

        Ok(())
    }

    pub fn save_auth_pool(&self, pool: &GlobalAuthPool) -> Result<(), ConfigError> {
        self.ensure_root_dirs()?;
        self.write_file(&self.paths.auth_toml, &render_auth_pool(pool))
    }

    pub fn load_auth_pool(&self) -> Result<GlobalAuthPool, ConfigError> {
        let Some(contents) = self.read_file_if_exists(&self.paths.auth_toml)? else {
            return Ok(GlobalAuthPool::new(Vec::new()));
        };

        parse_auth_pool(&contents)
    }

    pub fn save_services(&self, services: &GlobalServiceProfiles) -> Result<(), ConfigError> {
        self.ensure_root_dirs()?;
        self.write_file(&self.paths.services_toml, &render_services(services))
    }

    pub fn load_services(&self) -> Result<GlobalServiceProfiles, ConfigError> {
        let Some(contents) = self.read_file_if_exists(&self.paths.services_toml)? else {
            return Ok(GlobalServiceProfiles {
                profiles: Vec::new(),
            });
        };

        parse_services(&contents)
    }

    pub fn save_instance_runtime(&self, config: &InstanceRuntimeConfig) -> Result<(), ConfigError> {
        self.ensure_root_dirs()?;
        self.write_file(
            &self.paths.runtime_toml_for(&config.instance_id),
            &render_runtime(config),
        )
    }

    pub fn load_instance_runtime(
        &self,
        instance_id: &str,
    ) -> Result<InstanceRuntimeConfig, ConfigError> {
        let path = self.paths.runtime_toml_for(instance_id);
        let contents = self.read_file_if_exists(&path)?.ok_or_else(|| {
            ConfigError::Io(format!("missing runtime config: {}", path.display()))
        })?;

        parse_runtime(&contents)
    }

    pub fn validate_runtime_config(
        config: &InstanceRuntimeConfig,
        auth_pool: &GlobalAuthPool,
    ) -> Result<(), ConfigError> {
        if !auth_pool_contains(auth_pool, &config.primary.auth_ref) {
            return Err(ConfigError::Validation(format!(
                "primary auth_ref not found: {}",
                config.primary.auth_ref.as_str()
            )));
        }

        if let Some(fallback) = &config.fallback {
            if !auth_pool_contains(auth_pool, &fallback.auth_ref) {
                return Err(ConfigError::Validation(format!(
                    "fallback auth_ref not found: {}",
                    fallback.auth_ref.as_str()
                )));
            }
        }

        Ok(())
    }

    pub fn validate_service_profiles(
        services: &GlobalServiceProfiles,
        auth_pool: &GlobalAuthPool,
    ) -> Result<(), ConfigError> {
        for profile in &services.profiles {
            if profile.mode != ServiceProfileMode::Explicit {
                continue;
            }

            let auth_ref = profile.auth_ref.as_ref().ok_or_else(|| {
                ConfigError::Validation(format!(
                    "explicit service profile missing auth_ref: {}",
                    render_service_role(&profile.role)
                ))
            })?;

            let model = profile.model.as_ref().ok_or_else(|| {
                ConfigError::Validation(format!(
                    "explicit service profile missing model: {}",
                    render_service_role(&profile.role)
                ))
            })?;

            if model.is_empty() {
                return Err(ConfigError::Validation(format!(
                    "explicit service profile missing model: {}",
                    render_service_role(&profile.role)
                )));
            }

            if !auth_pool_contains(auth_pool, auth_ref) {
                return Err(ConfigError::Validation(format!(
                    "explicit service profile auth_ref not found: {}",
                    auth_ref.as_str()
                )));
            }
        }

        Ok(())
    }

    fn ensure_root_dirs(&self) -> Result<(), ConfigError> {
        fs::create_dir_all(&self.paths.root).map_err(io_error)?;
        fs::create_dir_all(&self.paths.instances_dir).map_err(io_error)?;
        Ok(())
    }

    fn read_file_if_exists(&self, path: &Path) -> Result<Option<String>, ConfigError> {
        match fs::read_to_string(path) {
            Ok(contents) => Ok(Some(contents)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(io_error(error)),
        }
    }

    fn write_file(&self, path: &Path, contents: &str) -> Result<(), ConfigError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(io_error)?;
        }

        fs::write(path, contents).map_err(io_error)
    }
}

fn io_error(error: std::io::Error) -> ConfigError {
    ConfigError::Io(error.to_string())
}

fn auth_pool_contains(auth_pool: &GlobalAuthPool, auth_ref: &AuthRef) -> bool {
    auth_pool
        .entries()
        .iter()
        .any(|entry| entry.auth_ref == *auth_ref)
}

fn render_auth_pool(pool: &GlobalAuthPool) -> String {
    let mut output = String::from("# AICore OS auth pool\n");

    for entry in pool.entries() {
        output.push_str("\n[[auth]]\n");
        output.push_str(&format!(
            "auth_ref = {}\n",
            render_string(entry.auth_ref.as_str())
        ));
        output.push_str(&format!("provider = {}\n", render_string(&entry.provider)));
        output.push_str(&format!(
            "kind = {}\n",
            render_string(render_auth_kind(&entry.kind))
        ));
        output.push_str(&format!(
            "secret_ref = {}\n",
            render_string(entry.secret_ref.as_str())
        ));
        output.push_str(&format!(
            "capabilities = {}\n",
            render_string_list(
                &entry
                    .capabilities
                    .iter()
                    .map(render_auth_capability)
                    .collect::<Vec<_>>()
            )
        ));
        output.push_str(&format!("enabled = {}\n", entry.enabled));
    }

    output
}

fn render_services(services: &GlobalServiceProfiles) -> String {
    let mut output = String::from("# AICore OS service profiles\n");

    for profile in &services.profiles {
        output.push_str("\n[[service]]\n");
        output.push_str(&format!(
            "role = {}\n",
            render_string(render_service_role(&profile.role))
        ));
        output.push_str(&format!(
            "mode = {}\n",
            render_string(render_service_profile_mode(&profile.mode))
        ));

        if let Some(auth_ref) = &profile.auth_ref {
            output.push_str(&format!(
                "auth_ref = {}\n",
                render_string(auth_ref.as_str())
            ));
        }

        if let Some(model) = &profile.model {
            output.push_str(&format!("model = {}\n", render_string(model)));
        }
    }

    output
}

fn render_runtime(config: &InstanceRuntimeConfig) -> String {
    let mut output = String::from("# AICore OS instance runtime\n");
    output.push_str(&format!(
        "instance_id = {}\n",
        render_string(&config.instance_id)
    ));
    output.push_str(&format!(
        "primary_auth_ref = {}\n",
        render_string(config.primary.auth_ref.as_str())
    ));
    output.push_str(&format!(
        "primary_model = {}\n",
        render_string(&config.primary.model)
    ));

    if let Some(fallback) = &config.fallback {
        output.push_str(&format!(
            "fallback_auth_ref = {}\n",
            render_string(fallback.auth_ref.as_str())
        ));
        output.push_str(&format!(
            "fallback_model = {}\n",
            render_string(&fallback.model)
        ));
    }

    output
}

fn parse_auth_pool(contents: &str) -> Result<GlobalAuthPool, ConfigError> {
    let mut entries = Vec::new();

    for section in parse_sections(contents, "[[auth]]")? {
        let fields = parse_key_values(&section)?;
        let capabilities = fields
            .get("capabilities")
            .map(|value| parse_string_list(value))
            .transpose()?
            .unwrap_or_default()
            .into_iter()
            .map(|value| parse_auth_capability(&value))
            .collect::<Result<Vec<_>, _>>()?;

        entries.push(AuthEntry {
            auth_ref: AuthRef::new(required_string_field(&fields, "auth_ref")?),
            provider: required_string_field(&fields, "provider")?,
            kind: parse_auth_kind(&required_string_field(&fields, "kind")?)?,
            secret_ref: SecretRef::new(required_string_field(&fields, "secret_ref")?),
            capabilities,
            enabled: parse_bool(&required_raw_field(&fields, "enabled")?)?,
        });
    }

    Ok(GlobalAuthPool::new(entries))
}

fn parse_services(contents: &str) -> Result<GlobalServiceProfiles, ConfigError> {
    let mut profiles = Vec::new();

    for section in parse_sections(contents, "[[service]]")? {
        let fields = parse_key_values(&section)?;
        profiles.push(ServiceProfile {
            role: parse_service_role(&required_string_field(&fields, "role")?)?,
            mode: parse_service_profile_mode(&required_string_field(&fields, "mode")?)?,
            auth_ref: optional_field(&fields, "auth_ref").map(AuthRef::new),
            model: optional_field(&fields, "model"),
        });
    }

    Ok(GlobalServiceProfiles { profiles })
}

fn parse_runtime(contents: &str) -> Result<InstanceRuntimeConfig, ConfigError> {
    let fields = parse_key_values(
        &contents
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>(),
    )?;

    let fallback_auth_ref = optional_field(&fields, "fallback_auth_ref");
    let fallback_model = optional_field(&fields, "fallback_model");

    let fallback = match (fallback_auth_ref, fallback_model) {
        (Some(auth_ref), Some(model)) => Some(ModelBinding {
            auth_ref: AuthRef::new(auth_ref),
            model,
        }),
        (None, None) => None,
        _ => {
            return Err(ConfigError::Parse(
                "fallback_auth_ref and fallback_model must both exist".to_string(),
            ));
        }
    };

    Ok(InstanceRuntimeConfig {
        instance_id: required_string_field(&fields, "instance_id")?,
        primary: ModelBinding {
            auth_ref: AuthRef::new(required_string_field(&fields, "primary_auth_ref")?),
            model: required_string_field(&fields, "primary_model")?,
        },
        fallback,
    })
}

fn parse_sections(contents: &str, header: &str) -> Result<Vec<Vec<String>>, ConfigError> {
    let mut sections = Vec::new();
    let mut current: Option<Vec<String>> = None;

    for line in contents.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if trimmed == header {
            if let Some(section) = current.take() {
                sections.push(section);
            }
            current = Some(Vec::new());
            continue;
        }

        let section = current.as_mut().ok_or_else(|| {
            ConfigError::Parse(format!("line outside section {header}: {trimmed}"))
        })?;
        section.push(trimmed.to_string());
    }

    if let Some(section) = current {
        sections.push(section);
    }

    Ok(sections)
}

fn parse_key_values(lines: &[String]) -> Result<HashMap<String, String>, ConfigError> {
    let mut fields = HashMap::new();

    for line in lines {
        let (key, raw_value) = line
            .split_once('=')
            .ok_or_else(|| ConfigError::Parse(format!("invalid key/value line: {line}")))?;

        fields.insert(key.trim().to_string(), raw_value.trim().to_string());
    }

    Ok(fields)
}

fn required_raw_field(fields: &HashMap<String, String>, key: &str) -> Result<String, ConfigError> {
    fields
        .get(key)
        .cloned()
        .ok_or_else(|| ConfigError::Parse(format!("missing field: {key}")))
}

fn required_string_field(
    fields: &HashMap<String, String>,
    key: &str,
) -> Result<String, ConfigError> {
    parse_string_value(&required_raw_field(fields, key)?)
}

fn optional_field(fields: &HashMap<String, String>, key: &str) -> Option<String> {
    fields
        .get(key)
        .and_then(|value| parse_string_value(value).ok())
}

fn parse_string_value(value: &str) -> Result<String, ConfigError> {
    let trimmed = value.trim();

    if !(trimmed.starts_with('"') && trimmed.ends_with('"')) {
        return Err(ConfigError::Parse(format!("invalid string value: {value}")));
    }

    let inner = &trimmed[1..trimmed.len() - 1];
    let mut parsed = String::new();
    let mut escaped = false;

    for ch in inner.chars() {
        if escaped {
            parsed.push(ch);
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else {
            parsed.push(ch);
        }
    }

    if escaped {
        return Err(ConfigError::Parse(format!(
            "unterminated escape in value: {value}"
        )));
    }

    Ok(parsed)
}

fn parse_string_list(value: &str) -> Result<Vec<String>, ConfigError> {
    let trimmed = value.trim();

    if !(trimmed.starts_with('[') && trimmed.ends_with(']')) {
        return Err(ConfigError::Parse(format!("invalid string list: {value}")));
    }

    let inner = trimmed[1..trimmed.len() - 1].trim();
    if inner.is_empty() {
        return Ok(Vec::new());
    }

    inner
        .split(',')
        .map(|item| parse_string_value(item.trim()))
        .collect()
}

fn parse_bool(value: &str) -> Result<bool, ConfigError> {
    match value.trim() {
        "true" => Ok(true),
        "false" => Ok(false),
        other => Err(ConfigError::Parse(format!("invalid bool value: {other}"))),
    }
}

fn render_string(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

fn render_string_list(values: &[&str]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(|value| render_string(value))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn render_auth_kind(kind: &AuthKind) -> &'static str {
    match kind {
        AuthKind::ApiKey => "api_key",
        AuthKind::OAuth => "oauth",
        AuthKind::Session => "session",
        AuthKind::Token => "token",
    }
}

fn parse_auth_kind(value: &str) -> Result<AuthKind, ConfigError> {
    match value {
        "api_key" => Ok(AuthKind::ApiKey),
        "oauth" => Ok(AuthKind::OAuth),
        "session" => Ok(AuthKind::Session),
        "token" => Ok(AuthKind::Token),
        other => Err(ConfigError::Parse(format!("unknown auth kind: {other}"))),
    }
}

fn render_auth_capability(capability: &AuthCapability) -> &'static str {
    match capability {
        AuthCapability::Chat => "chat",
        AuthCapability::Vision => "vision",
        AuthCapability::Search => "search",
        AuthCapability::Embedding => "embedding",
    }
}

fn parse_auth_capability(value: &str) -> Result<AuthCapability, ConfigError> {
    match value {
        "chat" => Ok(AuthCapability::Chat),
        "vision" => Ok(AuthCapability::Vision),
        "search" => Ok(AuthCapability::Search),
        "embedding" => Ok(AuthCapability::Embedding),
        other => Err(ConfigError::Parse(format!(
            "unknown auth capability: {other}"
        ))),
    }
}

fn render_service_role(role: &ServiceRole) -> &'static str {
    match role {
        ServiceRole::MemoryExtractor => "memory_extractor",
        ServiceRole::MemoryCurator => "memory_curator",
        ServiceRole::MemoryDreamer => "memory_dreamer",
        ServiceRole::EvolutionProposer => "evolution_proposer",
        ServiceRole::EvolutionReviewer => "evolution_reviewer",
        ServiceRole::Search => "search",
        ServiceRole::Tts => "tts",
        ServiceRole::ImageGeneration => "image_generation",
        ServiceRole::VideoGeneration => "video_generation",
        ServiceRole::Vision => "vision",
        ServiceRole::Reranker => "reranker",
    }
}

fn parse_service_role(value: &str) -> Result<ServiceRole, ConfigError> {
    match value {
        "memory_extractor" => Ok(ServiceRole::MemoryExtractor),
        "memory_curator" => Ok(ServiceRole::MemoryCurator),
        "memory_dreamer" => Ok(ServiceRole::MemoryDreamer),
        "evolution_proposer" => Ok(ServiceRole::EvolutionProposer),
        "evolution_reviewer" => Ok(ServiceRole::EvolutionReviewer),
        "search" => Ok(ServiceRole::Search),
        "tts" => Ok(ServiceRole::Tts),
        "image_generation" => Ok(ServiceRole::ImageGeneration),
        "video_generation" => Ok(ServiceRole::VideoGeneration),
        "vision" => Ok(ServiceRole::Vision),
        "reranker" => Ok(ServiceRole::Reranker),
        other => Err(ConfigError::Parse(format!("unknown service role: {other}"))),
    }
}

fn render_service_profile_mode(mode: &ServiceProfileMode) -> &'static str {
    match mode {
        ServiceProfileMode::InheritInstance => "inherit_instance",
        ServiceProfileMode::Explicit => "explicit",
        ServiceProfileMode::Disabled => "disabled",
    }
}

fn parse_service_profile_mode(value: &str) -> Result<ServiceProfileMode, ConfigError> {
    match value {
        "inherit_instance" => Ok(ServiceProfileMode::InheritInstance),
        "explicit" => Ok(ServiceProfileMode::Explicit),
        "disabled" => Ok(ServiceProfileMode::Disabled),
        other => Err(ConfigError::Parse(format!(
            "unknown service profile mode: {other}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use aicore_auth::{AuthCapability, AuthEntry, AuthKind, AuthRef, GlobalAuthPool, SecretRef};

    use super::{
        ConfigPaths, ConfigStore, GlobalServiceProfiles, InstanceRuntimeConfig, ModelBinding,
        ServiceProfile, ServiceProfileMode, ServiceRole,
    };

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

    #[test]
    fn separates_auth_pool_from_runtime_config() {
        let auth_pool = GlobalAuthPool::new(vec![AuthEntry {
            auth_ref: AuthRef::new("auth.openrouter.main"),
            provider: "openrouter".to_string(),
            kind: AuthKind::ApiKey,
            secret_ref: SecretRef::new("secret://auth.openrouter.main"),
            capabilities: vec![AuthCapability::Chat],
            enabled: true,
        }]);

        let runtime = InstanceRuntimeConfig {
            instance_id: "global-main".to_string(),
            primary: ModelBinding {
                auth_ref: AuthRef::new("auth.openrouter.main"),
                model: "openai/gpt-5".to_string(),
            },
            fallback: None,
        };

        assert_eq!(auth_pool.entries().len(), 1);
        assert_eq!(runtime.primary.model, "openai/gpt-5");
    }

    #[test]
    fn primary_model_binding_uses_auth_ref() {
        let runtime = InstanceRuntimeConfig {
            instance_id: "inst_project_a".to_string(),
            primary: ModelBinding {
                auth_ref: AuthRef::new("auth.openrouter.main"),
                model: "anthropic/claude-sonnet".to_string(),
            },
            fallback: None,
        };

        assert_eq!(
            runtime.primary.auth_ref,
            AuthRef::new("auth.openrouter.main")
        );
    }

    #[test]
    fn fallback_model_binding_is_optional() {
        let runtime = InstanceRuntimeConfig {
            instance_id: "inst_project_a".to_string(),
            primary: ModelBinding {
                auth_ref: AuthRef::new("auth.openrouter.main"),
                model: "anthropic/claude-sonnet".to_string(),
            },
            fallback: Some(ModelBinding {
                auth_ref: AuthRef::new("auth.openai.backup"),
                model: "gpt-4.1".to_string(),
            }),
        };

        assert_eq!(runtime.fallback.as_ref().unwrap().model, "gpt-4.1");
    }

    #[test]
    fn runtime_config_can_have_different_primary_and_fallback_auth_refs() {
        let runtime = InstanceRuntimeConfig {
            instance_id: "inst_project_a".to_string(),
            primary: ModelBinding {
                auth_ref: AuthRef::new("auth.openrouter.main"),
                model: "anthropic/claude-sonnet".to_string(),
            },
            fallback: Some(ModelBinding {
                auth_ref: AuthRef::new("auth.openai.backup"),
                model: "gpt-4.1".to_string(),
            }),
        };

        assert_ne!(
            runtime.primary.auth_ref,
            runtime.fallback.as_ref().unwrap().auth_ref
        );
    }

    #[test]
    fn runtime_config_does_not_store_secret_ref_or_secret_value() {
        let runtime = InstanceRuntimeConfig {
            instance_id: "inst_project_a".to_string(),
            primary: ModelBinding {
                auth_ref: AuthRef::new("auth.openrouter.main"),
                model: "anthropic/claude-sonnet".to_string(),
            },
            fallback: None,
        };

        assert_eq!(
            runtime.primary.auth_ref,
            AuthRef::new("auth.openrouter.main")
        );
        assert_ne!(runtime.primary.model, "secret://auth.openrouter.main");
        assert_ne!(runtime.primary.model, "sk-live-secret-value");
    }

    #[test]
    fn separates_service_profiles_from_instance_runtime() {
        let services = GlobalServiceProfiles {
            profiles: vec![ServiceProfile {
                role: ServiceRole::MemoryDreamer,
                mode: ServiceProfileMode::InheritInstance,
                auth_ref: None,
                model: None,
            }],
        };

        let runtime = InstanceRuntimeConfig {
            instance_id: "inst_project_a".to_string(),
            primary: ModelBinding {
                auth_ref: AuthRef::new("auth.openrouter.main"),
                model: "anthropic/claude-sonnet".to_string(),
            },
            fallback: Some(ModelBinding {
                auth_ref: AuthRef::new("auth.openai.backup"),
                model: "gpt-4.1".to_string(),
            }),
        };

        assert_eq!(services.profiles[0].role, ServiceRole::MemoryDreamer);
        assert_eq!(runtime.instance_id, "inst_project_a");
    }

    #[test]
    fn default_service_profile_inherits_instance() {
        let profile = ServiceProfile {
            role: ServiceRole::MemoryDreamer,
            mode: ServiceProfileMode::InheritInstance,
            auth_ref: None,
            model: None,
        };

        assert_eq!(profile.mode, ServiceProfileMode::InheritInstance);
        assert_eq!(profile.auth_ref, None);
        assert_eq!(profile.model, None);
    }

    #[test]
    fn explicit_service_profile_uses_auth_ref_and_model() {
        let profile = ServiceProfile {
            role: ServiceRole::Search,
            mode: ServiceProfileMode::Explicit,
            auth_ref: Some(AuthRef::new("auth.openrouter.search")),
            model: Some("perplexity/sonar".to_string()),
        };

        assert_eq!(profile.mode, ServiceProfileMode::Explicit);
        assert_eq!(
            profile.auth_ref,
            Some(AuthRef::new("auth.openrouter.search"))
        );
        assert_eq!(profile.model.as_deref(), Some("perplexity/sonar"));
    }

    #[test]
    fn disabled_service_profile_has_no_auth_or_model_requirement() {
        let profile = ServiceProfile {
            role: ServiceRole::EvolutionReviewer,
            mode: ServiceProfileMode::Disabled,
            auth_ref: None,
            model: None,
        };

        assert_eq!(profile.mode, ServiceProfileMode::Disabled);
        assert_eq!(profile.auth_ref, None);
        assert_eq!(profile.model, None);
    }

    #[test]
    fn memory_dreamer_can_be_explicit() {
        let profile = ServiceProfile {
            role: ServiceRole::MemoryDreamer,
            mode: ServiceProfileMode::Explicit,
            auth_ref: Some(AuthRef::new("auth.openrouter.memory")),
            model: Some("openai/gpt-5".to_string()),
        };

        assert_eq!(profile.role, ServiceRole::MemoryDreamer);
        assert_eq!(profile.mode, ServiceProfileMode::Explicit);
    }

    #[test]
    fn evolution_reviewer_can_be_disabled() {
        let profile = ServiceProfile {
            role: ServiceRole::EvolutionReviewer,
            mode: ServiceProfileMode::Disabled,
            auth_ref: None,
            model: None,
        };

        assert_eq!(profile.role, ServiceRole::EvolutionReviewer);
        assert_eq!(profile.mode, ServiceProfileMode::Disabled);
    }

    #[test]
    fn config_paths_resolve_expected_files() {
        let paths = ConfigPaths::new("/tmp/aicore-config");

        assert_eq!(paths.root, PathBuf::from("/tmp/aicore-config"));
        assert_eq!(
            paths.auth_toml,
            PathBuf::from("/tmp/aicore-config/auth.toml")
        );
        assert_eq!(
            paths.services_toml,
            PathBuf::from("/tmp/aicore-config/services.toml")
        );
        assert_eq!(
            paths.instances_dir,
            PathBuf::from("/tmp/aicore-config/instances")
        );
    }

    #[test]
    fn config_paths_resolve_instance_runtime_file() {
        let paths = ConfigPaths::new("/tmp/aicore-config");

        assert_eq!(
            paths.runtime_toml_for("global-main"),
            PathBuf::from("/tmp/aicore-config/instances/global-main/runtime.toml")
        );
    }

    #[test]
    fn ensure_default_files_creates_empty_auth_and_services_files() {
        let root = temp_root("default-files");
        let store = ConfigStore::new(ConfigPaths::new(&root));

        store
            .ensure_default_files()
            .expect("default files should be created");

        assert!(store.paths.root.exists());
        assert!(store.paths.auth_toml.exists());
        assert!(store.paths.services_toml.exists());
        assert!(store.paths.instances_dir.exists());

        let auth = store
            .load_auth_pool()
            .expect("default auth pool should load");
        let services = store.load_services().expect("default services should load");

        assert!(auth.entries().is_empty());
        assert!(services.profiles.is_empty());
    }

    #[test]
    fn save_and_load_auth_pool() {
        let root = temp_root("auth-pool");
        let store = ConfigStore::new(ConfigPaths::new(&root));
        let pool = auth_pool();

        store.save_auth_pool(&pool).expect("auth pool should save");
        let loaded = store.load_auth_pool().expect("auth pool should load");

        assert_eq!(loaded.entries().len(), 2);
        assert_eq!(
            loaded.entries()[0].auth_ref,
            AuthRef::new("auth.openrouter.main")
        );
    }

    #[test]
    fn save_and_load_instance_runtime_config() {
        let root = temp_root("runtime-config");
        let store = ConfigStore::new(ConfigPaths::new(&root));
        let runtime = InstanceRuntimeConfig {
            instance_id: "global-main".to_string(),
            primary: ModelBinding {
                auth_ref: AuthRef::new("auth.openrouter.main"),
                model: "openai/gpt-5".to_string(),
            },
            fallback: Some(ModelBinding {
                auth_ref: AuthRef::new("auth.openai.backup"),
                model: "gpt-4.1".to_string(),
            }),
        };

        store
            .save_instance_runtime(&runtime)
            .expect("runtime config should save");
        let loaded = store
            .load_instance_runtime("global-main")
            .expect("runtime config should load");

        assert_eq!(
            loaded.primary.auth_ref,
            AuthRef::new("auth.openrouter.main")
        );
        assert_eq!(
            loaded.fallback.unwrap().auth_ref,
            AuthRef::new("auth.openai.backup")
        );
    }

    #[test]
    fn save_and_load_global_service_profiles() {
        let root = temp_root("services");
        let store = ConfigStore::new(ConfigPaths::new(&root));
        let services = GlobalServiceProfiles {
            profiles: vec![ServiceProfile {
                role: ServiceRole::MemoryDreamer,
                mode: ServiceProfileMode::Explicit,
                auth_ref: Some(AuthRef::new("auth.openrouter.main")),
                model: Some("openai/gpt-5".to_string()),
            }],
        };

        store
            .save_services(&services)
            .expect("services should save");
        let loaded = store.load_services().expect("services should load");

        assert_eq!(loaded.profiles.len(), 1);
        assert_eq!(loaded.profiles[0].role, ServiceRole::MemoryDreamer);
        assert_eq!(
            loaded.profiles[0].auth_ref,
            Some(AuthRef::new("auth.openrouter.main"))
        );
    }

    #[test]
    fn validate_runtime_config_accepts_known_primary_auth_ref() {
        let runtime = InstanceRuntimeConfig {
            instance_id: "global-main".to_string(),
            primary: ModelBinding {
                auth_ref: AuthRef::new("auth.openrouter.main"),
                model: "openai/gpt-5".to_string(),
            },
            fallback: None,
        };

        assert!(ConfigStore::validate_runtime_config(&runtime, &auth_pool()).is_ok());
    }

    #[test]
    fn validate_runtime_config_rejects_missing_primary_auth_ref() {
        let runtime = InstanceRuntimeConfig {
            instance_id: "global-main".to_string(),
            primary: ModelBinding {
                auth_ref: AuthRef::new("auth.missing"),
                model: "openai/gpt-5".to_string(),
            },
            fallback: None,
        };

        assert!(ConfigStore::validate_runtime_config(&runtime, &auth_pool()).is_err());
    }

    #[test]
    fn validate_runtime_config_rejects_missing_fallback_auth_ref() {
        let runtime = InstanceRuntimeConfig {
            instance_id: "global-main".to_string(),
            primary: ModelBinding {
                auth_ref: AuthRef::new("auth.openrouter.main"),
                model: "openai/gpt-5".to_string(),
            },
            fallback: Some(ModelBinding {
                auth_ref: AuthRef::new("auth.missing"),
                model: "gpt-4.1".to_string(),
            }),
        };

        assert!(ConfigStore::validate_runtime_config(&runtime, &auth_pool()).is_err());
    }

    #[test]
    fn validate_explicit_service_profile_requires_auth_ref_and_model() {
        let services = GlobalServiceProfiles {
            profiles: vec![ServiceProfile {
                role: ServiceRole::Search,
                mode: ServiceProfileMode::Explicit,
                auth_ref: None,
                model: None,
            }],
        };

        assert!(ConfigStore::validate_service_profiles(&services, &auth_pool()).is_err());
    }

    #[test]
    fn validate_explicit_service_profile_rejects_unknown_auth_ref() {
        let services = GlobalServiceProfiles {
            profiles: vec![ServiceProfile {
                role: ServiceRole::Search,
                mode: ServiceProfileMode::Explicit,
                auth_ref: Some(AuthRef::new("auth.missing")),
                model: Some("perplexity/sonar".to_string()),
            }],
        };

        assert!(ConfigStore::validate_service_profiles(&services, &auth_pool()).is_err());
    }

    #[test]
    fn validate_inherit_instance_service_profile_without_auth_or_model() {
        let services = GlobalServiceProfiles {
            profiles: vec![ServiceProfile {
                role: ServiceRole::MemoryDreamer,
                mode: ServiceProfileMode::InheritInstance,
                auth_ref: None,
                model: None,
            }],
        };

        assert!(ConfigStore::validate_service_profiles(&services, &auth_pool()).is_ok());
    }

    #[test]
    fn validate_disabled_service_profile_without_auth_or_model() {
        let services = GlobalServiceProfiles {
            profiles: vec![ServiceProfile {
                role: ServiceRole::EvolutionReviewer,
                mode: ServiceProfileMode::Disabled,
                auth_ref: None,
                model: None,
            }],
        };

        assert!(ConfigStore::validate_service_profiles(&services, &auth_pool()).is_ok());
    }
}
