use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromptModuleKind {
    InstanceSoul,
    VisibleCapabilities,
    MemoryContext,
    SkillsContext,
    TeamContext,
    OutputContract,
    TransientNotices,
    UserMessage,
}

impl PromptModuleKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::InstanceSoul => "instance_soul",
            Self::VisibleCapabilities => "visible_capabilities",
            Self::MemoryContext => "memory_context",
            Self::SkillsContext => "skills_context",
            Self::TeamContext => "team_context",
            Self::OutputContract => "output_contract",
            Self::TransientNotices => "transient_notices",
            Self::UserMessage => "user_message",
        }
    }

    pub fn fixed_order() -> Vec<Self> {
        vec![
            Self::InstanceSoul,
            Self::VisibleCapabilities,
            Self::MemoryContext,
            Self::SkillsContext,
            Self::TeamContext,
            Self::OutputContract,
            Self::TransientNotices,
            Self::UserMessage,
        ]
    }

    pub fn from_contract_value(value: &str) -> Option<Self> {
        match value {
            "instance_soul" => Some(Self::InstanceSoul),
            "visible_capabilities" => Some(Self::VisibleCapabilities),
            "memory_context" => Some(Self::MemoryContext),
            "skills_context" => Some(Self::SkillsContext),
            "team_context" => Some(Self::TeamContext),
            "output_contract" => Some(Self::OutputContract),
            "transient_notices" => Some(Self::TransientNotices),
            "user_message" => Some(Self::UserMessage),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromptModuleSource {
    CurrentInstanceSoul,
    GlobalMainSoul,
    GlobalMainUserProfile,
    CapabilityProjection,
    VisibleMemorySummary,
    SkillContext,
    TeamSummary,
    OutputContract,
    TransientNotice,
    UserInput,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromptModuleVisibility {
    ModelVisible,
    UserVisibleSummary,
    HiddenByPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelProtocolVersion {
    V1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelResponseEventKind {
    RequestStarted,
    AssistantDelta,
    AssistantFinal,
    ProviderError,
    Cancelled,
    StoppedBeforeFinal,
    Completed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelStopReason {
    StopRequested,
    ProviderCompleted,
    ProviderError,
    Cancelled,
    MaxOutputTokens,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelRunStatus {
    NotFinal,
    Running,
    Completed,
    Failed,
    Cancelled,
    StoppedBeforeFinal,
}
