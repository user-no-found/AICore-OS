mod adapter;
mod dummy;
mod engine_ipc;
mod invoker;
mod profile;
mod prompt;
mod resolver;
mod runtime;
mod types;

pub use adapter::ProviderAdapter;
pub use dummy::DummyProvider;
pub use engine_ipc::{
    ProviderEngineEvent, ProviderEngineEventKind, ProviderEngineMessage, ProviderEngineRequest,
};
pub use invoker::ProviderInvoker;
pub use profile::ProviderRegistry;
pub use prompt::PromptBuilder;
pub use resolver::ProviderResolver;
pub use runtime::{
    ProviderRuntimeResolveInput, ProviderRuntimeResolveOutput, ProviderRuntimeResolver,
};
pub use types::{
    ModelRequest, ModelResponse, PromptBuildInput, PromptBuildResult, ProviderAdapterStatus,
    ProviderApiMode, ProviderAuthMode, ProviderAvailability, ProviderDescriptor, ProviderError,
    ProviderKind, ProviderProfile, ProviderRuntime, RequestEngineKind, ResolvedModel,
};

#[cfg(test)]
mod tests {
    use aicore_auth::{AuthCapability, AuthEntry, AuthKind, AuthRef, GlobalAuthPool, SecretRef};
    use aicore_config::{
        InstanceRuntimeConfig, ModelBinding, ProviderProfileOverride, ProviderProfilesConfig,
    };
    use aicore_memory::{
        MemoryKernel, MemoryPaths, MemoryPermanence, MemoryScope, MemoryType, RememberInput,
        SearchQuery,
    };
    use std::{
        env, fs,
        io::Write,
        process::{Command, Stdio},
    };

    use crate::{
        ModelRequest, PromptBuildInput, PromptBuilder, ProviderAdapterStatus, ProviderApiMode,
        ProviderAuthMode, ProviderAvailability, ProviderEngineEvent, ProviderEngineEventKind,
        ProviderEngineMessage, ProviderEngineRequest, ProviderInvoker, ProviderKind,
        ProviderProfile, ProviderRegistry, ProviderResolver, ProviderRuntime,
        ProviderRuntimeResolveInput, ProviderRuntimeResolver,
    };

    fn auth_pool() -> GlobalAuthPool {
        GlobalAuthPool::new(vec![
            AuthEntry {
                auth_ref: AuthRef::new("auth.dummy.main"),
                provider: "dummy".to_string(),
                kind: AuthKind::ApiKey,
                secret_ref: SecretRef::new("secret://auth.dummy.main"),
                capabilities: vec![AuthCapability::Chat],
                enabled: true,
            },
            AuthEntry {
                auth_ref: AuthRef::new("auth.openrouter.main"),
                provider: "openrouter".to_string(),
                kind: AuthKind::ApiKey,
                secret_ref: SecretRef::new("secret://auth.openrouter.main"),
                capabilities: vec![AuthCapability::Chat],
                enabled: true,
            },
            AuthEntry {
                auth_ref: AuthRef::new("auth.openai.main"),
                provider: "openai".to_string(),
                kind: AuthKind::ApiKey,
                secret_ref: SecretRef::new("secret://auth.openai.main"),
                capabilities: vec![AuthCapability::Chat],
                enabled: true,
            },
        ])
    }

    fn auth_pool_with_disabled_entry() -> GlobalAuthPool {
        GlobalAuthPool::new(vec![AuthEntry {
            auth_ref: AuthRef::new("auth.dummy.main"),
            provider: "dummy".to_string(),
            kind: AuthKind::ApiKey,
            secret_ref: SecretRef::new("secret://auth.dummy.main"),
            capabilities: vec![AuthCapability::Chat],
            enabled: false,
        }])
    }

    fn runtime_config() -> InstanceRuntimeConfig {
        InstanceRuntimeConfig {
            instance_id: "global-main".to_string(),
            primary: ModelBinding {
                auth_ref: AuthRef::new("auth.dummy.main"),
                model: "dummy/default-chat".to_string(),
            },
            fallback: None,
        }
    }

    fn runtime_config_openrouter() -> InstanceRuntimeConfig {
        InstanceRuntimeConfig {
            instance_id: "global-main".to_string(),
            primary: ModelBinding {
                auth_ref: AuthRef::new("auth.openrouter.main"),
                model: "openai/gpt-5".to_string(),
            },
            fallback: None,
        }
    }

    fn runtime_config_openai() -> InstanceRuntimeConfig {
        InstanceRuntimeConfig {
            instance_id: "global-main".to_string(),
            primary: ModelBinding {
                auth_ref: AuthRef::new("auth.openai.main"),
                model: "gpt-4.1".to_string(),
            },
            fallback: None,
        }
    }

    fn auth_pool_for_provider(provider: &str) -> GlobalAuthPool {
        GlobalAuthPool::new(vec![AuthEntry {
            auth_ref: AuthRef::new("auth.test.main"),
            provider: provider.to_string(),
            kind: AuthKind::ApiKey,
            secret_ref: SecretRef::new("secret://auth.test.main"),
            capabilities: vec![AuthCapability::Chat],
            enabled: true,
        }])
    }

    fn runtime_for_model(model: &str) -> InstanceRuntimeConfig {
        InstanceRuntimeConfig {
            instance_id: "global-main".to_string(),
            primary: ModelBinding {
                auth_ref: AuthRef::new("auth.test.main"),
                model: model.to_string(),
            },
            fallback: None,
        }
    }

    fn resolve_runtime(provider: &str, model: &str) -> crate::ProviderRuntime {
        let auth_pool = auth_pool_for_provider(provider);
        let runtime = runtime_for_model(model);
        let registry = ProviderRegistry::builtin();

        ProviderRuntimeResolver::resolve(ProviderRuntimeResolveInput {
            auth_pool: &auth_pool,
            runtime: &runtime,
            registry: &registry,
        })
        .expect("runtime should resolve")
        .provider_runtime
    }

    fn python3_available() -> bool {
        Command::new("python3")
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    fn run_fake_worker(content: &str) -> Option<Vec<ProviderEngineEvent>> {
        if !python3_available() {
            eprintln!("python3 unavailable; skipping fake worker smoke");
            return None;
        }

        let request = ProviderEngineRequest {
            protocol_version: "provider.engine.v1".to_string(),
            invocation_id: "inv-fake".to_string(),
            provider_id: "dummy".to_string(),
            adapter_id: "dummy".to_string(),
            engine_id: "python.fake".to_string(),
            api_mode: "dummy".to_string(),
            model: "dummy/default-chat".to_string(),
            base_url: None,
            credential_lease_ref: None,
            messages: vec![ProviderEngineMessage {
                role: "user".to_string(),
                content: content.to_string(),
            }],
            tools_json: None,
            parameters_json: None,
            stream: false,
            timeout_ms: None,
        };
        let request_json = serde_json::to_string(&request).expect("request should serialize");
        let python_root = format!("{}/python", env!("CARGO_MANIFEST_DIR"));
        let mut child = Command::new("python3")
            .arg("-m")
            .arg("aicore_provider_engine.worker")
            .arg("--engine")
            .arg("fake")
            .env("PYTHONPATH", python_root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("fake worker should spawn");

        child
            .stdin
            .as_mut()
            .expect("stdin should be available")
            .write_all(format!("{request_json}\n").as_bytes())
            .expect("request should be written");
        drop(child.stdin.take());

        let output = child.wait_with_output().expect("worker should finish");
        assert!(
            output.status.success(),
            "worker failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
        let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
        assert!(!stdout.contains("sk-live-secret-value"));
        assert!(!stderr.contains("sk-live-secret-value"));

        Some(
            stdout
                .lines()
                .map(|line| serde_json::from_str(line).expect("event line should parse"))
                .collect(),
        )
    }

    fn run_sdk_worker_with_env(
        engine: &str,
        api_mode: &str,
        extra_env: &[(&str, &str)],
    ) -> Option<(Vec<ProviderEngineEvent>, String, String)> {
        if !python3_available() {
            eprintln!("python3 unavailable; skipping SDK worker smoke");
            return None;
        }

        let request = ProviderEngineRequest {
            protocol_version: "provider.engine.v1".to_string(),
            invocation_id: format!("inv-{engine}"),
            provider_id: engine.to_string(),
            adapter_id: engine.to_string(),
            engine_id: format!("python.{engine}"),
            api_mode: api_mode.to_string(),
            model: "test-model".to_string(),
            base_url: None,
            credential_lease_ref: Some("env:AICORE_PROVIDER_TEST_SECRET".to_string()),
            messages: vec![ProviderEngineMessage {
                role: "user".to_string(),
                content: "ping".to_string(),
            }],
            tools_json: None,
            parameters_json: None,
            stream: false,
            timeout_ms: None,
        };
        let request_json = serde_json::to_string(&request).expect("request should serialize");
        let python_root = format!("{}/python", env!("CARGO_MANIFEST_DIR"));
        let mut command = Command::new("python3");
        command
            .arg("-m")
            .arg("aicore_provider_engine.worker")
            .arg("--engine")
            .arg(engine)
            .env("PYTHONPATH", python_root)
            .env("AICORE_PROVIDER_TEST_SECRET", "sk-live-secret-value")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        for (key, value) in extra_env {
            command.env(key, value);
        }

        let mut child = command.spawn().expect("SDK worker should spawn");
        child
            .stdin
            .as_mut()
            .expect("stdin should be available")
            .write_all(format!("{request_json}\n").as_bytes())
            .expect("request should be written");
        drop(child.stdin.take());

        let output = child.wait_with_output().expect("worker should finish");
        assert!(
            output.status.success(),
            "worker failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
        let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
        let events = stdout
            .lines()
            .map(|line| serde_json::from_str(line).expect("event line should parse"))
            .collect();

        Some((events, stdout, stderr))
    }

    fn temp_paths(name: &str) -> MemoryPaths {
        let root = env::temp_dir().join(format!("aicore-provider-tests-{name}"));
        if root.exists() {
            fs::remove_dir_all(&root).expect("temp root should be removable");
        }
        MemoryPaths::new(root)
    }

    fn global_scope() -> MemoryScope {
        MemoryScope::GlobalMain {
            instance_id: "global-main".to_string(),
        }
    }

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

    #[test]
    fn engine_request_serializes_without_raw_secret() {
        let request = ProviderEngineRequest {
            protocol_version: "provider.engine.v1".to_string(),
            invocation_id: "inv-1".to_string(),
            provider_id: "openai".to_string(),
            adapter_id: "openai".to_string(),
            engine_id: "python.openai".to_string(),
            api_mode: "openai_responses".to_string(),
            model: "gpt-4.1".to_string(),
            base_url: None,
            credential_lease_ref: Some("lease:auth.openai.main".to_string()),
            messages: vec![ProviderEngineMessage {
                role: "user".to_string(),
                content: "ping".to_string(),
            }],
            tools_json: None,
            parameters_json: None,
            stream: false,
            timeout_ms: Some(30_000),
        };

        let rendered = serde_json::to_string(&request).expect("request should serialize");

        assert!(rendered.contains("credential_lease_ref"));
        assert!(!rendered.contains("sk-live-secret-value"));
        assert!(!rendered.contains("secret://"));
    }

    #[test]
    fn engine_event_round_trips_message_delta() {
        let event = ProviderEngineEvent {
            protocol_version: "provider.engine.v1".to_string(),
            invocation_id: "inv-1".to_string(),
            kind: ProviderEngineEventKind::MessageDelta,
            content: Some("hello".to_string()),
            payload_json: None,
            user_message_zh: None,
            machine_code: None,
        };

        let encoded = serde_json::to_string(&event).expect("event should serialize");
        let decoded: ProviderEngineEvent =
            serde_json::from_str(&encoded).expect("event should deserialize");

        assert_eq!(decoded, event);
    }

    #[test]
    fn engine_event_round_trips_error_with_chinese_user_message() {
        let event = ProviderEngineEvent {
            protocol_version: "provider.engine.v1".to_string(),
            invocation_id: "inv-1".to_string(),
            kind: ProviderEngineEventKind::Error,
            content: None,
            payload_json: None,
            user_message_zh: Some("Provider 请求失败".to_string()),
            machine_code: Some("provider_error".to_string()),
        };

        let encoded = serde_json::to_string(&event).expect("event should serialize");
        let decoded: ProviderEngineEvent =
            serde_json::from_str(&encoded).expect("event should deserialize");

        assert_eq!(
            decoded.user_message_zh.as_deref(),
            Some("Provider 请求失败")
        );
        assert_eq!(decoded.machine_code.as_deref(), Some("provider_error"));
    }

    #[test]
    fn engine_request_jsonl_is_single_line() {
        let request = ProviderEngineRequest {
            protocol_version: "provider.engine.v1".to_string(),
            invocation_id: "inv-1".to_string(),
            provider_id: "dummy".to_string(),
            adapter_id: "dummy".to_string(),
            engine_id: "python.fake".to_string(),
            api_mode: "dummy".to_string(),
            model: "dummy/default-chat".to_string(),
            base_url: None,
            credential_lease_ref: None,
            messages: vec![ProviderEngineMessage {
                role: "user".to_string(),
                content: "ping".to_string(),
            }],
            tools_json: None,
            parameters_json: None,
            stream: false,
            timeout_ms: None,
        };

        let encoded = serde_json::to_string(&request).expect("request should serialize");

        assert!(!encoded.contains('\n'));
    }

    #[test]
    fn python_fake_engine_returns_started_delta_finished() {
        let Some(events) = run_fake_worker("ping") else {
            return;
        };

        assert_eq!(events[0].kind, ProviderEngineEventKind::Started);
        assert!(events.iter().any(|event| {
            event.kind == ProviderEngineEventKind::MessageDelta
                && event.content.as_deref() == Some("pong")
        }));
        assert!(
            events
                .iter()
                .any(|event| event.kind == ProviderEngineEventKind::Finished)
        );
    }

    #[test]
    fn python_fake_engine_error_is_structured() {
        let Some(events) = run_fake_worker("fail") else {
            return;
        };

        let error = events
            .iter()
            .find(|event| event.kind == ProviderEngineEventKind::Error)
            .expect("fake failure should emit structured error");
        assert_eq!(error.user_message_zh.as_deref(), Some("Provider 请求失败"));
        assert_eq!(error.machine_code.as_deref(), Some("fake_error"));
    }

    #[test]
    fn python_fake_engine_stdout_contains_only_jsonl_events() {
        let Some(events) = run_fake_worker("ping") else {
            return;
        };

        assert!(
            events
                .iter()
                .all(|event| event.protocol_version == "provider.engine.v1")
        );
    }

    #[test]
    fn openai_engine_reports_missing_sdk_when_package_absent() {
        let Some((events, _, _)) = run_sdk_worker_with_env(
            "openai",
            "openai_responses",
            &[("AICORE_PROVIDER_FORCE_MISSING_OPENAI", "1")],
        ) else {
            return;
        };

        let error = events
            .iter()
            .find(|event| event.kind == ProviderEngineEventKind::Error)
            .expect("forced missing openai SDK should emit error");
        assert_eq!(error.machine_code.as_deref(), Some("openai_sdk_missing"));
    }

    #[test]
    fn anthropic_engine_reports_missing_sdk_when_package_absent() {
        let Some((events, _, _)) = run_sdk_worker_with_env(
            "anthropic",
            "anthropic_messages",
            &[("AICORE_PROVIDER_FORCE_MISSING_ANTHROPIC", "1")],
        ) else {
            return;
        };

        let error = events
            .iter()
            .find(|event| event.kind == ProviderEngineEventKind::Error)
            .expect("forced missing anthropic SDK should emit error");
        assert_eq!(error.machine_code.as_deref(), Some("anthropic_sdk_missing"));
    }

    #[test]
    fn openai_engine_request_does_not_log_secret() {
        let Some((_, stdout, stderr)) = run_sdk_worker_with_env(
            "openai",
            "openai_responses",
            &[("AICORE_PROVIDER_FORCE_MISSING_OPENAI", "1")],
        ) else {
            return;
        };

        assert!(!stdout.contains("sk-live-secret-value"));
        assert!(!stderr.contains("sk-live-secret-value"));
    }

    #[test]
    fn anthropic_engine_request_does_not_log_secret() {
        let Some((_, stdout, stderr)) = run_sdk_worker_with_env(
            "anthropic",
            "anthropic_messages",
            &[("AICORE_PROVIDER_FORCE_MISSING_ANTHROPIC", "1")],
        ) else {
            return;
        };

        assert!(!stdout.contains("sk-live-secret-value"));
        assert!(!stderr.contains("sk-live-secret-value"));
    }

    #[test]
    fn provider_resolver_classifies_openrouter_as_real_provider_boundary() {
        let resolved =
            ProviderResolver::resolve_primary(&auth_pool(), &runtime_config_openrouter())
                .expect("resolver should classify openrouter");

        assert_eq!(resolved.provider, "openrouter");
        assert_eq!(resolved.kind, ProviderKind::OpenRouter);
        assert_eq!(
            resolved.availability,
            ProviderAvailability::AdapterUnavailable
        );
    }

    #[test]
    fn provider_resolver_classifies_openai_as_real_provider_boundary() {
        let resolved = ProviderResolver::resolve_primary(&auth_pool(), &runtime_config_openai())
            .expect("resolver should classify openai");

        assert_eq!(resolved.provider, "openai");
        assert_eq!(resolved.kind, ProviderKind::OpenAI);
        assert_eq!(
            resolved.availability,
            ProviderAvailability::AdapterUnavailable
        );
    }

    #[test]
    fn provider_resolver_accepts_chat_capability() {
        let resolved = ProviderResolver::resolve_primary(&auth_pool(), &runtime_config())
            .expect("chat capability should pass");
        assert_eq!(resolved.auth_ref.as_str(), "auth.dummy.main");
    }

    #[test]
    fn provider_resolver_rejects_search_only_auth_for_chat_model() {
        let auth_pool = GlobalAuthPool::new(vec![AuthEntry {
            auth_ref: AuthRef::new("auth.search.only"),
            provider: "dummy".to_string(),
            kind: AuthKind::ApiKey,
            secret_ref: SecretRef::new("secret://auth.search.only"),
            capabilities: vec![AuthCapability::Search],
            enabled: true,
        }]);
        let runtime = InstanceRuntimeConfig {
            instance_id: "global-main".to_string(),
            primary: ModelBinding {
                auth_ref: AuthRef::new("auth.search.only"),
                model: "dummy/default-chat".to_string(),
            },
            fallback: None,
        };

        let error = ProviderResolver::resolve_primary(&auth_pool, &runtime)
            .expect_err("search-only auth should fail for chat");
        assert!(matches!(error, crate::ProviderError::Resolve(_)));
    }

    #[test]
    fn provider_resolver_rejects_embedding_only_auth_for_chat_model() {
        let auth_pool = GlobalAuthPool::new(vec![AuthEntry {
            auth_ref: AuthRef::new("auth.embedding.only"),
            provider: "dummy".to_string(),
            kind: AuthKind::ApiKey,
            secret_ref: SecretRef::new("secret://auth.embedding.only"),
            capabilities: vec![AuthCapability::Embedding],
            enabled: true,
        }]);
        let runtime = InstanceRuntimeConfig {
            instance_id: "global-main".to_string(),
            primary: ModelBinding {
                auth_ref: AuthRef::new("auth.embedding.only"),
                model: "dummy/default-chat".to_string(),
            },
            fallback: None,
        };

        let error = ProviderResolver::resolve_primary(&auth_pool, &runtime)
            .expect_err("embedding-only auth should fail for chat");
        assert!(matches!(error, crate::ProviderError::Resolve(_)));
    }

    #[test]
    fn provider_resolver_rejects_non_chat_auth_for_chat_model() {
        let auth_pool = GlobalAuthPool::new(vec![AuthEntry {
            auth_ref: AuthRef::new("auth.vision.only"),
            provider: "dummy".to_string(),
            kind: AuthKind::ApiKey,
            secret_ref: SecretRef::new("secret://auth.vision.only"),
            capabilities: vec![AuthCapability::Vision],
            enabled: true,
        }]);
        let runtime = InstanceRuntimeConfig {
            instance_id: "global-main".to_string(),
            primary: ModelBinding {
                auth_ref: AuthRef::new("auth.vision.only"),
                model: "dummy/default-chat".to_string(),
            },
            fallback: None,
        };

        let error = ProviderResolver::resolve_primary(&auth_pool, &runtime)
            .expect_err("non-chat auth should fail for chat");
        assert!(matches!(error, crate::ProviderError::Resolve(_)));
    }

    #[test]
    fn provider_resolver_rejects_missing_auth_ref() {
        let runtime = InstanceRuntimeConfig {
            instance_id: "global-main".to_string(),
            primary: ModelBinding {
                auth_ref: AuthRef::new("auth.missing"),
                model: "openai/gpt-5".to_string(),
            },
            fallback: None,
        };

        assert!(ProviderResolver::resolve_primary(&auth_pool(), &runtime).is_err());
    }

    #[test]
    fn provider_resolver_rejects_unknown_provider_or_marks_unsupported() {
        let auth_pool = GlobalAuthPool::new(vec![AuthEntry {
            auth_ref: AuthRef::new("auth.unknown.main"),
            provider: "mystery".to_string(),
            kind: AuthKind::ApiKey,
            secret_ref: SecretRef::new("secret://auth.unknown.main"),
            capabilities: vec![AuthCapability::Chat],
            enabled: true,
        }]);
        let runtime = InstanceRuntimeConfig {
            instance_id: "global-main".to_string(),
            primary: ModelBinding {
                auth_ref: AuthRef::new("auth.unknown.main"),
                model: "mystery/model".to_string(),
            },
            fallback: None,
        };

        let error = ProviderResolver::resolve_primary(&auth_pool, &runtime)
            .expect_err("unknown provider should not be silently supported");
        assert!(matches!(error, crate::ProviderError::Resolve(_)));
    }

    #[test]
    fn provider_resolver_rejects_disabled_auth_ref() {
        assert!(
            ProviderResolver::resolve_primary(&auth_pool_with_disabled_entry(), &runtime_config())
                .is_err()
        );
    }

    #[test]
    fn provider_invoker_routes_dummy_to_dummy_provider() {
        let resolved = ProviderResolver::resolve_primary(&auth_pool(), &runtime_config())
            .expect("resolver should resolve primary model");
        let request = ModelRequest {
            instance_id: "global-main".to_string(),
            conversation_id: "main".to_string(),
            prompt: "hello".to_string(),
            resolved_model: resolved,
        };

        let response = ProviderInvoker::invoke(&request).expect("dummy provider should run");

        assert_eq!(response.role, "assistant");
        assert!(response.content.contains("dummy"));
        assert!(response.content.contains("dummy/default-chat"));
    }

    #[test]
    fn provider_invoker_does_not_silently_dummy_real_provider() {
        let resolved =
            ProviderResolver::resolve_primary(&auth_pool(), &runtime_config_openrouter())
                .expect("resolver should classify openrouter");
        let request = ModelRequest {
            instance_id: "global-main".to_string(),
            conversation_id: "main".to_string(),
            prompt: "hello".to_string(),
            resolved_model: resolved,
        };

        let error = ProviderInvoker::invoke(&request)
            .expect_err("real provider should not be silently routed to dummy");
        assert!(matches!(error, crate::ProviderError::Invoke(_)));
    }

    #[test]
    fn provider_invoker_returns_unavailable_for_real_provider_without_adapter() {
        let resolved = ProviderResolver::resolve_primary(&auth_pool(), &runtime_config_openai())
            .expect("resolver should classify openai");
        let request = ModelRequest {
            instance_id: "global-main".to_string(),
            conversation_id: "main".to_string(),
            prompt: "hello".to_string(),
            resolved_model: resolved,
        };

        let error = ProviderInvoker::invoke(&request)
            .expect_err("real provider should be gated as unavailable");
        match error {
            crate::ProviderError::Invoke(message) => {
                assert!(message.contains("provider adapter unavailable"));
                assert!(!message.contains("secret://"));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn provider_boundary_does_not_expose_secret_in_error() {
        let resolved =
            ProviderResolver::resolve_primary(&auth_pool(), &runtime_config_openrouter())
                .expect("resolver should classify openrouter");
        let request = ModelRequest {
            instance_id: "global-main".to_string(),
            conversation_id: "main".to_string(),
            prompt: "secret boundary".to_string(),
            resolved_model: resolved,
        };

        let error = ProviderInvoker::invoke(&request)
            .expect_err("real provider should be gated as unavailable");
        let rendered = format!("{error:?}");
        assert!(!rendered.contains("secret://"));
        assert!(!rendered.contains("auth.openrouter.main"));
    }

    #[test]
    fn model_request_keeps_prompt_and_resolved_model_boundary() {
        let resolved = ProviderResolver::resolve_primary(&auth_pool(), &runtime_config())
            .expect("resolver should resolve dummy");
        let request = ModelRequest {
            instance_id: "global-main".to_string(),
            conversation_id: "main".to_string(),
            prompt: "boundary prompt".to_string(),
            resolved_model: resolved.clone(),
        };

        assert_eq!(request.prompt, "boundary prompt");
        assert_eq!(request.resolved_model, resolved);
    }

    #[test]
    fn prompt_builder_includes_background_memory() {
        let prompt = PromptBuilder::build(PromptBuildInput {
            instance_id: "global-main".to_string(),
            system_rules: "You are the AICore instance runtime.".to_string(),
            relevant_memory: vec![],
            user_request: "请总结当前状态".to_string(),
        })
        .prompt;

        assert!(prompt.contains("MEMORY SNAPSHOT:"));
        assert!(prompt.contains("background context only"));
    }

    #[test]
    fn prompt_builder_marks_memory_as_not_current_instruction() {
        let prompt = PromptBuilder::build(PromptBuildInput {
            instance_id: "global-main".to_string(),
            system_rules: "You are the AICore instance runtime.".to_string(),
            relevant_memory: vec![],
            user_request: "继续实现".to_string(),
        })
        .prompt;

        assert!(prompt.contains("not the current user instruction"));
        assert!(prompt.contains("not as the latest request"));
    }

    #[test]
    fn prompt_builder_puts_current_user_request_last() {
        let prompt = PromptBuilder::build(PromptBuildInput {
            instance_id: "global-main".to_string(),
            system_rules: "You are the AICore instance runtime.".to_string(),
            relevant_memory: vec![],
            user_request: "最后的用户请求".to_string(),
        })
        .prompt;

        assert!(prompt.ends_with("最后的用户请求"));
        let current_request_pos = prompt.find("CURRENT USER REQUEST:").unwrap();
        let memory_pos = prompt.find("RELEVANT MEMORY:").unwrap();
        assert!(current_request_pos > memory_pos);
    }

    #[test]
    fn prompt_builder_respects_memory_pack_limit() {
        let paths = temp_paths("prompt-pack-limit");
        let mut kernel = MemoryKernel::open(paths).expect("memory kernel should open");

        kernel
            .remember_user_explicit(RememberInput {
                memory_type: MemoryType::Core,
                permanence: MemoryPermanence::Permanent,
                scope: global_scope(),
                content: "重要长期记忆".to_string(),
                localized_summary: "重要长期记忆".to_string(),
                state_key: None,
                current_state: None,
            })
            .expect("remember should succeed");
        kernel
            .remember_user_explicit(RememberInput {
                memory_type: MemoryType::Working,
                permanence: MemoryPermanence::Standard,
                scope: global_scope(),
                content: "第二条较长记忆内容".to_string(),
                localized_summary: "第二条较长记忆内容".to_string(),
                state_key: None,
                current_state: None,
            })
            .expect("remember should succeed");

        let pack = kernel.build_memory_context_pack(
            SearchQuery {
                text: String::new(),
                scope: Some(global_scope()),
                memory_type: None,
                source: None,
                permanence: None,
                limit: None,
            },
            8,
        );

        assert_eq!(pack.len(), 1);
        assert_eq!(pack[0].localized_summary, "重要长期记忆");
    }

    #[test]
    fn prompt_builder_excludes_archived_memory() {
        let paths = temp_paths("prompt-pack-archived");
        let mut kernel = MemoryKernel::open(paths).expect("memory kernel should open");

        let active_id = kernel
            .remember_user_explicit(RememberInput {
                memory_type: MemoryType::Core,
                permanence: MemoryPermanence::Standard,
                scope: global_scope(),
                content: "active memory".to_string(),
                localized_summary: "active memory".to_string(),
                state_key: None,
                current_state: None,
            })
            .expect("remember should succeed");
        let archived_id = kernel
            .remember_user_explicit(RememberInput {
                memory_type: MemoryType::Core,
                permanence: MemoryPermanence::Permanent,
                scope: global_scope(),
                content: "archived memory".to_string(),
                localized_summary: "archived memory".to_string(),
                state_key: None,
                current_state: None,
            })
            .expect("remember should succeed");
        kernel
            .archive(&archived_id)
            .expect("archive should succeed");

        let pack = kernel.build_memory_context_pack(
            SearchQuery {
                text: "memory".to_string(),
                scope: Some(global_scope()),
                memory_type: None,
                source: None,
                permanence: None,
                limit: None,
            },
            128,
        );

        assert_eq!(pack.len(), 1);
        assert_eq!(pack[0].memory_id, active_id);
    }

    #[test]
    fn prompt_builder_uses_search_result_order() {
        let paths = temp_paths("prompt-pack-order");
        let mut kernel = MemoryKernel::open(paths).expect("memory kernel should open");

        let decision_id = kernel
            .remember_user_explicit(RememberInput {
                memory_type: MemoryType::Decision,
                permanence: MemoryPermanence::Permanent,
                scope: global_scope(),
                content: "shared request".to_string(),
                localized_summary: "shared request".to_string(),
                state_key: None,
                current_state: None,
            })
            .expect("remember should succeed");
        let core_id = kernel
            .remember_user_explicit(RememberInput {
                memory_type: MemoryType::Core,
                permanence: MemoryPermanence::Permanent,
                scope: global_scope(),
                content: "shared request".to_string(),
                localized_summary: "shared request".to_string(),
                state_key: None,
                current_state: None,
            })
            .expect("remember should succeed");

        let pack = kernel.build_memory_context_pack(
            SearchQuery {
                text: "shared".to_string(),
                scope: Some(global_scope()),
                memory_type: None,
                source: None,
                permanence: None,
                limit: None,
            },
            128,
        );

        assert_eq!(pack.len(), 2);
        assert_eq!(pack[0].memory_id, core_id);
        assert_eq!(pack[1].memory_id, decision_id);
    }
}
