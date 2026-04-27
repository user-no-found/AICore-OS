use aicore_auth::AuthRef;
use aicore_memory::MemoryRecord;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderKind {
    Dummy,
    OpenRouter,
    OpenAI,
    Anthropic,
    Kimi,
    KimiCoding,
    DeepSeek,
    Glm,
    MiniMax,
    MiniMaxOpenAI,
    OpenAICodexLogin,
    CustomOpenAICompatible,
    CustomAnthropicCompatible,
    Xiaomi,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderAvailability {
    Available,
    AdapterUnavailable,
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
    pub availability: ProviderAvailability,
    pub runtime: ProviderRuntime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderApiMode {
    Dummy,
    OpenAiChatCompletions,
    OpenAiResponses,
    AnthropicMessages,
    GeminiGenerateContent,
    CodexResponses,
}

impl ProviderApiMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Dummy => "dummy",
            Self::OpenAiChatCompletions => "openai_chat_completions",
            Self::OpenAiResponses => "openai_responses",
            Self::AnthropicMessages => "anthropic_messages",
            Self::GeminiGenerateContent => "gemini_generate_content",
            Self::CodexResponses => "codex_responses",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderAuthMode {
    None,
    ApiKey,
    OAuth,
    Session,
    ExternalProcess,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequestEngineKind {
    Dummy,
    PythonSdk,
    RustHttp,
    ExternalProcess,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderAdapterStatus {
    Available,
    EngineUnavailable,
    ProfileRequired,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderProfile {
    pub provider_id: String,
    pub adapter_id: String,
    pub display_name_zh: String,
    pub default_base_url: Option<String>,
    pub base_url_env_var: Option<String>,
    pub default_api_mode: ProviderApiMode,
    pub preferred_engine_id: String,
    pub fallback_engine_ids: Vec<String>,
    pub auth_modes: Vec<ProviderAuthMode>,
    pub capabilities: Vec<String>,
    pub status: ProviderAdapterStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderRuntime {
    pub provider_id: String,
    pub adapter_id: String,
    pub engine_id: String,
    pub api_mode: ProviderApiMode,
    pub auth_mode: ProviderAuthMode,
    pub model: String,
    pub base_url: Option<String>,
    pub auth_ref: Option<AuthRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelRequest {
    pub instance_id: String,
    pub conversation_id: String,
    pub prompt: String,
    pub resolved_model: ResolvedModel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptBuildInput {
    pub instance_id: String,
    pub system_rules: String,
    pub relevant_memory: Vec<MemoryRecord>,
    pub user_request: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptBuildResult {
    pub prompt: String,
    pub memory_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelResponse {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderError {
    Resolve(String),
    Invoke(String),
}
