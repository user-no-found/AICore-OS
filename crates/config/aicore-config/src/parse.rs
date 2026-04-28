use std::collections::HashMap;

use aicore_auth::{AuthCapability, AuthEntry, AuthKind, AuthRef, GlobalAuthPool, SecretRef};

use crate::render::{
    render_auth_capability, render_auth_kind, render_service_profile_mode, render_service_role,
};
use crate::{
    ConfigError, GlobalServiceProfiles, InstanceRuntimeConfig, ModelBinding,
    ProviderProfileOverride, ProviderProfilesConfig, ServiceProfile, ServiceProfileMode,
    ServiceRole,
};

pub(crate) fn parse_auth_pool(contents: &str) -> Result<GlobalAuthPool, ConfigError> {
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

pub(crate) fn parse_services(contents: &str) -> Result<GlobalServiceProfiles, ConfigError> {
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

pub(crate) fn parse_provider_profiles(
    contents: &str,
) -> Result<ProviderProfilesConfig, ConfigError> {
    let mut profiles = Vec::new();

    for section in parse_sections(contents, "[[provider]]")? {
        let fields = parse_key_values(&section)?;
        profiles.push(ProviderProfileOverride {
            provider_id: required_string_field(&fields, "provider_id")?,
            base_url: optional_field(&fields, "base_url"),
            api_mode: optional_field(&fields, "api_mode"),
            engine_id: optional_field(&fields, "engine_id"),
            enabled: fields
                .get("enabled")
                .map(|value| parse_bool(value))
                .transpose()?
                .unwrap_or(true),
        });
    }

    Ok(ProviderProfilesConfig { profiles })
}

pub(crate) fn parse_runtime(contents: &str) -> Result<InstanceRuntimeConfig, ConfigError> {
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

fn parse_auth_kind(value: &str) -> Result<AuthKind, ConfigError> {
    match value {
        _ if value == render_auth_kind(&AuthKind::ApiKey) => Ok(AuthKind::ApiKey),
        _ if value == render_auth_kind(&AuthKind::OAuth) => Ok(AuthKind::OAuth),
        _ if value == render_auth_kind(&AuthKind::Session) => Ok(AuthKind::Session),
        _ if value == render_auth_kind(&AuthKind::Token) => Ok(AuthKind::Token),
        other => Err(ConfigError::Parse(format!("unknown auth kind: {other}"))),
    }
}

fn parse_auth_capability(value: &str) -> Result<AuthCapability, ConfigError> {
    match value {
        _ if value == render_auth_capability(&AuthCapability::Chat) => Ok(AuthCapability::Chat),
        _ if value == render_auth_capability(&AuthCapability::Vision) => Ok(AuthCapability::Vision),
        _ if value == render_auth_capability(&AuthCapability::Search) => Ok(AuthCapability::Search),
        _ if value == render_auth_capability(&AuthCapability::Embedding) => {
            Ok(AuthCapability::Embedding)
        }
        other => Err(ConfigError::Parse(format!(
            "unknown auth capability: {other}"
        ))),
    }
}

fn parse_service_role(value: &str) -> Result<ServiceRole, ConfigError> {
    match value {
        _ if value == render_service_role(&ServiceRole::MemoryExtractor) => {
            Ok(ServiceRole::MemoryExtractor)
        }
        _ if value == render_service_role(&ServiceRole::MemoryCurator) => {
            Ok(ServiceRole::MemoryCurator)
        }
        _ if value == render_service_role(&ServiceRole::MemoryDreamer) => {
            Ok(ServiceRole::MemoryDreamer)
        }
        _ if value == render_service_role(&ServiceRole::EvolutionProposer) => {
            Ok(ServiceRole::EvolutionProposer)
        }
        _ if value == render_service_role(&ServiceRole::EvolutionReviewer) => {
            Ok(ServiceRole::EvolutionReviewer)
        }
        _ if value == render_service_role(&ServiceRole::Search) => Ok(ServiceRole::Search),
        _ if value == render_service_role(&ServiceRole::Tts) => Ok(ServiceRole::Tts),
        _ if value == render_service_role(&ServiceRole::ImageGeneration) => {
            Ok(ServiceRole::ImageGeneration)
        }
        _ if value == render_service_role(&ServiceRole::VideoGeneration) => {
            Ok(ServiceRole::VideoGeneration)
        }
        _ if value == render_service_role(&ServiceRole::Vision) => Ok(ServiceRole::Vision),
        _ if value == render_service_role(&ServiceRole::Reranker) => Ok(ServiceRole::Reranker),
        other => Err(ConfigError::Parse(format!("unknown service role: {other}"))),
    }
}

fn parse_service_profile_mode(value: &str) -> Result<ServiceProfileMode, ConfigError> {
    match value {
        _ if value == render_service_profile_mode(&ServiceProfileMode::InheritInstance) => {
            Ok(ServiceProfileMode::InheritInstance)
        }
        _ if value == render_service_profile_mode(&ServiceProfileMode::Explicit) => {
            Ok(ServiceProfileMode::Explicit)
        }
        _ if value == render_service_profile_mode(&ServiceProfileMode::Disabled) => {
            Ok(ServiceProfileMode::Disabled)
        }
        other => Err(ConfigError::Parse(format!(
            "unknown service profile mode: {other}"
        ))),
    }
}
