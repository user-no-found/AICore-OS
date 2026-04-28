use aicore_kernel::{ConversationStatus, GatewaySource, IngressResult, TurnStatus};
use aicore_provider::{ProviderError, ProviderKind};

pub(crate) fn ingress_decision_name(ingress: &IngressResult) -> &'static str {
    match ingress.decision {
        aicore_kernel::InterruptDecision::StartTurn => "start_turn",
        aicore_kernel::InterruptDecision::Queue => "queue",
        aicore_kernel::InterruptDecision::AppendContext => "append_context",
        aicore_kernel::InterruptDecision::SoftInterrupt => "soft_interrupt",
        aicore_kernel::InterruptDecision::HardInterrupt => "hard_interrupt",
    }
}

pub(crate) fn gateway_source_name(source: &GatewaySource) -> &'static str {
    match source {
        GatewaySource::Cli => "cli",
        GatewaySource::Tui => "tui",
        GatewaySource::Web => "web",
        GatewaySource::External => "external",
    }
}

pub(crate) fn turn_status_name(status: &TurnStatus) -> &'static str {
    match status {
        TurnStatus::Running => "running",
        TurnStatus::Completed => "completed",
        TurnStatus::Interrupted => "interrupted",
        TurnStatus::CancelRequested => "cancel_requested",
    }
}

pub(crate) fn conversation_status_name(status: &ConversationStatus) -> &'static str {
    match status {
        ConversationStatus::Idle => "idle",
        ConversationStatus::Running => "running",
        ConversationStatus::Queued => "queued",
        ConversationStatus::Interrupted => "interrupted",
    }
}

pub(crate) fn provider_kind_name(kind: &ProviderKind) -> &'static str {
    match kind {
        ProviderKind::Dummy => "dummy",
        ProviderKind::OpenRouter => "openrouter",
        ProviderKind::OpenAI => "openai",
        ProviderKind::Anthropic => "anthropic",
        ProviderKind::Kimi => "kimi",
        ProviderKind::KimiCoding => "kimi-coding",
        ProviderKind::DeepSeek => "deepseek",
        ProviderKind::Glm => "glm",
        ProviderKind::MiniMax => "minimax",
        ProviderKind::MiniMaxOpenAI => "minimax-openai",
        ProviderKind::OpenAICodexLogin => "openai-codex-login",
        ProviderKind::CustomOpenAICompatible => "custom-openai-compatible",
        ProviderKind::CustomAnthropicCompatible => "custom-anthropic-compatible",
        ProviderKind::Xiaomi => "xiaomi",
    }
}

pub(crate) fn provider_error_message(error: ProviderError) -> String {
    match error {
        ProviderError::Resolve(message) => message,
        ProviderError::Invoke(message) => message,
    }
}
