use crate::{
    DummyProvider, ModelRequest, ModelResponse, ProviderAvailability, ProviderError, ProviderKind,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderInvoker;

impl ProviderInvoker {
    pub fn invoke(request: &ModelRequest) -> Result<ModelResponse, ProviderError> {
        match &request.resolved_model.availability {
            ProviderAvailability::Available => {}
            ProviderAvailability::AdapterUnavailable => {
                return Err(ProviderError::Invoke(format!(
                    "provider adapter unavailable: {}",
                    request.resolved_model.provider
                )));
            }
        }

        match &request.resolved_model.kind {
            ProviderKind::Dummy => Ok(DummyProvider::generate(request)),
            ProviderKind::OpenRouter | ProviderKind::OpenAI => Err(ProviderError::Invoke(format!(
                "provider adapter unavailable: {}",
                request.resolved_model.provider
            ))),
        }
    }
}
