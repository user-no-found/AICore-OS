use aicore_auth::AuthRef;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderKind {
    Dummy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderDescriptor {
    pub kind: ProviderKind,
    pub provider: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedModel {
    pub auth_ref: AuthRef,
    pub model: String,
    pub provider: String,
    pub kind: ProviderKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelRequest {
    pub instance_id: String,
    pub conversation_id: String,
    pub prompt: String,
    pub resolved_model: ResolvedModel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelResponse {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderError {
    Resolve(String),
}
