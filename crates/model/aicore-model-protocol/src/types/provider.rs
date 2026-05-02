use super::{ModelId, ProviderId};
use aicore_foundation::{InstanceId, SessionId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelProviderRef {
    pub provider_id: ProviderId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelProfileRef {
    pub model_id: ModelId,
    pub profile_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelInvocationContext {
    pub instance_id: InstanceId,
    pub session_id: Option<SessionId>,
    pub turn_id: Option<String>,
    pub correlation_id: Option<String>,
    pub causation_id: Option<String>,
}
