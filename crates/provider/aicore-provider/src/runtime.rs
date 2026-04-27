use aicore_auth::{AuthCapability, AuthKind, GlobalAuthPool};
use aicore_config::InstanceRuntimeConfig;

use crate::{
    ProviderAdapter, ProviderAdapterStatus, ProviderAuthMode, ProviderAvailability, ProviderError,
    ProviderKind, ProviderRegistry, ProviderRuntime, ResolvedModel,
};

pub struct ProviderRuntimeResolveInput<'a> {
    pub auth_pool: &'a GlobalAuthPool,
    pub runtime: &'a InstanceRuntimeConfig,
    pub registry: &'a ProviderRegistry,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderRuntimeResolveOutput {
    pub resolved_model: ResolvedModel,
    pub provider_runtime: ProviderRuntime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderRuntimeResolver;

impl ProviderRuntimeResolver {
    pub fn resolve(
        input: ProviderRuntimeResolveInput<'_>,
    ) -> Result<ProviderRuntimeResolveOutput, ProviderError> {
        let entry = input
            .auth_pool
            .available_entries()
            .into_iter()
            .find(|entry| entry.auth_ref == input.runtime.primary.auth_ref)
            .ok_or_else(|| {
                ProviderError::Resolve(format!(
                    "missing or disabled auth_ref: {}",
                    input.runtime.primary.auth_ref.as_str()
                ))
            })?;

        if !entry.capabilities.contains(&AuthCapability::Chat) {
            return Err(ProviderError::Resolve(format!(
                "auth_ref missing required chat capability: {}",
                input.runtime.primary.auth_ref.as_str()
            )));
        }

        let provider_id = input.registry.canonical_provider_id(&entry.provider);
        let profile = input.registry.profile(&provider_id)?;

        match profile.status {
            ProviderAdapterStatus::Unsupported => {
                return Err(ProviderError::Resolve(format!(
                    "unsupported provider: {provider_id}"
                )));
            }
            ProviderAdapterStatus::ProfileRequired if profile.default_base_url.is_none() => {
                return Err(ProviderError::Resolve(format!(
                    "provider profile required: {provider_id}"
                )));
            }
            ProviderAdapterStatus::Available
            | ProviderAdapterStatus::EngineUnavailable
            | ProviderAdapterStatus::ProfileRequired => {}
        }

        let provider_runtime = ProviderRuntime {
            provider_id: provider_id.clone(),
            adapter_id: profile.adapter_id.clone(),
            engine_id: profile.preferred_engine_id.clone(),
            api_mode: ProviderAdapter::select_api_mode(profile, &input.runtime.primary.model),
            auth_mode: auth_mode(&entry.kind),
            model: input.runtime.primary.model.clone(),
            base_url: profile.default_base_url.clone(),
            auth_ref: Some(input.runtime.primary.auth_ref.clone()),
        };

        let resolved_model = ResolvedModel {
            auth_ref: input.runtime.primary.auth_ref.clone(),
            model: input.runtime.primary.model.clone(),
            provider: provider_id.clone(),
            kind: classify_provider_kind(&provider_id)?,
            availability: classify_provider_availability(&provider_id),
            runtime: provider_runtime.clone(),
        };

        Ok(ProviderRuntimeResolveOutput {
            resolved_model,
            provider_runtime,
        })
    }
}

pub fn classify_provider_kind(provider: &str) -> Result<ProviderKind, ProviderError> {
    match provider {
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

fn classify_provider_availability(provider: &str) -> ProviderAvailability {
    match provider {
        "dummy" => ProviderAvailability::Available,
        _ => ProviderAvailability::AdapterUnavailable,
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
