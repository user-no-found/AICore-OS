use aicore_auth::GlobalAuthPool;
use aicore_config::InstanceRuntimeConfig;

use crate::{ProviderError, ProviderKind, ResolvedModel};

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

        Ok(ResolvedModel {
            auth_ref: runtime.primary.auth_ref.clone(),
            model: runtime.primary.model.clone(),
            provider: entry.provider.clone(),
            kind: classify_provider_kind(&entry.provider)?,
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
