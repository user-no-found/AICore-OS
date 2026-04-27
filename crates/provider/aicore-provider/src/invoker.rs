use crate::{
    DummyProvider, ModelRequest, ModelResponse, ProviderEngineManager, ProviderEngineMessage,
    ProviderEngineRequest, ProviderError, ProviderKind, normalizer::events_to_model_response,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderInvoker;

impl ProviderInvoker {
    pub fn invoke(request: &ModelRequest) -> Result<ModelResponse, ProviderError> {
        match &request.resolved_model.kind {
            ProviderKind::Dummy => Ok(DummyProvider::generate(request)),
            _ => Self::invoke_with_manager(request, &ProviderEngineManager::default_for_crate()),
        }
    }

    pub fn invoke_with_manager(
        request: &ModelRequest,
        manager: &ProviderEngineManager,
    ) -> Result<ModelResponse, ProviderError> {
        let engine_request = Self::build_engine_request(request);
        let engine_id = engine_request.engine_id.clone();
        let events = manager.invoke_python_engine(&engine_id, engine_request)?;
        events_to_model_response(&events)
    }

    pub fn build_engine_request(request: &ModelRequest) -> ProviderEngineRequest {
        let runtime = &request.resolved_model.runtime;

        ProviderEngineRequest {
            protocol_version: "provider.engine.v1".to_string(),
            invocation_id: format!("{}:{}", request.instance_id, request.conversation_id),
            provider_id: runtime.provider_id.clone(),
            adapter_id: runtime.adapter_id.clone(),
            engine_id: runtime.engine_id.clone(),
            api_mode: runtime.api_mode.as_str().to_string(),
            model: runtime.model.clone(),
            base_url: runtime.base_url.clone(),
            credential_lease_ref: runtime
                .auth_ref
                .as_ref()
                .map(|auth_ref| format!("lease:{}", auth_ref.as_str())),
            messages: vec![ProviderEngineMessage {
                role: "user".to_string(),
                content: request.prompt.clone(),
            }],
            tools_json: None,
            parameters_json: None,
            stream: false,
            timeout_ms: None,
        }
    }
}
