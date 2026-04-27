use crate::{DummyProvider, ModelRequest, ModelResponse, ProviderError, ProviderKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderInvoker;

impl ProviderInvoker {
    pub fn invoke(request: &ModelRequest) -> Result<ModelResponse, ProviderError> {
        match &request.resolved_model.kind {
            ProviderKind::Dummy => Ok(DummyProvider::generate(request)),
            ProviderKind::OpenRouter => Err(ProviderError::Invoke(
                "provider adapter unavailable: openrouter".to_string(),
            )),
            ProviderKind::OpenAI => Err(ProviderError::Invoke(
                "provider adapter unavailable: openai".to_string(),
            )),
        }
    }
}
