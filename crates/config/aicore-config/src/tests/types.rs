use super::*;

#[test]
fn separates_auth_pool_from_runtime_config() {
    let auth_pool = GlobalAuthPool::new(vec![AuthEntry {
        auth_ref: AuthRef::new("auth.openrouter.main"),
        provider: "openrouter".to_string(),
        kind: AuthKind::ApiKey,
        secret_ref: SecretRef::new("secret://auth.openrouter.main"),
        capabilities: vec![AuthCapability::Chat],
        enabled: true,
    }]);

    let runtime = InstanceRuntimeConfig {
        instance_id: "global-main".to_string(),
        primary: ModelBinding {
            auth_ref: AuthRef::new("auth.openrouter.main"),
            model: "openai/gpt-5".to_string(),
        },
        fallback: None,
    };

    assert_eq!(auth_pool.entries().len(), 1);
    assert_eq!(runtime.primary.model, "openai/gpt-5");
}

#[test]
fn primary_model_binding_uses_auth_ref() {
    let runtime = InstanceRuntimeConfig {
        instance_id: "inst_project_a".to_string(),
        primary: ModelBinding {
            auth_ref: AuthRef::new("auth.openrouter.main"),
            model: "anthropic/claude-sonnet".to_string(),
        },
        fallback: None,
    };

    assert_eq!(
        runtime.primary.auth_ref,
        AuthRef::new("auth.openrouter.main")
    );
}

#[test]
fn fallback_model_binding_is_optional() {
    let runtime = InstanceRuntimeConfig {
        instance_id: "inst_project_a".to_string(),
        primary: ModelBinding {
            auth_ref: AuthRef::new("auth.openrouter.main"),
            model: "anthropic/claude-sonnet".to_string(),
        },
        fallback: Some(ModelBinding {
            auth_ref: AuthRef::new("auth.openai.backup"),
            model: "gpt-4.1".to_string(),
        }),
    };

    assert_eq!(runtime.fallback.as_ref().unwrap().model, "gpt-4.1");
}

#[test]
fn runtime_config_can_have_different_primary_and_fallback_auth_refs() {
    let runtime = InstanceRuntimeConfig {
        instance_id: "inst_project_a".to_string(),
        primary: ModelBinding {
            auth_ref: AuthRef::new("auth.openrouter.main"),
            model: "anthropic/claude-sonnet".to_string(),
        },
        fallback: Some(ModelBinding {
            auth_ref: AuthRef::new("auth.openai.backup"),
            model: "gpt-4.1".to_string(),
        }),
    };

    assert_ne!(
        runtime.primary.auth_ref,
        runtime.fallback.as_ref().unwrap().auth_ref
    );
}

#[test]
fn runtime_config_does_not_store_secret_ref_or_secret_value() {
    let runtime = InstanceRuntimeConfig {
        instance_id: "inst_project_a".to_string(),
        primary: ModelBinding {
            auth_ref: AuthRef::new("auth.openrouter.main"),
            model: "anthropic/claude-sonnet".to_string(),
        },
        fallback: None,
    };

    assert_eq!(
        runtime.primary.auth_ref,
        AuthRef::new("auth.openrouter.main")
    );
    assert_ne!(runtime.primary.model, "secret://auth.openrouter.main");
    assert_ne!(runtime.primary.model, "sk-live-secret-value");
}

#[test]
fn separates_service_profiles_from_instance_runtime() {
    let services = GlobalServiceProfiles {
        profiles: vec![ServiceProfile {
            role: ServiceRole::MemoryDreamer,
            mode: ServiceProfileMode::InheritInstance,
            auth_ref: None,
            model: None,
        }],
    };

    let runtime = InstanceRuntimeConfig {
        instance_id: "inst_project_a".to_string(),
        primary: ModelBinding {
            auth_ref: AuthRef::new("auth.openrouter.main"),
            model: "anthropic/claude-sonnet".to_string(),
        },
        fallback: Some(ModelBinding {
            auth_ref: AuthRef::new("auth.openai.backup"),
            model: "gpt-4.1".to_string(),
        }),
    };

    assert_eq!(services.profiles[0].role, ServiceRole::MemoryDreamer);
    assert_eq!(runtime.instance_id, "inst_project_a");
}

#[test]
fn default_service_profile_inherits_instance() {
    let profile = ServiceProfile {
        role: ServiceRole::MemoryDreamer,
        mode: ServiceProfileMode::InheritInstance,
        auth_ref: None,
        model: None,
    };

    assert_eq!(profile.mode, ServiceProfileMode::InheritInstance);
    assert_eq!(profile.auth_ref, None);
    assert_eq!(profile.model, None);
}

#[test]
fn explicit_service_profile_uses_auth_ref_and_model() {
    let profile = ServiceProfile {
        role: ServiceRole::Search,
        mode: ServiceProfileMode::Explicit,
        auth_ref: Some(AuthRef::new("auth.openrouter.search")),
        model: Some("perplexity/sonar".to_string()),
    };

    assert_eq!(profile.mode, ServiceProfileMode::Explicit);
    assert_eq!(
        profile.auth_ref,
        Some(AuthRef::new("auth.openrouter.search"))
    );
    assert_eq!(profile.model.as_deref(), Some("perplexity/sonar"));
}

#[test]
fn disabled_service_profile_has_no_auth_or_model_requirement() {
    let profile = ServiceProfile {
        role: ServiceRole::EvolutionReviewer,
        mode: ServiceProfileMode::Disabled,
        auth_ref: None,
        model: None,
    };

    assert_eq!(profile.mode, ServiceProfileMode::Disabled);
    assert_eq!(profile.auth_ref, None);
    assert_eq!(profile.model, None);
}

#[test]
fn memory_dreamer_can_be_explicit() {
    let profile = ServiceProfile {
        role: ServiceRole::MemoryDreamer,
        mode: ServiceProfileMode::Explicit,
        auth_ref: Some(AuthRef::new("auth.openrouter.memory")),
        model: Some("openai/gpt-5".to_string()),
    };

    assert_eq!(profile.role, ServiceRole::MemoryDreamer);
    assert_eq!(profile.mode, ServiceProfileMode::Explicit);
}

#[test]
fn evolution_reviewer_can_be_disabled() {
    let profile = ServiceProfile {
        role: ServiceRole::EvolutionReviewer,
        mode: ServiceProfileMode::Disabled,
        auth_ref: None,
        model: None,
    };

    assert_eq!(profile.role, ServiceRole::EvolutionReviewer);
    assert_eq!(profile.mode, ServiceProfileMode::Disabled);
}
