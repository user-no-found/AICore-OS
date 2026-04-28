use super::*;

#[test]
fn runtime_resolver_routes_openai_to_openai_responses() {
    let runtime = resolve_runtime("openai", "gpt-4.1");

    assert_eq!(runtime.api_mode, ProviderApiMode::OpenAiResponses);
    assert_eq!(runtime.engine_id, "python.openai");
}

#[test]
fn runtime_resolver_routes_openrouter_to_openai_chat_completions() {
    let runtime = resolve_runtime("openrouter", "anthropic/claude-sonnet");

    assert_eq!(runtime.api_mode, ProviderApiMode::OpenAiChatCompletions);
    assert_eq!(runtime.engine_id, "python.openai");
}

#[test]
fn runtime_resolver_routes_anthropic_to_anthropic_messages() {
    let runtime = resolve_runtime("anthropic", "claude-sonnet-4-5");

    assert_eq!(runtime.api_mode, ProviderApiMode::AnthropicMessages);
    assert_eq!(runtime.engine_id, "python.anthropic");
}

#[test]
fn runtime_resolver_routes_kimi_to_openai_chat_completions() {
    let runtime = resolve_runtime("kimi", "moonshot-v1-8k");

    assert_eq!(runtime.api_mode, ProviderApiMode::OpenAiChatCompletions);
    assert_eq!(runtime.engine_id, "python.openai");
}

#[test]
fn runtime_resolver_routes_kimi_coding_to_anthropic_messages() {
    let runtime = resolve_runtime("kimi-coding", "kimi-k2-coder");

    assert_eq!(runtime.api_mode, ProviderApiMode::AnthropicMessages);
    assert_eq!(runtime.engine_id, "python.anthropic");
}

#[test]
fn runtime_resolver_routes_deepseek_to_openai_chat_completions() {
    let runtime = resolve_runtime("deepseek", "deepseek-chat");

    assert_eq!(runtime.api_mode, ProviderApiMode::OpenAiChatCompletions);
}

#[test]
fn runtime_resolver_routes_glm_to_openai_chat_completions() {
    let runtime = resolve_runtime("glm", "glm-4.6");

    assert_eq!(runtime.api_mode, ProviderApiMode::OpenAiChatCompletions);
}

#[test]
fn runtime_resolver_routes_minimax_to_anthropic_messages() {
    let runtime = resolve_runtime("minimax", "MiniMax-M2");

    assert_eq!(runtime.api_mode, ProviderApiMode::AnthropicMessages);
}

#[test]
fn runtime_resolver_keeps_codex_login_separate() {
    let runtime = resolve_runtime("openai-codex-login", "gpt-5-codex");

    assert_eq!(runtime.api_mode, ProviderApiMode::CodexResponses);
    assert_eq!(runtime.engine_id, "python.codex_bridge");
}

#[test]
fn runtime_resolver_uses_custom_openai_compatible_profile() {
    let auth_pool = auth_pool_for_provider("custom-openai-compatible");
    let runtime = runtime_for_model("llama-local");
    let registry = ProviderRegistry::with_overrides(&ProviderProfilesConfig {
        profiles: vec![ProviderProfileOverride {
            provider_id: "custom-openai-compatible".to_string(),
            base_url: Some("http://localhost:11434/v1".to_string()),
            api_mode: Some("openai_chat_completions".to_string()),
            engine_id: Some("python.openai".to_string()),
            enabled: true,
        }],
    });

    let output = ProviderRuntimeResolver::resolve(ProviderRuntimeResolveInput {
        auth_pool: &auth_pool,
        runtime: &runtime,
        registry: &registry,
    })
    .expect("custom openai endpoint should resolve");

    assert_eq!(
        output.provider_runtime.api_mode,
        ProviderApiMode::OpenAiChatCompletions
    );
    assert_eq!(
        output.provider_runtime.base_url.as_deref(),
        Some("http://localhost:11434/v1")
    );
}

#[test]
fn runtime_resolver_uses_custom_anthropic_compatible_profile() {
    let auth_pool = auth_pool_for_provider("custom-anthropic-compatible");
    let runtime = runtime_for_model("claude-compatible");
    let registry = ProviderRegistry::with_overrides(&ProviderProfilesConfig {
        profiles: vec![ProviderProfileOverride {
            provider_id: "custom-anthropic-compatible".to_string(),
            base_url: Some("http://localhost:8080/anthropic".to_string()),
            api_mode: Some("anthropic_messages".to_string()),
            engine_id: Some("python.anthropic".to_string()),
            enabled: true,
        }],
    });

    let output = ProviderRuntimeResolver::resolve(ProviderRuntimeResolveInput {
        auth_pool: &auth_pool,
        runtime: &runtime,
        registry: &registry,
    })
    .expect("custom anthropic endpoint should resolve");

    assert_eq!(
        output.provider_runtime.api_mode,
        ProviderApiMode::AnthropicMessages
    );
    assert_eq!(output.provider_runtime.engine_id, "python.anthropic");
}

#[test]
fn runtime_resolver_rejects_xiaomi_without_base_url() {
    let auth_pool = auth_pool_for_provider("xiaomi");
    let runtime = runtime_for_model("mimo");
    let registry = ProviderRegistry::builtin();

    let error = ProviderRuntimeResolver::resolve(ProviderRuntimeResolveInput {
        auth_pool: &auth_pool,
        runtime: &runtime,
        registry: &registry,
    })
    .expect_err("xiaomi should require explicit profile");

    assert!(matches!(error, crate::ProviderError::Resolve(_)));
}

#[test]
fn runtime_resolver_allows_xiaomi_with_explicit_profile_base_url() {
    let auth_pool = auth_pool_for_provider("xiaomi");
    let runtime = runtime_for_model("mimo");
    let registry = ProviderRegistry::with_overrides(&ProviderProfilesConfig {
        profiles: vec![ProviderProfileOverride {
            provider_id: "xiaomi".to_string(),
            base_url: Some("https://api.example.xiaomi.invalid/v1".to_string()),
            api_mode: Some("openai_chat_completions".to_string()),
            engine_id: Some("python.openai".to_string()),
            enabled: true,
        }],
    });

    let output = ProviderRuntimeResolver::resolve(ProviderRuntimeResolveInput {
        auth_pool: &auth_pool,
        runtime: &runtime,
        registry: &registry,
    })
    .expect("xiaomi with explicit base_url should resolve");

    assert_eq!(
        output.provider_runtime.api_mode,
        ProviderApiMode::OpenAiChatCompletions
    );
    assert_eq!(
        output.provider_runtime.base_url.as_deref(),
        Some("https://api.example.xiaomi.invalid/v1")
    );
}

#[test]
fn runtime_resolver_rejects_non_chat_auth() {
    let auth_pool = GlobalAuthPool::new(vec![AuthEntry {
        auth_ref: AuthRef::new("auth.test.main"),
        provider: "openai".to_string(),
        kind: AuthKind::ApiKey,
        secret_ref: SecretRef::new("secret://auth.test.main"),
        capabilities: vec![AuthCapability::Embedding],
        enabled: true,
    }]);
    let runtime = runtime_for_model("gpt-4.1");
    let registry = ProviderRegistry::builtin();

    let error = ProviderRuntimeResolver::resolve(ProviderRuntimeResolveInput {
        auth_pool: &auth_pool,
        runtime: &runtime,
        registry: &registry,
    })
    .expect_err("non-chat auth should fail");

    assert!(matches!(error, crate::ProviderError::Resolve(_)));
}

#[test]
fn runtime_resolver_does_not_include_secret_ref_in_public_error() {
    let auth_pool = auth_pool_for_provider("xiaomi");
    let runtime = runtime_for_model("mimo");
    let registry = ProviderRegistry::builtin();

    let error = ProviderRuntimeResolver::resolve(ProviderRuntimeResolveInput {
        auth_pool: &auth_pool,
        runtime: &runtime,
        registry: &registry,
    })
    .expect_err("xiaomi should require profile");
    let rendered = format!("{error:?}");

    assert!(!rendered.contains("secret://"));
    assert!(!rendered.contains("auth.test.main"));
}
