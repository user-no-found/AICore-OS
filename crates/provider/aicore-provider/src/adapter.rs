use crate::{ProviderApiMode, ProviderProfile};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderAdapter;

impl ProviderAdapter {
    pub fn select_api_mode(profile: &ProviderProfile, model: &str) -> ProviderApiMode {
        match profile.provider_id.as_str() {
            "openai-codex-login" => ProviderApiMode::CodexResponses,
            "openai" => ProviderApiMode::OpenAiResponses,
            "anthropic" | "kimi-coding" | "minimax" => ProviderApiMode::AnthropicMessages,
            "openrouter" => ProviderApiMode::OpenAiChatCompletions,
            "custom-openai-compatible" => ProviderApiMode::OpenAiChatCompletions,
            "custom-anthropic-compatible" => ProviderApiMode::AnthropicMessages,
            _ => endpoint_api_mode(profile, model)
                .unwrap_or_else(|| profile.default_api_mode.clone()),
        }
    }
}

fn endpoint_api_mode(profile: &ProviderProfile, _model: &str) -> Option<ProviderApiMode> {
    let base_url = profile.default_base_url.as_ref()?;
    let lower = base_url.to_ascii_lowercase();

    if lower.contains("api.kimi.com") && lower.contains("/coding") {
        return Some(ProviderApiMode::AnthropicMessages);
    }

    if lower.trim_end_matches('/').ends_with("/anthropic") {
        return Some(ProviderApiMode::AnthropicMessages);
    }

    None
}
