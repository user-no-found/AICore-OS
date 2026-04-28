use super::*;

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
    let resolved = ProviderResolver::resolve_primary(&auth_pool(), &runtime_config_openrouter())
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
