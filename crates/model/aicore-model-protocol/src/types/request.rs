use super::{
    ModelProfileRef, ModelProtocolVersion, ModelProviderRef, ModelRequestId, ModelRunId,
    PromptAssembly,
};
use aicore_foundation::{InstanceId, SessionId, Timestamp};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelRequestEnvelope {
    pub request_id: ModelRequestId,
    pub run_id: ModelRunId,
    pub instance_id: InstanceId,
    pub session_id: Option<SessionId>,
    pub turn_id: Option<String>,
    pub correlation_id: Option<String>,
    pub causation_id: Option<String>,
    pub provider: ModelProviderRef,
    pub profile: ModelProfileRef,
    pub protocol_version: ModelProtocolVersion,
    pub assembly: PromptAssembly,
    pub options: ModelRequestOptions,
    pub created_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelRequestOptions {
    pub stream: bool,
    pub max_output_units: Option<u64>,
    pub temperature: Option<f32>,
}
