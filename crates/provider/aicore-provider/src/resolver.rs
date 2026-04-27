use aicore_auth::GlobalAuthPool;
use aicore_config::InstanceRuntimeConfig;

use crate::{
    ProviderError, ProviderRegistry, ProviderRuntimeResolveInput, ProviderRuntimeResolver,
    ResolvedModel,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderResolver;

impl ProviderResolver {
    pub fn resolve_primary(
        auth_pool: &GlobalAuthPool,
        runtime: &InstanceRuntimeConfig,
    ) -> Result<ResolvedModel, ProviderError> {
        let registry = ProviderRegistry::builtin();
        ProviderRuntimeResolver::resolve(ProviderRuntimeResolveInput {
            auth_pool,
            runtime,
            registry: &registry,
        })
        .map(|output| output.resolved_model)
    }
}
