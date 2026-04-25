mod dummy;
mod resolver;
mod types;

pub use dummy::DummyProvider;
pub use resolver::ProviderResolver;
pub use types::{
    ModelRequest, ModelResponse, ProviderDescriptor, ProviderError, ProviderKind, ResolvedModel,
};

#[cfg(test)]
mod tests {
    use aicore_auth::{AuthCapability, AuthEntry, AuthKind, AuthRef, GlobalAuthPool, SecretRef};
    use aicore_config::{InstanceRuntimeConfig, ModelBinding};

    use crate::{DummyProvider, ModelRequest, ProviderKind, ProviderResolver};

    fn auth_pool() -> GlobalAuthPool {
        GlobalAuthPool::new(vec![AuthEntry {
            auth_ref: AuthRef::new("auth.openrouter.main"),
            provider: "openrouter".to_string(),
            kind: AuthKind::ApiKey,
            secret_ref: SecretRef::new("secret://auth.openrouter.main"),
            capabilities: vec![AuthCapability::Chat],
            enabled: true,
        }])
    }

    fn runtime_config() -> InstanceRuntimeConfig {
        InstanceRuntimeConfig {
            instance_id: "global-main".to_string(),
            primary: ModelBinding {
                auth_ref: AuthRef::new("auth.openrouter.main"),
                model: "openai/gpt-5".to_string(),
            },
            fallback: None,
        }
    }

    #[test]
    fn provider_resolver_resolves_primary_model() {
        let resolved = ProviderResolver::resolve_primary(&auth_pool(), &runtime_config())
            .expect("resolver should resolve primary model");

        assert_eq!(resolved.auth_ref.as_str(), "auth.openrouter.main");
        assert_eq!(resolved.model, "openai/gpt-5");
        assert_eq!(resolved.provider, "openrouter");
        assert_eq!(resolved.kind, ProviderKind::Dummy);
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
    fn dummy_provider_returns_assistant_response() {
        let resolved = ProviderResolver::resolve_primary(&auth_pool(), &runtime_config())
            .expect("resolver should resolve primary model");
        let request = ModelRequest {
            instance_id: "global-main".to_string(),
            conversation_id: "main".to_string(),
            prompt: "hello".to_string(),
            resolved_model: resolved,
        };

        let response = DummyProvider::generate(&request);

        assert_eq!(response.role, "assistant");
        assert!(response.content.contains("dummy"));
        assert!(response.content.contains("openai/gpt-5"));
    }
}
