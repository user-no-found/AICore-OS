use aicore_auth::{AuthCapability, AuthKind, GlobalAuthPool};
use aicore_config::InstanceRuntimeConfig;

use crate::{
    ProviderAdapterStatus, ProviderAuthMode, ProviderAvailability, ProviderError, ProviderKind,
    ProviderRegistry, ProviderRuntime, ResolvedModel,
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

        let registry = ProviderRegistry::builtin();
        let provider_id = registry.canonical_provider_id(&entry.provider);
        let profile = registry.profile(&provider_id)?;

        match profile.status {
            ProviderAdapterStatus::Unsupported => {
                return Err(ProviderError::Resolve(format!(
                    "unsupported provider: {provider_id}"
                )));
            }
            ProviderAdapterStatus::ProfileRequired => {
                return Err(ProviderError::Resolve(format!(
                    "provider profile required: {provider_id}"
                )));
            }
            ProviderAdapterStatus::Available | ProviderAdapterStatus::EngineUnavailable => {}
        }

        Ok(ResolvedModel {
            auth_ref: runtime.primary.auth_ref.clone(),
            model: runtime.primary.model.clone(),
            provider: provider_id.clone(),
            kind: classify_provider_kind(&provider_id)?,
            availability: classify_provider_availability(&provider_id)?,
            runtime: ProviderRuntime {
                provider_id: provider_id.clone(),
                adapter_id: profile.adapter_id.clone(),
                engine_id: profile.preferred_engine_id.clone(),
                api_mode: profile.default_api_mode.clone(),
                auth_mode: auth_mode(&entry.kind),
                model: runtime.primary.model.clone(),
                base_url: profile.default_base_url.clone(),
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
        "anthropic" => Ok(ProviderKind::Anthropic),
        "kimi" => Ok(ProviderKind::Kimi),
        "kimi-coding" => Ok(ProviderKind::KimiCoding),
        "deepseek" => Ok(ProviderKind::DeepSeek),
        "glm" => Ok(ProviderKind::Glm),
        "minimax" => Ok(ProviderKind::MiniMax),
        "minimax-openai" => Ok(ProviderKind::MiniMaxOpenAI),
        "openai-codex-login" => Ok(ProviderKind::OpenAICodexLogin),
        "custom-openai-compatible" => Ok(ProviderKind::CustomOpenAICompatible),
        "custom-anthropic-compatible" => Ok(ProviderKind::CustomAnthropicCompatible),
        "xiaomi" => Ok(ProviderKind::Xiaomi),
        other => Err(ProviderError::Resolve(format!(
            "unsupported provider: {other}"
        ))),
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
        "openrouter"
        | "openai"
        | "anthropic"
        | "kimi"
        | "kimi-coding"
        | "deepseek"
        | "glm"
        | "minimax"
        | "minimax-openai"
        | "openai-codex-login"
        | "custom-openai-compatible"
        | "custom-anthropic-compatible"
        | "xiaomi" => Ok(ProviderAvailability::AdapterUnavailable),
        other => Err(ProviderError::Resolve(format!(
            "unsupported provider: {other}"
        ))),
    }
}
