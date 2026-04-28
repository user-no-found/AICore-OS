use super::*;

#[test]
fn provider_resolver_resolves_primary_model() {
    let resolved = ProviderResolver::resolve_primary(&auth_pool(), &runtime_config())
        .expect("resolver should resolve primary model");

    assert_eq!(resolved.auth_ref.as_str(), "auth.dummy.main");
    assert_eq!(resolved.model, "dummy/default-chat");
    assert_eq!(resolved.provider, "dummy");
    assert_eq!(resolved.kind, ProviderKind::Dummy);
    assert_eq!(resolved.availability, ProviderAvailability::Available);
    assert_eq!(resolved.runtime.provider_id, "dummy");
    assert_eq!(resolved.runtime.api_mode, ProviderApiMode::Dummy);
    assert_eq!(resolved.runtime.engine_id, "dummy");
}

#[test]
fn provider_profile_declares_adapter_engine_and_api_mode() {
    let profile = ProviderProfile {
        provider_id: "openai".to_string(),
        adapter_id: "openai".to_string(),
        display_name_zh: "OpenAI".to_string(),
        default_base_url: Some("https://api.openai.com/v1".to_string()),
        base_url_env_var: Some("OPENAI_BASE_URL".to_string()),
        default_api_mode: ProviderApiMode::OpenAiResponses,
        preferred_engine_id: "python.openai".to_string(),
        fallback_engine_ids: vec!["rust.openai_compatible_http".to_string()],
        auth_modes: vec![ProviderAuthMode::ApiKey],
        capabilities: vec!["chat".to_string()],
        status: ProviderAdapterStatus::Available,
    };

    assert_eq!(profile.provider_id, "openai");
    assert_eq!(profile.adapter_id, "openai");
    assert_eq!(profile.default_api_mode, ProviderApiMode::OpenAiResponses);
    assert_eq!(profile.preferred_engine_id, "python.openai");
}

#[test]
fn provider_runtime_carries_provider_adapter_engine_and_model() {
    let runtime = ProviderRuntime {
        provider_id: "openrouter".to_string(),
        adapter_id: "openrouter".to_string(),
        engine_id: "python.openai".to_string(),
        api_mode: ProviderApiMode::OpenAiChatCompletions,
        auth_mode: ProviderAuthMode::ApiKey,
        model: "openai/gpt-5".to_string(),
        base_url: Some("https://openrouter.ai/api/v1".to_string()),
        auth_ref: Some(AuthRef::new("auth.openrouter.main")),
    };

    assert_eq!(runtime.provider_id, "openrouter");
    assert_eq!(runtime.adapter_id, "openrouter");
    assert_eq!(runtime.engine_id, "python.openai");
    assert_eq!(runtime.model, "openai/gpt-5");
}

#[test]
fn provider_runtime_does_not_carry_raw_secret() {
    let runtime = ProviderRuntime {
        provider_id: "openai".to_string(),
        adapter_id: "openai".to_string(),
        engine_id: "python.openai".to_string(),
        api_mode: ProviderApiMode::OpenAiResponses,
        auth_mode: ProviderAuthMode::ApiKey,
        model: "gpt-4.1".to_string(),
        base_url: None,
        auth_ref: Some(AuthRef::new("auth.openai.main")),
    };
    let rendered = format!("{runtime:?}");

    assert!(!rendered.contains("sk-live-secret-value"));
    assert!(!rendered.contains("secret://"));
}

#[test]
fn provider_registry_resolves_openai() {
    let registry = ProviderRegistry::builtin();
    let profile = registry
        .profile("openai")
        .expect("openai profile should exist");

    assert_eq!(profile.provider_id, "openai");
    assert_eq!(profile.default_api_mode, ProviderApiMode::OpenAiResponses);
    assert_eq!(profile.preferred_engine_id, "python.openai");
}

#[test]
fn provider_registry_resolves_openrouter() {
    let registry = ProviderRegistry::builtin();
    let profile = registry
        .profile("openrouter")
        .expect("openrouter profile should exist");

    assert_eq!(profile.provider_id, "openrouter");
    assert_eq!(
        profile.default_api_mode,
        ProviderApiMode::OpenAiChatCompletions
    );
    assert_eq!(profile.preferred_engine_id, "python.openai");
}

#[test]
fn provider_registry_resolves_anthropic() {
    let registry = ProviderRegistry::builtin();
    let profile = registry
        .profile("anthropic")
        .expect("anthropic profile should exist");

    assert_eq!(profile.provider_id, "anthropic");
    assert_eq!(profile.default_api_mode, ProviderApiMode::AnthropicMessages);
    assert_eq!(profile.preferred_engine_id, "python.anthropic");
}

#[test]
fn provider_registry_resolves_kimi_alias() {
    let registry = ProviderRegistry::builtin();

    assert_eq!(registry.canonical_provider_id("moonshot"), "kimi");
    assert_eq!(
        registry
            .profile("moonshot")
            .expect("moonshot alias should resolve")
            .provider_id,
        "kimi"
    );
}

#[test]
fn provider_registry_resolves_glm_aliases() {
    let registry = ProviderRegistry::builtin();

    assert_eq!(registry.canonical_provider_id("zai"), "glm");
    assert_eq!(registry.canonical_provider_id("zhipu"), "glm");
}

#[test]
fn provider_registry_keeps_codex_login_separate_from_openai() {
    let registry = ProviderRegistry::builtin();
    let profile = registry
        .profile("codex")
        .expect("codex alias should resolve");

    assert_eq!(profile.provider_id, "openai-codex-login");
    assert_eq!(profile.default_api_mode, ProviderApiMode::CodexResponses);
    assert_ne!(profile.provider_id, "openai");
}

#[test]
fn provider_registry_marks_xiaomi_profile_required_without_base_url() {
    let registry = ProviderRegistry::builtin();
    let profile = registry.profile("xiaomi").expect("xiaomi skeleton exists");

    assert_eq!(profile.provider_id, "xiaomi");
    assert_eq!(profile.status, ProviderAdapterStatus::ProfileRequired);
    assert_eq!(profile.default_base_url, None);
}

#[test]
fn unknown_provider_returns_resolve_error() {
    let registry = ProviderRegistry::builtin();
    let error = registry
        .profile("mystery")
        .expect_err("unknown provider should fail");

    assert!(matches!(error, crate::ProviderError::Resolve(_)));
}

#[test]
fn provider_registry_applies_custom_openai_endpoint_override() {
    let registry = ProviderRegistry::with_overrides(&ProviderProfilesConfig {
        profiles: vec![ProviderProfileOverride {
            provider_id: "custom-openai-compatible".to_string(),
            base_url: Some("http://localhost:11434/v1".to_string()),
            api_mode: Some("openai_chat_completions".to_string()),
            engine_id: Some("python.openai".to_string()),
            enabled: true,
        }],
    });
    let profile = registry
        .profile("custom-openai-compatible")
        .expect("custom endpoint should be enabled by override");

    assert_eq!(
        profile.default_base_url.as_deref(),
        Some("http://localhost:11434/v1")
    );
    assert_eq!(profile.status, ProviderAdapterStatus::Available);
}

#[test]
fn provider_registry_disabled_override_is_unsupported() {
    let registry = ProviderRegistry::with_overrides(&ProviderProfilesConfig {
        profiles: vec![ProviderProfileOverride {
            provider_id: "openai".to_string(),
            base_url: None,
            api_mode: None,
            engine_id: None,
            enabled: false,
        }],
    });
    let profile = registry
        .profile("openai")
        .expect("profile should still exist");

    assert_eq!(profile.status, ProviderAdapterStatus::Unsupported);
}

#[test]
fn provider_registry_override_can_enable_xiaomi_with_base_url() {
    let registry = ProviderRegistry::with_overrides(&ProviderProfilesConfig {
        profiles: vec![ProviderProfileOverride {
            provider_id: "xiaomi".to_string(),
            base_url: Some("https://api.example.xiaomi.invalid/v1".to_string()),
            api_mode: Some("openai_chat_completions".to_string()),
            engine_id: Some("python.openai".to_string()),
            enabled: true,
        }],
    });
    let profile = registry
        .profile("xiaomi")
        .expect("xiaomi profile should exist");

    assert_eq!(profile.status, ProviderAdapterStatus::Available);
    assert_eq!(
        profile.default_base_url.as_deref(),
        Some("https://api.example.xiaomi.invalid/v1")
    );
}
