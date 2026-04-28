use super::*;

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
    let resolved = ProviderResolver::resolve_primary(&auth_pool(), &runtime_config_openrouter())
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
            assert!(
                message.contains("Provider")
                    || message.contains("provider")
                    || message.contains("engine")
            );
            assert!(!message.contains("secret://"));
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn provider_invoker_keeps_dummy_path_available() {
    let resolved = ProviderResolver::resolve_primary(&auth_pool(), &runtime_config())
        .expect("resolver should resolve dummy");
    let request = ModelRequest {
        instance_id: "global-main".to_string(),
        conversation_id: "main".to_string(),
        prompt: "hello".to_string(),
        resolved_model: resolved,
    };

    let response = ProviderInvoker::invoke(&request).expect("dummy provider should run");

    assert_eq!(response.role, "assistant");
    assert!(response.content.contains("dummy"));
}

#[test]
fn provider_invoker_routes_openrouter_to_python_openai_engine() {
    let request = ModelRequest {
        instance_id: "global-main".to_string(),
        conversation_id: "main".to_string(),
        prompt: "hello".to_string(),
        resolved_model: resolve_model("openrouter", "anthropic/claude-sonnet"),
    };
    let engine_request = ProviderInvoker::build_engine_request(&request);

    assert_eq!(engine_request.provider_id, "openrouter");
    assert_eq!(engine_request.api_mode, "openai_chat_completions");
    assert_eq!(engine_request.engine_id, "python.openai");
}

#[test]
fn provider_invoker_routes_openai_to_python_openai_engine() {
    let request = ModelRequest {
        instance_id: "global-main".to_string(),
        conversation_id: "main".to_string(),
        prompt: "hello".to_string(),
        resolved_model: resolve_model("openai", "gpt-4.1"),
    };
    let engine_request = ProviderInvoker::build_engine_request(&request);

    assert_eq!(engine_request.api_mode, "openai_responses");
    assert_eq!(engine_request.engine_id, "python.openai");
}

#[test]
fn provider_invoker_routes_anthropic_to_python_anthropic_engine() {
    let request = ModelRequest {
        instance_id: "global-main".to_string(),
        conversation_id: "main".to_string(),
        prompt: "hello".to_string(),
        resolved_model: resolve_model("anthropic", "claude-sonnet-4-5"),
    };
    let engine_request = ProviderInvoker::build_engine_request(&request);

    assert_eq!(engine_request.api_mode, "anthropic_messages");
    assert_eq!(engine_request.engine_id, "python.anthropic");
}

#[test]
fn provider_invoker_routes_kimi_to_python_openai_engine() {
    let request = ModelRequest {
        instance_id: "global-main".to_string(),
        conversation_id: "main".to_string(),
        prompt: "hello".to_string(),
        resolved_model: resolve_model("kimi", "moonshot-v1-8k"),
    };
    let engine_request = ProviderInvoker::build_engine_request(&request);

    assert_eq!(engine_request.api_mode, "openai_chat_completions");
    assert_eq!(engine_request.engine_id, "python.openai");
}

#[test]
fn provider_invoker_routes_kimi_coding_to_python_anthropic_engine() {
    let request = ModelRequest {
        instance_id: "global-main".to_string(),
        conversation_id: "main".to_string(),
        prompt: "hello".to_string(),
        resolved_model: resolve_model("kimi-coding", "kimi-k2-coder"),
    };
    let engine_request = ProviderInvoker::build_engine_request(&request);

    assert_eq!(engine_request.api_mode, "anthropic_messages");
    assert_eq!(engine_request.engine_id, "python.anthropic");
}

#[test]
fn provider_invoker_rejects_xiaomi_without_profile() {
    let auth_pool = auth_pool_for_provider("xiaomi");
    let runtime = runtime_for_model("mimo");
    let registry = ProviderRegistry::builtin();

    assert!(
        ProviderRuntimeResolver::resolve(ProviderRuntimeResolveInput {
            auth_pool: &auth_pool,
            runtime: &runtime,
            registry: &registry,
        })
        .is_err()
    );
}

#[test]
fn provider_invoker_engine_unavailable_returns_structured_failure() {
    let request = ModelRequest {
        instance_id: "global-main".to_string(),
        conversation_id: "main".to_string(),
        prompt: "hello".to_string(),
        resolved_model: resolve_model("openai-codex-login", "gpt-5-codex"),
    };
    let manager = ProviderEngineManager::default_for_crate();
    let error = ProviderInvoker::invoke_with_manager(&request, &manager)
        .expect_err("codex bridge should be unavailable in M1");

    assert!(matches!(error, crate::ProviderError::Invoke(_)));
}

#[test]
fn provider_invoker_does_not_expose_credential_lease_ref() {
    let request = ModelRequest {
        instance_id: "global-main".to_string(),
        conversation_id: "main".to_string(),
        prompt: "hello".to_string(),
        resolved_model: resolve_model("openai", "gpt-4.1"),
    };
    let error = ProviderInvoker::invoke(&request)
        .expect_err("real provider should fail without credential lease resolver");
    let rendered = format!("{error:?}");

    assert!(!rendered.contains("credential_lease_ref"));
    assert!(!rendered.contains("lease:auth.test.main"));
    assert!(!rendered.contains("secret://"));
}

#[test]
fn provider_boundary_does_not_expose_secret_in_error() {
    let resolved = ProviderResolver::resolve_primary(&auth_pool(), &runtime_config_openrouter())
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
