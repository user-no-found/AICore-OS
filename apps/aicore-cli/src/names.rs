use aicore_agent::{AgentSessionStopReason, AgentTurnOutcome};
use aicore_auth::{AuthCapability, AuthKind, SecretRef};
use aicore_config::{ServiceProfileMode, ServiceRole};
use aicore_kernel::{DeliveryIdentity, OutputTarget};
use aicore_memory::{MemoryPermanence, MemorySource, MemoryType};

pub(crate) fn agent_turn_outcome_name(outcome: &AgentTurnOutcome) -> &'static str {
    match outcome {
        AgentTurnOutcome::Completed => "completed",
        AgentTurnOutcome::Queued => "queued",
        AgentTurnOutcome::AppendedContext => "appended_context",
        AgentTurnOutcome::Interrupted => "interrupted",
        AgentTurnOutcome::Failed => "failed",
    }
}

pub(crate) fn agent_turn_failure_stage_name(
    stage: &aicore_agent::AgentTurnFailureStage,
) -> &'static str {
    match stage {
        aicore_agent::AgentTurnFailureStage::ProviderResolve => "provider_resolve",
        aicore_agent::AgentTurnFailureStage::ProviderInvoke => "provider_invoke",
        aicore_agent::AgentTurnFailureStage::RuntimeAppend => "runtime_append",
    }
}

pub(crate) fn agent_session_stop_reason_name(reason: &AgentSessionStopReason) -> &'static str {
    match reason {
        AgentSessionStopReason::Failed => "failed",
        AgentSessionStopReason::Queued => "queued",
        AgentSessionStopReason::AppendedContext => "appended_context",
        AgentSessionStopReason::Interrupted => "interrupted",
    }
}

pub(crate) fn bool_status_name(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

pub(crate) fn output_target_name(target: &OutputTarget) -> &'static str {
    match target {
        OutputTarget::Origin => "origin",
        OutputTarget::ActiveViews => "active-views",
        OutputTarget::FollowedExternal => "followed-external",
    }
}

pub(crate) fn delivery_identity_name(identity: &DeliveryIdentity) -> String {
    match identity {
        DeliveryIdentity::ActiveViews => "active-views".to_string(),
        DeliveryIdentity::External {
            platform,
            target_id,
        } => {
            format!("external:{platform}:{target_id}")
        }
    }
}

pub(crate) fn init_status_name(created: bool) -> &'static str {
    if created {
        "已创建"
    } else {
        "已存在，未覆盖"
    }
}

pub(crate) fn secret_config_status(secret_ref: &SecretRef) -> &'static str {
    if secret_ref.as_str().is_empty() {
        "missing"
    } else {
        "configured"
    }
}

pub(crate) fn auth_kind_name(kind: &AuthKind) -> &'static str {
    match kind {
        AuthKind::ApiKey => "api-key",
        AuthKind::OAuth => "oauth",
        AuthKind::Session => "session",
        AuthKind::Token => "token",
    }
}

pub(crate) fn auth_capability_name(capability: &AuthCapability) -> &'static str {
    match capability {
        AuthCapability::Chat => "chat",
        AuthCapability::Vision => "vision",
        AuthCapability::Search => "search",
        AuthCapability::Embedding => "embedding",
    }
}

pub(crate) fn service_role_name(role: &ServiceRole) -> &'static str {
    match role {
        ServiceRole::MemoryExtractor => "memory_extractor",
        ServiceRole::MemoryCurator => "memory_curator",
        ServiceRole::MemoryDreamer => "memory_dreamer",
        ServiceRole::EvolutionProposer => "evolution_proposer",
        ServiceRole::EvolutionReviewer => "evolution_reviewer",
        ServiceRole::Search => "search",
        ServiceRole::Tts => "tts",
        ServiceRole::ImageGeneration => "image_generation",
        ServiceRole::VideoGeneration => "video_generation",
        ServiceRole::Vision => "vision",
        ServiceRole::Reranker => "reranker",
    }
}

pub(crate) fn service_mode_name(mode: &ServiceProfileMode) -> &'static str {
    match mode {
        ServiceProfileMode::InheritInstance => "inherit_instance",
        ServiceProfileMode::Explicit => "explicit",
        ServiceProfileMode::Disabled => "disabled",
    }
}

pub(crate) fn memory_type_name(memory_type: &MemoryType) -> &'static str {
    match memory_type {
        MemoryType::Core => "core",
        MemoryType::Working => "working",
        MemoryType::Status => "status",
        MemoryType::Decision => "decision",
    }
}

pub(crate) fn memory_source_name(source: &MemorySource) -> &'static str {
    match source {
        MemorySource::UserExplicit => "user_explicit",
        MemorySource::UserCorrection => "user_correction",
        MemorySource::AssistantSummary => "assistant_summary",
        MemorySource::RuleBasedAgent => "rule_based_agent",
    }
}

pub(crate) fn memory_permanence_name(permanence: &MemoryPermanence) -> &'static str {
    match permanence {
        MemoryPermanence::Standard => "standard",
        MemoryPermanence::Permanent => "permanent",
    }
}

pub(crate) fn provider_kind_name(kind: &aicore_provider::ProviderKind) -> &'static str {
    match kind {
        aicore_provider::ProviderKind::Dummy => "dummy",
        aicore_provider::ProviderKind::OpenRouter => "openrouter",
        aicore_provider::ProviderKind::OpenAI => "openai",
        aicore_provider::ProviderKind::Anthropic => "anthropic",
        aicore_provider::ProviderKind::Kimi => "kimi",
        aicore_provider::ProviderKind::KimiCoding => "kimi-coding",
        aicore_provider::ProviderKind::DeepSeek => "deepseek",
        aicore_provider::ProviderKind::Glm => "glm",
        aicore_provider::ProviderKind::MiniMax => "minimax",
        aicore_provider::ProviderKind::MiniMaxOpenAI => "minimax-openai",
        aicore_provider::ProviderKind::OpenAICodexLogin => "openai-codex-login",
        aicore_provider::ProviderKind::CustomOpenAICompatible => "custom-openai-compatible",
        aicore_provider::ProviderKind::CustomAnthropicCompatible => "custom-anthropic-compatible",
        aicore_provider::ProviderKind::Xiaomi => "xiaomi",
    }
}

pub(crate) fn provider_availability_name(
    availability: &aicore_provider::ProviderAvailability,
) -> &'static str {
    match availability {
        aicore_provider::ProviderAvailability::Available => "available",
        aicore_provider::ProviderAvailability::AdapterUnavailable => "boundary",
    }
}
