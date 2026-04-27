#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct ProviderEngineRequest {
    pub protocol_version: String,
    pub invocation_id: String,
    pub provider_id: String,
    pub adapter_id: String,
    pub engine_id: String,
    pub api_mode: String,
    pub model: String,
    pub base_url: Option<String>,
    pub credential_lease_ref: Option<String>,
    pub messages: Vec<ProviderEngineMessage>,
    pub tools_json: Option<String>,
    pub parameters_json: Option<String>,
    pub stream: bool,
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct ProviderEngineMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum ProviderEngineEventKind {
    Started,
    MessageDelta,
    ReasoningDelta,
    ToolCallDelta,
    Usage,
    Finished,
    Error,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct ProviderEngineEvent {
    pub protocol_version: String,
    pub invocation_id: String,
    pub kind: ProviderEngineEventKind,
    pub content: Option<String>,
    pub payload_json: Option<String>,
    pub user_message_zh: Option<String>,
    pub machine_code: Option<String>,
}
