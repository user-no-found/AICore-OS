use crate::{
    ModelRequestEnvelope, ModelResponseEvent, PromptAssembly, PromptAssemblyError,
    PromptAssemblyRequest,
};

pub trait ModelPromptAssembler {
    fn assemble(
        &self,
        request: PromptAssemblyRequest,
    ) -> Result<PromptAssembly, PromptAssemblyError>;
}

pub trait ModelProvider {
    fn invoke(&self, request: &ModelRequestEnvelope) -> Result<Vec<ModelResponseEvent>, String>;
}

pub trait ModelRunRecorder {
    fn record_event(&mut self, event: ModelResponseEvent) -> Result<(), String>;
}
