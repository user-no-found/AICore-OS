use aicore_auth::{AuthCapability, AuthKind, GlobalAuthPool};
use aicore_config::InstanceRuntimeConfig;

use crate::{
    ProviderApiMode, ProviderAuthMode, ProviderAvailability, ProviderError, ProviderKind,
    ProviderRuntime, ResolvedModel,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderResolver;

impl ProviderResolver {
    pub fn resolve_primary(
        auth_pool: &GlobalAuthPool,
        runtime: &InstanceRuntimeConfig,
    ) -> Result<ResolvedModel, ProviderError> {
        let entry = auth_pool
            .available_entries()
            .into_iter()
            .find(|entry| entry.auth_ref == runtime.primary.auth_ref)
            .ok_or_else(|| {
                ProviderError::Resolve(format!(
                    "missing or disabled auth_ref: {}",
                    runtime.primary.auth_ref.as_str()
                ))
            })?;

        if !entry.capabilities.contains(&AuthCapability::Chat) {
            return Err(ProviderError::Resolve(format!(
                "auth_ref missing required chat capability: {}",
                runtime.primary.auth_ref.as_str()
            )));
        }

        Ok(ResolvedModel {
            auth_ref: runtime.primary.auth_ref.clone(),
            model: runtime.primary.model.clone(),
            provider: entry.provider.clone(),
            kind: classify_provider_kind(&entry.provider)?,
            availability: classify_provider_availability(&entry.provider)?,
            runtime: ProviderRuntime {
                provider_id: entry.provider.to_ascii_lowercase(),
                adapter_id: entry.provider.to_ascii_lowercase(),
                engine_id: default_engine_id(&entry.provider).to_string(),
                api_mode: default_api_mode(&entry.provider)?,
                auth_mode: auth_mode(&entry.kind),
                model: runtime.primary.model.clone(),
                base_url: None,
                auth_ref: Some(runtime.primary.auth_ref.clone()),
            },
        })
    }
}

fn classify_provider_kind(provider: &str) -> Result<ProviderKind, ProviderError> {
    match provider.to_ascii_lowercase().as_str() {
        "dummy" => Ok(ProviderKind::Dummy),
        "openrouter" => Ok(ProviderKind::OpenRouter),
        "openai" => Ok(ProviderKind::OpenAI),
        other => Err(ProviderError::Resolve(format!(
            "unsupported provider: {other}"
        ))),
    }
}

fn default_api_mode(provider: &str) -> Result<ProviderApiMode, ProviderError> {
    match provider.to_ascii_lowercase().as_str() {
        "dummy" => Ok(ProviderApiMode::Dummy),
        "openrouter" => Ok(ProviderApiMode::OpenAiChatCompletions),
        "openai" => Ok(ProviderApiMode::OpenAiResponses),
        other => Err(ProviderError::Resolve(format!(
            "unsupported provider: {other}"
        ))),
    }
}

fn default_engine_id(provider: &str) -> &'static str {
    match provider.to_ascii_lowercase().as_str() {
        "dummy" => "dummy",
        "openrouter" | "openai" => "python.openai",
        _ => "unavailable",
    }
}

fn auth_mode(kind: &AuthKind) -> ProviderAuthMode {
    match kind {
        AuthKind::ApiKey => ProviderAuthMode::ApiKey,
        AuthKind::OAuth => ProviderAuthMode::OAuth,
        AuthKind::Session => ProviderAuthMode::Session,
        AuthKind::Token => ProviderAuthMode::OAuth,
    }
}

fn classify_provider_availability(provider: &str) -> Result<ProviderAvailability, ProviderError> {
    match provider.to_ascii_lowercase().as_str() {
        "dummy" => Ok(ProviderAvailability::Available),
        "openrouter" | "openai" => Ok(ProviderAvailability::AdapterUnavailable),
        other => Err(ProviderError::Resolve(format!(
            "unsupported provider: {other}"
        ))),
    }
}
