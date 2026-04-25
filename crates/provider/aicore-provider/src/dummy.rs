use crate::{ModelRequest, ModelResponse};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DummyProvider;

impl DummyProvider {
    pub fn generate(request: &ModelRequest) -> ModelResponse {
        ModelResponse {
            role: "assistant".to_string(),
            content: format!(
                "dummy provider response for {} via {}",
                request.resolved_model.model, request.resolved_model.provider
            ),
        }
    }
}
