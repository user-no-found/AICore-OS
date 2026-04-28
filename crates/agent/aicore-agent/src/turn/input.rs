use aicore_kernel::{InterruptMode, TransportEnvelope};
use aicore_memory::MemoryScope;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentTurnInput {
    pub instance_id: String,
    pub transport_envelope: TransportEnvelope,
    pub interrupt_mode: InterruptMode,
    pub scope: MemoryScope,
    pub user_input: String,
    pub memory_query: Option<String>,
    pub memory_limit: Option<usize>,
    pub memory_token_budget: usize,
    pub system_rules: String,
    pub include_debug_prompt: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentTurnDebug {
    pub prompt: Option<String>,
    pub prompt_length: usize,
    pub prompt_sections: Vec<String>,
    pub memory_ids: Vec<String>,
}
