use aicore_auth::AuthRef;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceProfile {
    pub role: String,
    pub auth_ref: Option<String>,
    pub model: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobalServiceProfiles {
    pub profiles: Vec<ServiceProfile>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelBinding {
    pub auth_ref: AuthRef,
    pub model: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceRuntimeConfig {
    pub instance_id: String,
    pub primary: ModelBinding,
    pub fallback: Option<ModelBinding>,
}

#[cfg(test)]
mod tests {
    use aicore_auth::{AuthCapability, AuthEntry, AuthKind, AuthRef, GlobalAuthPool, SecretRef};

    use super::{GlobalServiceProfiles, InstanceRuntimeConfig, ModelBinding, ServiceProfile};

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
                role: "memory.dreamer".to_string(),
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

        assert_eq!(services.profiles[0].role, "memory.dreamer");
        assert_eq!(runtime.instance_id, "inst_project_a");
    }
}
