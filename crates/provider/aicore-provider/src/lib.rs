mod adapter;
mod dummy;
mod engine_ipc;
mod engine_manager;
mod invoker;
mod normalizer;
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
pub use engine_manager::ProviderEngineManager;
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
mod tests;
