use std::collections::HashMap;

use crate::{
    ProviderAdapterStatus, ProviderApiMode, ProviderAuthMode, ProviderError, ProviderProfile,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderRegistry {
    profiles: HashMap<String, ProviderProfile>,
    aliases: HashMap<String, String>,
}

impl ProviderRegistry {
    pub fn builtin() -> Self {
        let mut registry = Self {
            profiles: HashMap::new(),
            aliases: HashMap::new(),
        };

        registry.insert_profiles(builtin_profiles());
        registry.insert_aliases([
            ("claude", "anthropic"),
            ("moonshot", "kimi"),
            ("kimi-coding-cn", "kimi-coding"),
            ("deep-seek", "deepseek"),
            ("zai", "glm"),
            ("zhipu", "glm"),
            ("minimax-cn", "minimax"),
            ("codex", "openai-codex-login"),
            ("openai-compatible", "custom-openai-compatible"),
            ("anthropic-compatible", "custom-anthropic-compatible"),
            ("mimo", "xiaomi"),
            ("xiaomi-mimo", "xiaomi"),
        ]);

        registry
    }

    pub fn profile(&self, provider: &str) -> Result<&ProviderProfile, ProviderError> {
        let canonical = self.canonical_provider_id(provider);
        self.profiles
            .get(&canonical)
            .ok_or_else(|| ProviderError::Resolve(format!("unsupported provider: {canonical}")))
    }

    pub fn canonical_provider_id(&self, provider: &str) -> String {
        let normalized = normalize_provider_id(provider);
        self.aliases.get(&normalized).cloned().unwrap_or(normalized)
    }

    pub fn insert_profile(&mut self, profile: ProviderProfile) {
        self.profiles.insert(profile.provider_id.clone(), profile);
    }

    fn insert_profiles(&mut self, profiles: Vec<ProviderProfile>) {
        for profile in profiles {
            self.insert_profile(profile);
        }
    }

    fn insert_aliases<const N: usize>(&mut self, aliases: [(&str, &str); N]) {
        for (alias, provider_id) in aliases {
            self.aliases
                .insert(normalize_provider_id(alias), provider_id.to_string());
        }
    }
}

fn normalize_provider_id(provider: &str) -> String {
    provider.trim().to_ascii_lowercase().replace('_', "-")
}

fn builtin_profiles() -> Vec<ProviderProfile> {
    vec![
        provider_profile(
            "dummy",
            "dummy",
            "Dummy",
            None,
            ProviderApiMode::Dummy,
            "dummy",
            Vec::new(),
            vec![ProviderAuthMode::None, ProviderAuthMode::ApiKey],
            ProviderAdapterStatus::Available,
        ),
        provider_profile(
            "openai",
            "openai",
            "OpenAI",
            Some("https://api.openai.com/v1"),
            ProviderApiMode::OpenAiResponses,
            "python.openai",
            vec!["rust.openai_compatible_http"],
            vec![ProviderAuthMode::ApiKey],
            ProviderAdapterStatus::Available,
        ),
        provider_profile(
            "openrouter",
            "openrouter",
            "OpenRouter",
            Some("https://openrouter.ai/api/v1"),
            ProviderApiMode::OpenAiChatCompletions,
            "python.openai",
            vec!["rust.openai_compatible_http"],
            vec![ProviderAuthMode::ApiKey],
            ProviderAdapterStatus::Available,
        ),
        provider_profile(
            "anthropic",
            "anthropic",
            "Anthropic",
            Some("https://api.anthropic.com"),
            ProviderApiMode::AnthropicMessages,
            "python.anthropic",
            vec!["rust.anthropic_compatible_http"],
            vec![ProviderAuthMode::ApiKey, ProviderAuthMode::OAuth],
            ProviderAdapterStatus::Available,
        ),
        provider_profile(
            "kimi",
            "kimi",
            "Kimi",
            Some("https://api.moonshot.cn/v1"),
            ProviderApiMode::OpenAiChatCompletions,
            "python.openai",
            vec!["rust.openai_compatible_http"],
            vec![ProviderAuthMode::ApiKey],
            ProviderAdapterStatus::Available,
        ),
        provider_profile(
            "kimi-coding",
            "kimi_coding",
            "Kimi Coding",
            None,
            ProviderApiMode::AnthropicMessages,
            "python.anthropic",
            Vec::new(),
            vec![ProviderAuthMode::ApiKey, ProviderAuthMode::Session],
            ProviderAdapterStatus::Available,
        ),
        provider_profile(
            "deepseek",
            "deepseek",
            "DeepSeek",
            Some("https://api.deepseek.com/v1"),
            ProviderApiMode::OpenAiChatCompletions,
            "python.openai",
            vec!["rust.openai_compatible_http"],
            vec![ProviderAuthMode::ApiKey],
            ProviderAdapterStatus::Available,
        ),
        provider_profile(
            "glm",
            "glm",
            "智谱 GLM",
            Some("https://open.bigmodel.cn/api/paas/v4"),
            ProviderApiMode::OpenAiChatCompletions,
            "python.openai",
            vec!["rust.openai_compatible_http"],
            vec![ProviderAuthMode::ApiKey],
            ProviderAdapterStatus::Available,
        ),
        provider_profile(
            "minimax",
            "minimax",
            "MiniMax",
            None,
            ProviderApiMode::AnthropicMessages,
            "python.anthropic",
            vec!["rust.anthropic_compatible_http"],
            vec![ProviderAuthMode::ApiKey],
            ProviderAdapterStatus::Available,
        ),
        provider_profile(
            "minimax-openai",
            "minimax_openai",
            "MiniMax OpenAI Compatible",
            None,
            ProviderApiMode::OpenAiChatCompletions,
            "python.openai",
            vec!["rust.openai_compatible_http"],
            vec![ProviderAuthMode::ApiKey],
            ProviderAdapterStatus::Available,
        ),
        provider_profile(
            "openai-codex-login",
            "openai_codex_login",
            "OpenAI Codex Login",
            None,
            ProviderApiMode::CodexResponses,
            "python.codex_bridge",
            Vec::new(),
            vec![ProviderAuthMode::OAuth, ProviderAuthMode::Session],
            ProviderAdapterStatus::EngineUnavailable,
        ),
        provider_profile(
            "custom-openai-compatible",
            "custom_openai",
            "OpenAI 兼容自定义端点",
            None,
            ProviderApiMode::OpenAiChatCompletions,
            "python.openai",
            vec!["rust.openai_compatible_http"],
            vec![ProviderAuthMode::ApiKey],
            ProviderAdapterStatus::ProfileRequired,
        ),
        provider_profile(
            "custom-anthropic-compatible",
            "custom_anthropic",
            "Anthropic 兼容自定义端点",
            None,
            ProviderApiMode::AnthropicMessages,
            "python.anthropic",
            vec!["rust.anthropic_compatible_http"],
            vec![ProviderAuthMode::ApiKey],
            ProviderAdapterStatus::ProfileRequired,
        ),
        provider_profile(
            "xiaomi",
            "xiaomi",
            "Xiaomi",
            None,
            ProviderApiMode::OpenAiChatCompletions,
            "python.openai",
            vec!["rust.openai_compatible_http"],
            vec![ProviderAuthMode::ApiKey],
            ProviderAdapterStatus::ProfileRequired,
        ),
    ]
}

fn provider_profile(
    provider_id: &str,
    adapter_id: &str,
    display_name_zh: &str,
    default_base_url: Option<&str>,
    default_api_mode: ProviderApiMode,
    preferred_engine_id: &str,
    fallback_engine_ids: Vec<&str>,
    auth_modes: Vec<ProviderAuthMode>,
    status: ProviderAdapterStatus,
) -> ProviderProfile {
    ProviderProfile {
        provider_id: provider_id.to_string(),
        adapter_id: adapter_id.to_string(),
        display_name_zh: display_name_zh.to_string(),
        default_base_url: default_base_url.map(ToOwned::to_owned),
        base_url_env_var: None,
        default_api_mode,
        preferred_engine_id: preferred_engine_id.to_string(),
        fallback_engine_ids: fallback_engine_ids
            .into_iter()
            .map(ToOwned::to_owned)
            .collect(),
        auth_modes,
        capabilities: vec!["chat".to_string()],
        status,
    }
}
