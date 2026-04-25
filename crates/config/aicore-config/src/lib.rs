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
    pub auth_ref: String,
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
    use aicore_auth::{AuthEntry, AuthPool};

    use super::{GlobalServiceProfiles, InstanceRuntimeConfig, ModelBinding, ServiceProfile};

    #[test]
    fn separates_auth_pool_from_runtime_config() {
        let auth_pool = AuthPool {
            entries: vec![AuthEntry {
                auth_ref: "auth.openrouter.main".to_string(),
                provider: "openrouter".to_string(),
                kind: "api_key".to_string(),
            }],
        };

        let runtime = InstanceRuntimeConfig {
            instance_id: "global-main".to_string(),
            primary: ModelBinding {
                auth_ref: "auth.openrouter.main".to_string(),
                model: "openai/gpt-5".to_string(),
            },
            fallback: None,
        };

        assert_eq!(auth_pool.entries.len(), 1);
        assert_eq!(runtime.primary.model, "openai/gpt-5");
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
                auth_ref: "auth.openrouter.main".to_string(),
                model: "anthropic/claude-sonnet".to_string(),
            },
            fallback: Some(ModelBinding {
                auth_ref: "auth.openai.backup".to_string(),
                model: "gpt-4.1".to_string(),
            }),
        };

        assert_eq!(services.profiles[0].role, "memory.dreamer");
        assert_eq!(runtime.instance_id, "inst_project_a");
    }
}
