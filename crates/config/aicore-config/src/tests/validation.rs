use super::*;

#[test]
fn validate_runtime_config_accepts_known_primary_auth_ref() {
    let runtime = InstanceRuntimeConfig {
        instance_id: "global-main".to_string(),
        primary: ModelBinding {
            auth_ref: AuthRef::new("auth.openrouter.main"),
            model: "openai/gpt-5".to_string(),
        },
        fallback: None,
    };

    assert!(ConfigStore::validate_runtime_config(&runtime, &auth_pool()).is_ok());
}

#[test]
fn validate_runtime_config_rejects_missing_primary_auth_ref() {
    let runtime = InstanceRuntimeConfig {
        instance_id: "global-main".to_string(),
        primary: ModelBinding {
            auth_ref: AuthRef::new("auth.missing"),
            model: "openai/gpt-5".to_string(),
        },
        fallback: None,
    };

    assert!(ConfigStore::validate_runtime_config(&runtime, &auth_pool()).is_err());
}

#[test]
fn validate_runtime_config_rejects_missing_fallback_auth_ref() {
    let runtime = InstanceRuntimeConfig {
        instance_id: "global-main".to_string(),
        primary: ModelBinding {
            auth_ref: AuthRef::new("auth.openrouter.main"),
            model: "openai/gpt-5".to_string(),
        },
        fallback: Some(ModelBinding {
            auth_ref: AuthRef::new("auth.missing"),
            model: "gpt-4.1".to_string(),
        }),
    };

    assert!(ConfigStore::validate_runtime_config(&runtime, &auth_pool()).is_err());
}

#[test]
fn validate_explicit_service_profile_requires_auth_ref_and_model() {
    let services = GlobalServiceProfiles {
        profiles: vec![ServiceProfile {
            role: ServiceRole::Search,
            mode: ServiceProfileMode::Explicit,
            auth_ref: None,
            model: None,
        }],
    };

    assert!(ConfigStore::validate_service_profiles(&services, &auth_pool()).is_err());
}

#[test]
fn validate_explicit_service_profile_rejects_unknown_auth_ref() {
    let services = GlobalServiceProfiles {
        profiles: vec![ServiceProfile {
            role: ServiceRole::Search,
            mode: ServiceProfileMode::Explicit,
            auth_ref: Some(AuthRef::new("auth.missing")),
            model: Some("perplexity/sonar".to_string()),
        }],
    };

    assert!(ConfigStore::validate_service_profiles(&services, &auth_pool()).is_err());
}

#[test]
fn validate_inherit_instance_service_profile_without_auth_or_model() {
    let services = GlobalServiceProfiles {
        profiles: vec![ServiceProfile {
            role: ServiceRole::MemoryDreamer,
            mode: ServiceProfileMode::InheritInstance,
            auth_ref: None,
            model: None,
        }],
    };

    assert!(ConfigStore::validate_service_profiles(&services, &auth_pool()).is_ok());
}

#[test]
fn validate_disabled_service_profile_without_auth_or_model() {
    let services = GlobalServiceProfiles {
        profiles: vec![ServiceProfile {
            role: ServiceRole::EvolutionReviewer,
            mode: ServiceProfileMode::Disabled,
            auth_ref: None,
            model: None,
        }],
    };

    assert!(ConfigStore::validate_service_profiles(&services, &auth_pool()).is_ok());
}
