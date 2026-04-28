#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentTurnFailureStage {
    ProviderResolve,
    ProviderInvoke,
    RuntimeAppend,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentTurnError(pub String);
