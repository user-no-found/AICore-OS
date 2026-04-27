use aicore_auth::GlobalAuthPool;
use aicore_config::InstanceRuntimeConfig;
use aicore_kernel::{
    ConversationStatus, GatewaySource, IngressResult, InstanceRuntime, InterruptMode,
    TransportEnvelope, TurnStatus,
};
use aicore_memory::{MemoryKernel, MemoryScope, SearchQuery};
use aicore_provider::{
    ModelRequest, PromptBuildInput, PromptBuilder, ProviderError, ProviderInvoker, ProviderResolver,
};

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentTurnOutcome {
    Completed,
    Queued,
    AppendedContext,
    Interrupted,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentTurnFailureStage {
    ProviderResolve,
    ProviderInvoke,
    RuntimeAppend,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnSurfaceEntry {
    pub conversation_id: String,
    pub turn_id: Option<String>,
    pub accepted_source: String,
    pub ingress_decision: String,
    pub outcome: AgentTurnOutcome,
    pub conversation_status: String,
    pub active_turn_status: Option<String>,
    pub queue_len: usize,
    pub event_count: usize,
    pub memory_count: usize,
    pub assistant_output_present: bool,
    pub provider_invoked: bool,
    pub provider_kind: Option<String>,
    pub provider_name: Option<String>,
    pub failure_stage: Option<AgentTurnFailureStage>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversationSurface {
    pub conversation_id: String,
    pub latest_turn: TurnSurfaceEntry,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentSessionSurface {
    pub conversation_id: String,
    pub turn_count: usize,
    pub latest_turn: Option<TurnSurfaceEntry>,
    pub turns: Vec<TurnSurfaceEntry>,
    pub event_count: usize,
    pub queue_len: usize,
    pub conversation_status: String,
    pub completed_all_inputs: bool,
    pub stop_reason: Option<AgentSessionStopReason>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentSessionOutput {
    surface: AgentSessionSurface,
    turn_outputs: Vec<AgentTurnOutput>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentTurnOutput {
    pub assistant_output: Option<String>,
    pub memory_count: usize,
    pub provider_name: Option<String>,
    pub provider_kind: Option<String>,
    pub prompt_builder_ok: bool,
    pub runtime_output_ok: bool,
    pub provider_invoked: bool,
    pub assistant_output_generated: bool,
    pub outcome: AgentTurnOutcome,
    pub error_message: Option<String>,
    pub failure_stage: Option<AgentTurnFailureStage>,
    pub accepted_source: String,
    pub ingress_decision: String,
    pub conversation_id: String,
    pub active_turn_id: Option<String>,
    pub active_turn_status: Option<String>,
    pub conversation_status: String,
    pub event_count: usize,
    pub queue_len: usize,
    pub debug: Option<AgentTurnDebug>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentTurnError(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentTurnRunner;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentSessionRunner;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentSessionContinuationPolicy {
    ContinueAll,
    StopOnFailed,
    StopOnNonCompleted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentSessionStopReason {
    Failed,
    Queued,
    AppendedContext,
    Interrupted,
}

impl AgentTurnOutput {
    pub fn to_surface_entry(&self) -> TurnSurfaceEntry {
        TurnSurfaceEntry {
            conversation_id: self.conversation_id.clone(),
            turn_id: self.active_turn_id.clone(),
            accepted_source: self.accepted_source.clone(),
            ingress_decision: self.ingress_decision.clone(),
            outcome: self.outcome.clone(),
            conversation_status: self.conversation_status.clone(),
            active_turn_status: self.active_turn_status.clone(),
            queue_len: self.queue_len,
            event_count: self.event_count,
            memory_count: self.memory_count,
            assistant_output_present: self.assistant_output.is_some(),
            provider_invoked: self.provider_invoked,
            provider_kind: self.provider_kind.clone(),
            provider_name: self.provider_name.clone(),
            failure_stage: self.failure_stage.clone(),
            error_message: self.error_message.clone(),
        }
    }

    pub fn to_conversation_surface(&self) -> ConversationSurface {
        ConversationSurface {
            conversation_id: self.conversation_id.clone(),
            latest_turn: self.to_surface_entry(),
        }
    }
}

impl AgentSessionOutput {
    pub fn surface(&self) -> &AgentSessionSurface {
        &self.surface
    }

    pub fn debug_turn_outputs(&self) -> &[AgentTurnOutput] {
        &self.turn_outputs
    }
}

impl AgentSessionRunner {
    pub fn run(
        runtime: &mut InstanceRuntime,
        memory_kernel: &MemoryKernel,
        auth_pool: &GlobalAuthPool,
        runtime_config: &InstanceRuntimeConfig,
        inputs: Vec<AgentTurnInput>,
    ) -> Result<AgentSessionOutput, AgentTurnError> {
        Self::run_with_policy(
            runtime,
            memory_kernel,
            auth_pool,
            runtime_config,
            inputs,
            AgentSessionContinuationPolicy::ContinueAll,
        )
    }

    pub fn run_with_policy(
        runtime: &mut InstanceRuntime,
        memory_kernel: &MemoryKernel,
        auth_pool: &GlobalAuthPool,
        runtime_config: &InstanceRuntimeConfig,
        inputs: Vec<AgentTurnInput>,
        policy: AgentSessionContinuationPolicy,
    ) -> Result<AgentSessionOutput, AgentTurnError> {
        let mut turn_outputs = Vec::new();
        let total_inputs = inputs.len();
        let mut completed_all_inputs = true;
        let mut stop_reason = None;

        for input in inputs {
            let output =
                AgentTurnRunner::run(runtime, memory_kernel, auth_pool, runtime_config, input)?;
            let outcome = output.outcome.clone();
            turn_outputs.push(output);
            if let Some(reason) = session_stop_reason(&policy, &outcome) {
                completed_all_inputs = turn_outputs.len() == total_inputs;
                stop_reason = Some(reason);
                break;
            }
        }

        Ok(AgentSessionOutput {
            surface: session_surface_from_outputs(
                runtime,
                &turn_outputs,
                completed_all_inputs,
                stop_reason,
            ),
            turn_outputs,
        })
    }
}

impl AgentTurnRunner {
    pub fn run(
        runtime: &mut InstanceRuntime,
        memory_kernel: &MemoryKernel,
        auth_pool: &GlobalAuthPool,
        runtime_config: &InstanceRuntimeConfig,
        input: AgentTurnInput,
    ) -> Result<AgentTurnOutput, AgentTurnError> {
        let ingress = runtime.handle_ingress(
            input.transport_envelope,
            &input.user_input,
            input.interrupt_mode,
        );
        match ingress.decision {
            aicore_kernel::InterruptDecision::StartTurn => {}
            aicore_kernel::InterruptDecision::Queue => {
                return Ok(non_generated_output(
                    runtime,
                    &ingress,
                    AgentTurnOutcome::Queued,
                ));
            }
            aicore_kernel::InterruptDecision::AppendContext => {
                return Ok(non_generated_output(
                    runtime,
                    &ingress,
                    AgentTurnOutcome::AppendedContext,
                ));
            }
            aicore_kernel::InterruptDecision::SoftInterrupt
            | aicore_kernel::InterruptDecision::HardInterrupt => {
                return Ok(non_generated_output(
                    runtime,
                    &ingress,
                    AgentTurnOutcome::Interrupted,
                ));
            }
        }

        let memory_query = input
            .memory_query
            .clone()
            .unwrap_or_else(|| input.user_input.clone());
        let memory_pack = memory_kernel.build_memory_context_pack(
            SearchQuery {
                text: memory_query,
                scope: Some(input.scope),
                memory_type: None,
                source: None,
                permanence: None,
                limit: input.memory_limit,
            },
            input.memory_token_budget,
        );
        let prompt = PromptBuilder::build(PromptBuildInput {
            instance_id: input.instance_id.clone(),
            system_rules: input.system_rules,
            relevant_memory: memory_pack.clone(),
            user_request: input.user_input,
        });
        let prompt_text = prompt.prompt;
        let prompt_length = prompt_text.len();
        let debug = AgentTurnDebug {
            prompt: input.include_debug_prompt.then_some(prompt_text.clone()),
            prompt_length,
            prompt_sections: vec![
                "SYSTEM".to_string(),
                "MEMORY SNAPSHOT".to_string(),
                "RELEVANT MEMORY".to_string(),
                "CURRENT USER REQUEST".to_string(),
            ],
            memory_ids: memory_pack
                .iter()
                .map(|record| record.memory_id.clone())
                .collect(),
        };
        let resolved = match ProviderResolver::resolve_primary(auth_pool, runtime_config) {
            Ok(resolved) => resolved,
            Err(error) => {
                runtime.complete_turn();
                return Ok(failed_output(
                    runtime,
                    &ingress,
                    AgentTurnFailureStage::ProviderResolve,
                    provider_error_message(error),
                    false,
                    None,
                    None,
                    memory_pack.len(),
                    true,
                    Some(debug),
                ));
            }
        };
        let request = ModelRequest {
            instance_id: input.instance_id,
            conversation_id: runtime.summary().conversation_id.clone(),
            prompt: prompt_text.clone(),
            resolved_model: resolved.clone(),
        };
        let provider_name = resolved.provider.clone();
        let provider_kind = provider_kind_name(&resolved.kind).to_string();
        let response = match ProviderInvoker::invoke(&request) {
            Ok(response) => response,
            Err(error) => {
                runtime.complete_turn();
                return Ok(failed_output(
                    runtime,
                    &ingress,
                    AgentTurnFailureStage::ProviderInvoke,
                    provider_error_message(error),
                    false,
                    Some(provider_name),
                    Some(provider_kind),
                    memory_pack.len(),
                    true,
                    Some(debug),
                ));
            }
        };
        let outputs = runtime.append_assistant_output(&response.content);
        let runtime_output_ok = outputs
            .events
            .iter()
            .any(|event| event.content == response.content);

        if !runtime_output_ok {
            runtime.complete_turn();
            return Ok(failed_output(
                runtime,
                &ingress,
                AgentTurnFailureStage::RuntimeAppend,
                "runtime 未收到 provider 输出".to_string(),
                true,
                Some(provider_name.clone()),
                Some(provider_kind.clone()),
                memory_pack.len(),
                true,
                Some(debug),
            ));
        }

        runtime.complete_turn();
        let turn_state = runtime.turn_state();
        let runtime_summary = runtime.summary();

        Ok(AgentTurnOutput {
            assistant_output: Some(response.content),
            memory_count: memory_pack.len(),
            provider_name: Some(provider_name),
            provider_kind: Some(provider_kind),
            prompt_builder_ok: true,
            runtime_output_ok,
            provider_invoked: true,
            assistant_output_generated: true,
            outcome: AgentTurnOutcome::Completed,
            error_message: None,
            failure_stage: None,
            accepted_source: gateway_source_name(&ingress.accepted_source).to_string(),
            ingress_decision: ingress_decision_name(&ingress).to_string(),
            conversation_id: runtime_summary.conversation_id,
            active_turn_id: ingress.active_turn_id,
            active_turn_status: turn_state
                .active_turn_status
                .as_ref()
                .map(turn_status_name)
                .map(ToString::to_string),
            conversation_status: conversation_status_name(&runtime_summary.conversation_status)
                .to_string(),
            event_count: runtime_summary.event_count,
            queue_len: turn_state.queue_len,
            debug: Some(debug),
        })
    }
}

fn non_generated_output(
    runtime: &InstanceRuntime,
    ingress: &IngressResult,
    outcome: AgentTurnOutcome,
) -> AgentTurnOutput {
    let turn_state = runtime.turn_state();
    let runtime_summary = runtime.summary();
    AgentTurnOutput {
        assistant_output: None,
        memory_count: 0,
        provider_name: None,
        provider_kind: None,
        prompt_builder_ok: false,
        runtime_output_ok: false,
        provider_invoked: false,
        assistant_output_generated: false,
        outcome,
        error_message: None,
        failure_stage: None,
        accepted_source: gateway_source_name(&ingress.accepted_source).to_string(),
        ingress_decision: ingress_decision_name(ingress).to_string(),
        conversation_id: runtime_summary.conversation_id,
        active_turn_id: ingress.active_turn_id.clone(),
        active_turn_status: turn_state
            .active_turn_status
            .as_ref()
            .map(turn_status_name)
            .map(ToString::to_string),
        conversation_status: conversation_status_name(&runtime_summary.conversation_status)
            .to_string(),
        event_count: runtime_summary.event_count,
        queue_len: turn_state.queue_len,
        debug: Some(AgentTurnDebug {
            prompt: None,
            prompt_length: 0,
            prompt_sections: Vec::new(),
            memory_ids: Vec::new(),
        }),
    }
}

fn failed_output(
    runtime: &InstanceRuntime,
    ingress: &IngressResult,
    failure_stage: AgentTurnFailureStage,
    error_message: String,
    provider_invoked: bool,
    provider_name: Option<String>,
    provider_kind: Option<String>,
    memory_count: usize,
    prompt_builder_ok: bool,
    debug: Option<AgentTurnDebug>,
) -> AgentTurnOutput {
    let turn_state = runtime.turn_state();
    let runtime_summary = runtime.summary();
    AgentTurnOutput {
        assistant_output: None,
        memory_count,
        provider_name,
        provider_kind,
        prompt_builder_ok,
        runtime_output_ok: false,
        provider_invoked,
        assistant_output_generated: false,
        outcome: AgentTurnOutcome::Failed,
        error_message: Some(error_message),
        failure_stage: Some(failure_stage),
        accepted_source: gateway_source_name(&ingress.accepted_source).to_string(),
        ingress_decision: ingress_decision_name(ingress).to_string(),
        conversation_id: runtime_summary.conversation_id,
        active_turn_id: ingress.active_turn_id.clone(),
        active_turn_status: turn_state
            .active_turn_status
            .as_ref()
            .map(turn_status_name)
            .map(ToString::to_string),
        conversation_status: conversation_status_name(&runtime_summary.conversation_status)
            .to_string(),
        event_count: runtime_summary.event_count,
        queue_len: turn_state.queue_len,
        debug,
    }
}

fn session_surface_from_outputs(
    runtime: &InstanceRuntime,
    turn_outputs: &[AgentTurnOutput],
    completed_all_inputs: bool,
    stop_reason: Option<AgentSessionStopReason>,
) -> AgentSessionSurface {
    let runtime_summary = runtime.summary();
    let turns = turn_outputs
        .iter()
        .map(AgentTurnOutput::to_surface_entry)
        .collect::<Vec<_>>();
    AgentSessionSurface {
        conversation_id: runtime_summary.conversation_id,
        turn_count: turns.len(),
        latest_turn: turns.last().cloned(),
        turns,
        event_count: runtime_summary.event_count,
        queue_len: runtime.turn_state().queue_len,
        conversation_status: conversation_status_name(&runtime_summary.conversation_status)
            .to_string(),
        completed_all_inputs,
        stop_reason,
    }
}

fn session_stop_reason(
    policy: &AgentSessionContinuationPolicy,
    outcome: &AgentTurnOutcome,
) -> Option<AgentSessionStopReason> {
    match policy {
        AgentSessionContinuationPolicy::ContinueAll => None,
        AgentSessionContinuationPolicy::StopOnFailed => match outcome {
            AgentTurnOutcome::Failed => Some(AgentSessionStopReason::Failed),
            _ => None,
        },
        AgentSessionContinuationPolicy::StopOnNonCompleted => match outcome {
            AgentTurnOutcome::Completed => None,
            AgentTurnOutcome::Failed => Some(AgentSessionStopReason::Failed),
            AgentTurnOutcome::Queued => Some(AgentSessionStopReason::Queued),
            AgentTurnOutcome::AppendedContext => Some(AgentSessionStopReason::AppendedContext),
            AgentTurnOutcome::Interrupted => Some(AgentSessionStopReason::Interrupted),
        },
    }
}

fn ingress_decision_name(ingress: &IngressResult) -> &'static str {
    match ingress.decision {
        aicore_kernel::InterruptDecision::StartTurn => "start_turn",
        aicore_kernel::InterruptDecision::Queue => "queue",
        aicore_kernel::InterruptDecision::AppendContext => "append_context",
        aicore_kernel::InterruptDecision::SoftInterrupt => "soft_interrupt",
        aicore_kernel::InterruptDecision::HardInterrupt => "hard_interrupt",
    }
}

fn gateway_source_name(source: &GatewaySource) -> &'static str {
    match source {
        GatewaySource::Cli => "cli",
        GatewaySource::Tui => "tui",
        GatewaySource::Web => "web",
        GatewaySource::External => "external",
    }
}

fn turn_status_name(status: &TurnStatus) -> &'static str {
    match status {
        TurnStatus::Running => "running",
        TurnStatus::Completed => "completed",
        TurnStatus::Interrupted => "interrupted",
        TurnStatus::CancelRequested => "cancel_requested",
    }
}

fn conversation_status_name(status: &ConversationStatus) -> &'static str {
    match status {
        ConversationStatus::Idle => "idle",
        ConversationStatus::Running => "running",
        ConversationStatus::Queued => "queued",
        ConversationStatus::Interrupted => "interrupted",
    }
}

fn provider_kind_name(kind: &aicore_provider::ProviderKind) -> &'static str {
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

fn provider_error_message(error: ProviderError) -> String {
    match error {
        ProviderError::Resolve(message) => message,
        ProviderError::Invoke(message) => message,
    }
}

#[cfg(test)]
mod tests {
    use std::{env, fs};

    use aicore_auth::{AuthCapability, AuthEntry, AuthKind, AuthRef, GlobalAuthPool, SecretRef};
    use aicore_config::{InstanceRuntimeConfig, ModelBinding};
    use aicore_kernel::default_runtime;
    use aicore_kernel::{GatewaySource, InterruptMode, TransportEnvelope};
    use aicore_memory::{MemoryKernel, MemoryPaths, MemoryPermanence, MemoryType, RememberInput};

    use super::{
        AgentSessionContinuationPolicy, AgentSessionRunner, AgentSessionStopReason,
        AgentTurnFailureStage, AgentTurnInput, AgentTurnOutcome, AgentTurnRunner,
    };

    fn temp_paths(name: &str) -> MemoryPaths {
        let root = env::temp_dir().join(format!("aicore-agent-tests-{name}"));
        if root.exists() {
            fs::remove_dir_all(&root).expect("temp root should be removable");
        }
        MemoryPaths::new(root)
    }

    fn global_scope() -> aicore_memory::MemoryScope {
        aicore_memory::MemoryScope::GlobalMain {
            instance_id: "global-main".to_string(),
        }
    }

    fn auth_pool() -> GlobalAuthPool {
        GlobalAuthPool::new(vec![
            AuthEntry {
                auth_ref: AuthRef::new("auth.dummy.main"),
                provider: "dummy".to_string(),
                kind: AuthKind::ApiKey,
                secret_ref: SecretRef::new("secret://auth.dummy.main"),
                capabilities: vec![AuthCapability::Chat],
                enabled: true,
            },
            AuthEntry {
                auth_ref: AuthRef::new("auth.openrouter.main"),
                provider: "openrouter".to_string(),
                kind: AuthKind::ApiKey,
                secret_ref: SecretRef::new("secret://auth.openrouter.main"),
                capabilities: vec![AuthCapability::Chat],
                enabled: true,
            },
            AuthEntry {
                auth_ref: AuthRef::new("auth.openai.main"),
                provider: "openai".to_string(),
                kind: AuthKind::ApiKey,
                secret_ref: SecretRef::new("secret://auth.openai.main"),
                capabilities: vec![AuthCapability::Chat],
                enabled: true,
            },
        ])
    }

    fn auth_pool_search_only() -> GlobalAuthPool {
        GlobalAuthPool::new(vec![AuthEntry {
            auth_ref: AuthRef::new("auth.search.only"),
            provider: "dummy".to_string(),
            kind: AuthKind::ApiKey,
            secret_ref: SecretRef::new("secret://auth.search.only"),
            capabilities: vec![AuthCapability::Search],
            enabled: true,
        }])
    }

    fn runtime_config() -> InstanceRuntimeConfig {
        InstanceRuntimeConfig {
            instance_id: "global-main".to_string(),
            primary: ModelBinding {
                auth_ref: AuthRef::new("auth.dummy.main"),
                model: "dummy/default-chat".to_string(),
            },
            fallback: None,
        }
    }

    fn runtime_config_openrouter() -> InstanceRuntimeConfig {
        InstanceRuntimeConfig {
            instance_id: "global-main".to_string(),
            primary: ModelBinding {
                auth_ref: AuthRef::new("auth.openrouter.main"),
                model: "openai/gpt-5".to_string(),
            },
            fallback: None,
        }
    }

    fn runtime_config_missing_auth() -> InstanceRuntimeConfig {
        InstanceRuntimeConfig {
            instance_id: "global-main".to_string(),
            primary: ModelBinding {
                auth_ref: AuthRef::new("auth.missing"),
                model: "openai/gpt-5".to_string(),
            },
            fallback: None,
        }
    }

    fn runtime_config_search_only() -> InstanceRuntimeConfig {
        InstanceRuntimeConfig {
            instance_id: "global-main".to_string(),
            primary: ModelBinding {
                auth_ref: AuthRef::new("auth.search.only"),
                model: "dummy/default-chat".to_string(),
            },
            fallback: None,
        }
    }

    fn base_input(user_input: &str) -> AgentTurnInput {
        AgentTurnInput {
            instance_id: "global-main".to_string(),
            transport_envelope: TransportEnvelope {
                source: GatewaySource::Cli,
                platform: None,
                target_id: None,
                sender_id: None,
                is_group: false,
                mentioned_bot: false,
            },
            interrupt_mode: InterruptMode::Queue,
            scope: global_scope(),
            user_input: user_input.to_string(),
            memory_query: None,
            memory_limit: Some(8),
            memory_token_budget: 128,
            system_rules: "You are the AICore instance runtime.".to_string(),
            include_debug_prompt: true,
        }
    }

    #[test]
    fn agent_turn_builds_prompt_with_memory_context() {
        let paths = temp_paths("agent-turn-memory-context");
        let mut memory = MemoryKernel::open(paths).expect("memory kernel should open");
        memory
            .remember_user_explicit(RememberInput {
                memory_type: MemoryType::Core,
                permanence: MemoryPermanence::Standard,
                scope: global_scope(),
                content: "memory context item".to_string(),
                localized_summary: "memory context item".to_string(),
                state_key: None,
                current_state: None,
            })
            .expect("remember should succeed");

        let mut runtime = default_runtime();
        let mut input = base_input("use memory context");
        input.memory_query = Some("memory context".to_string());
        let output = AgentTurnRunner::run(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config(),
            input,
        )
        .expect("agent turn should succeed");

        let prompt = output
            .debug
            .as_ref()
            .and_then(|debug| debug.prompt.as_ref())
            .expect("debug prompt should exist");
        assert!(prompt.contains("RELEVANT MEMORY:"));
        assert!(prompt.contains("memory context item"));
    }

    #[test]
    fn agent_turn_marks_memory_as_background_context() {
        let memory = MemoryKernel::open(temp_paths("agent-turn-background"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();
        let output = AgentTurnRunner::run(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config(),
            base_input("do work"),
        )
        .expect("agent turn should succeed");

        let prompt = output
            .debug
            .as_ref()
            .and_then(|debug| debug.prompt.as_ref())
            .expect("debug prompt should exist");
        assert!(prompt.contains("background context only"));
        assert!(prompt.contains("not the current user instruction"));
    }

    #[test]
    fn agent_turn_puts_current_user_request_last() {
        let memory = MemoryKernel::open(temp_paths("agent-turn-request-last"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();
        let output = AgentTurnRunner::run(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config(),
            base_input("final request section"),
        )
        .expect("agent turn should succeed");

        let prompt = output
            .debug
            .as_ref()
            .and_then(|debug| debug.prompt.as_ref())
            .expect("debug prompt should exist");
        assert!(prompt.ends_with("final request section"));
    }

    #[test]
    fn agent_turn_uses_search_result_order_for_memory_pack() {
        let paths = temp_paths("agent-turn-search-order");
        let mut memory = MemoryKernel::open(paths).expect("memory kernel should open");
        memory
            .remember_user_explicit(RememberInput {
                memory_type: MemoryType::Decision,
                permanence: MemoryPermanence::Permanent,
                scope: global_scope(),
                content: "shared memory".to_string(),
                localized_summary: "shared memory".to_string(),
                state_key: None,
                current_state: None,
            })
            .expect("remember should succeed");
        memory
            .remember_user_explicit(RememberInput {
                memory_type: MemoryType::Core,
                permanence: MemoryPermanence::Permanent,
                scope: global_scope(),
                content: "shared memory".to_string(),
                localized_summary: "shared memory".to_string(),
                state_key: None,
                current_state: None,
            })
            .expect("remember should succeed");

        let mut runtime = default_runtime();
        let output = AgentTurnRunner::run(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config(),
            base_input("shared"),
        )
        .expect("agent turn should succeed");

        let prompt = output
            .debug
            .as_ref()
            .and_then(|debug| debug.prompt.as_ref())
            .expect("debug prompt should exist");
        let core_pos = prompt.find("[core]").expect("core memory should exist");
        let decision_pos = prompt
            .find("[decision]")
            .expect("decision memory should exist");
        assert!(core_pos < decision_pos);
    }

    #[test]
    fn agent_turn_excludes_archived_memory() {
        let paths = temp_paths("agent-turn-archived");
        let mut memory = MemoryKernel::open(paths).expect("memory kernel should open");
        let archived_id = memory
            .remember_user_explicit(RememberInput {
                memory_type: MemoryType::Core,
                permanence: MemoryPermanence::Permanent,
                scope: global_scope(),
                content: "archived context".to_string(),
                localized_summary: "archived context".to_string(),
                state_key: None,
                current_state: None,
            })
            .expect("remember should succeed");
        memory
            .archive(&archived_id)
            .expect("archive should succeed");

        let mut runtime = default_runtime();
        let output = AgentTurnRunner::run(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config(),
            base_input("archived"),
        )
        .expect("agent turn should succeed");

        let prompt = output
            .debug
            .as_ref()
            .and_then(|debug| debug.prompt.as_ref())
            .expect("debug prompt should exist");
        assert!(!prompt.contains("archived context"));
        assert_eq!(output.memory_count, 0);
    }

    #[test]
    fn agent_turn_uses_provider_resolver_and_dummy_provider() {
        let memory = MemoryKernel::open(temp_paths("agent-turn-provider"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();
        let output = AgentTurnRunner::run(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config(),
            base_input("provider test"),
        )
        .expect("agent turn should succeed");

        assert_eq!(output.provider_name.as_deref(), Some("dummy"));
        assert_eq!(output.provider_kind.as_deref(), Some("dummy"));
        assert!(
            output
                .assistant_output
                .as_deref()
                .expect("assistant output should exist")
                .contains("dummy provider response")
        );
    }

    #[test]
    fn agent_turn_appends_assistant_output_to_runtime() {
        let memory = MemoryKernel::open(temp_paths("agent-turn-runtime-append"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();
        let output = AgentTurnRunner::run(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config(),
            base_input("append output"),
        )
        .expect("agent turn should succeed");

        assert_eq!(output.outcome, AgentTurnOutcome::Completed);
        assert!(output.runtime_output_ok);
        assert!(output.event_count >= 2);
        assert!(output.active_turn_id.is_some());
        assert_eq!(output.conversation_status, "idle");
        assert_eq!(output.active_turn_status, None);
    }

    #[test]
    fn agent_turn_returns_memory_count() {
        let paths = temp_paths("agent-turn-memory-count");
        let mut memory = MemoryKernel::open(paths).expect("memory kernel should open");
        memory
            .remember_user_explicit(RememberInput {
                memory_type: MemoryType::Core,
                permanence: MemoryPermanence::Standard,
                scope: global_scope(),
                content: "counted memory".to_string(),
                localized_summary: "counted memory".to_string(),
                state_key: None,
                current_state: None,
            })
            .expect("remember should succeed");

        let mut runtime = default_runtime();
        let output = AgentTurnRunner::run(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config(),
            base_input("counted"),
        )
        .expect("agent turn should succeed");

        assert_eq!(output.memory_count, 1);
        assert!(output.prompt_builder_ok);
    }

    #[test]
    fn agent_turn_public_output_does_not_expose_full_prompt() {
        let memory = MemoryKernel::open(temp_paths("agent-turn-public-no-prompt"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();
        let mut input = base_input("hide prompt");
        input.include_debug_prompt = false;

        let output = AgentTurnRunner::run(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config(),
            input,
        )
        .expect("agent turn should succeed");

        let debug = output.debug.expect("debug metadata should exist");
        assert!(debug.prompt.is_none());
        assert!(debug.prompt_length > 0);
    }

    #[test]
    fn agent_turn_debug_prompt_requires_explicit_request() {
        let memory = MemoryKernel::open(temp_paths("agent-turn-debug-explicit"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();
        let without_debug =
            AgentTurnRunner::run(&mut runtime, &memory, &auth_pool(), &runtime_config(), {
                let mut input = base_input("no debug prompt");
                input.include_debug_prompt = false;
                input
            })
            .expect("agent turn should succeed");
        assert!(
            without_debug
                .debug
                .as_ref()
                .expect("debug metadata should exist")
                .prompt
                .is_none()
        );

        let with_debug =
            AgentTurnRunner::run(&mut runtime, &memory, &auth_pool(), &runtime_config(), {
                let mut input = base_input("with debug prompt");
                input.include_debug_prompt = true;
                input
            })
            .expect("agent turn should succeed");
        assert!(
            with_debug
                .debug
                .as_ref()
                .expect("debug metadata should exist")
                .prompt
                .is_some()
        );
    }

    #[test]
    fn agent_turn_returns_conversation_surface_metadata() {
        let memory = MemoryKernel::open(temp_paths("agent-turn-surface-metadata"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();
        let output = AgentTurnRunner::run(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config(),
            base_input("surface metadata"),
        )
        .expect("agent turn should succeed");

        assert_eq!(output.accepted_source, "cli");
        assert!(!output.ingress_decision.is_empty());
        assert!(!output.conversation_id.is_empty());
        assert!(output.event_count >= 2);
    }

    #[test]
    fn agent_turn_uses_supplied_transport_envelope() {
        let memory = MemoryKernel::open(temp_paths("agent-turn-supplied-envelope"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();
        let mut input = base_input("external envelope");
        input.transport_envelope = TransportEnvelope {
            source: GatewaySource::External,
            platform: Some("feishu".to_string()),
            target_id: Some("chat-1".to_string()),
            sender_id: Some("user-1".to_string()),
            is_group: true,
            mentioned_bot: true,
        };

        let output = AgentTurnRunner::run(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config(),
            input,
        )
        .expect("agent turn should succeed");

        assert_eq!(output.accepted_source, "external");
    }

    #[test]
    fn agent_turn_no_longer_hardcodes_cli_envelope() {
        let memory = MemoryKernel::open(temp_paths("agent-turn-no-hardcoded-cli"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();
        let mut input = base_input("tui envelope");
        input.transport_envelope = TransportEnvelope {
            source: GatewaySource::Tui,
            platform: None,
            target_id: None,
            sender_id: None,
            is_group: false,
            mentioned_bot: false,
        };

        let output = AgentTurnRunner::run(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config(),
            input,
        )
        .expect("agent turn should succeed");

        assert_eq!(output.accepted_source, "tui");
    }

    #[test]
    fn agent_turn_start_turn_invokes_provider_and_appends_output() {
        let memory = MemoryKernel::open(temp_paths("agent-turn-start-turn-provider"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();

        let output = AgentTurnRunner::run(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config(),
            base_input("start turn"),
        )
        .expect("agent turn should succeed");

        assert_eq!(output.outcome, AgentTurnOutcome::Completed);
        assert!(output.provider_invoked);
        assert!(output.assistant_output_generated);
        assert!(output.assistant_output.is_some());
    }

    #[test]
    fn agent_turn_start_turn_completes_turn() {
        let memory = MemoryKernel::open(temp_paths("agent-turn-completes-turn"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();

        let output = AgentTurnRunner::run(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config(),
            base_input("complete turn"),
        )
        .expect("agent turn should succeed");

        assert_eq!(output.outcome, AgentTurnOutcome::Completed);
        assert_eq!(runtime.turn_state().active_turn_id, None);
        assert_eq!(
            runtime.summary().conversation_status,
            aicore_kernel::ConversationStatus::Idle
        );
    }

    #[test]
    fn agent_turn_queue_decision_does_not_invoke_provider() {
        let memory = MemoryKernel::open(temp_paths("agent-turn-queue-no-provider"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();
        runtime.handle_ingress(
            TransportEnvelope {
                source: GatewaySource::Cli,
                platform: None,
                target_id: None,
                sender_id: None,
                is_group: false,
                mentioned_bot: false,
            },
            "existing turn",
            InterruptMode::Queue,
        );

        let output = AgentTurnRunner::run(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config(),
            base_input("queued input"),
        )
        .expect("agent turn should succeed");

        assert_eq!(output.outcome, AgentTurnOutcome::Queued);
        assert!(!output.provider_invoked);
        assert!(!output.assistant_output_generated);
        assert_eq!(output.assistant_output, None);
    }

    #[test]
    fn agent_turn_queue_decision_does_not_append_assistant_output() {
        let memory = MemoryKernel::open(temp_paths("agent-turn-queue-no-append"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();
        runtime.handle_ingress(
            TransportEnvelope {
                source: GatewaySource::Cli,
                platform: None,
                target_id: None,
                sender_id: None,
                is_group: false,
                mentioned_bot: false,
            },
            "existing turn",
            InterruptMode::Queue,
        );
        let initial_event_count = runtime.summary().event_count;

        let output = AgentTurnRunner::run(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config(),
            base_input("queued input"),
        )
        .expect("agent turn should succeed");

        assert_eq!(output.event_count, runtime.summary().event_count);
        assert_eq!(runtime.summary().event_count, initial_event_count);
    }

    #[test]
    fn agent_turn_queue_result_reports_queue_len() {
        let memory = MemoryKernel::open(temp_paths("agent-turn-queue-len"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();
        runtime.handle_ingress(
            TransportEnvelope {
                source: GatewaySource::Cli,
                platform: None,
                target_id: None,
                sender_id: None,
                is_group: false,
                mentioned_bot: false,
            },
            "existing turn",
            InterruptMode::Queue,
        );

        let output = AgentTurnRunner::run(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config(),
            base_input("queued input"),
        )
        .expect("agent turn should succeed");

        assert_eq!(output.outcome, AgentTurnOutcome::Queued);
        assert!(output.queue_len >= 1);
    }

    #[test]
    fn agent_turn_soft_interrupt_does_not_invoke_provider() {
        let memory = MemoryKernel::open(temp_paths("agent-turn-soft-interrupt"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();
        runtime.handle_ingress(
            TransportEnvelope {
                source: GatewaySource::Cli,
                platform: None,
                target_id: None,
                sender_id: None,
                is_group: false,
                mentioned_bot: false,
            },
            "existing turn",
            InterruptMode::Queue,
        );

        let output =
            AgentTurnRunner::run(&mut runtime, &memory, &auth_pool(), &runtime_config(), {
                let mut input = base_input("soft interrupt");
                input.interrupt_mode = InterruptMode::SoftInterrupt;
                input
            })
            .expect("agent turn should succeed");

        assert_eq!(output.outcome, AgentTurnOutcome::Interrupted);
        assert_eq!(output.ingress_decision, "soft_interrupt");
        assert!(!output.provider_invoked);
    }

    #[test]
    fn agent_turn_hard_interrupt_does_not_invoke_provider() {
        let memory = MemoryKernel::open(temp_paths("agent-turn-hard-interrupt"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();
        runtime.handle_ingress(
            TransportEnvelope {
                source: GatewaySource::Cli,
                platform: None,
                target_id: None,
                sender_id: None,
                is_group: false,
                mentioned_bot: false,
            },
            "existing turn",
            InterruptMode::Queue,
        );

        let output =
            AgentTurnRunner::run(&mut runtime, &memory, &auth_pool(), &runtime_config(), {
                let mut input = base_input("hard interrupt");
                input.interrupt_mode = InterruptMode::HardInterrupt;
                input
            })
            .expect("agent turn should succeed");

        assert_eq!(output.outcome, AgentTurnOutcome::Interrupted);
        assert_eq!(output.ingress_decision, "hard_interrupt");
        assert!(!output.provider_invoked);
    }

    #[test]
    fn agent_turn_interrupt_result_reports_decision() {
        let memory = MemoryKernel::open(temp_paths("agent-turn-interrupt-decision"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();
        runtime.handle_ingress(
            TransportEnvelope {
                source: GatewaySource::Cli,
                platform: None,
                target_id: None,
                sender_id: None,
                is_group: false,
                mentioned_bot: false,
            },
            "existing turn",
            InterruptMode::Queue,
        );

        let output =
            AgentTurnRunner::run(&mut runtime, &memory, &auth_pool(), &runtime_config(), {
                let mut input = base_input("soft interrupt");
                input.interrupt_mode = InterruptMode::SoftInterrupt;
                input
            })
            .expect("agent turn should succeed");

        assert_eq!(output.ingress_decision, "soft_interrupt");
        assert_eq!(output.active_turn_status.as_deref(), Some("interrupted"));
        assert_eq!(output.conversation_status, "interrupted");
    }

    #[test]
    fn agent_turn_completed_result_contains_assistant_output() {
        let memory = MemoryKernel::open(temp_paths("agent-turn-completed-output"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();

        let output = AgentTurnRunner::run(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config(),
            base_input("completed output"),
        )
        .expect("agent turn should succeed");

        assert_eq!(output.outcome, AgentTurnOutcome::Completed);
        assert!(output.assistant_output.is_some());
    }

    #[test]
    fn agent_turn_non_generated_result_has_no_assistant_output() {
        let memory = MemoryKernel::open(temp_paths("agent-turn-no-generated-output"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();
        runtime.handle_ingress(
            TransportEnvelope {
                source: GatewaySource::Cli,
                platform: None,
                target_id: None,
                sender_id: None,
                is_group: false,
                mentioned_bot: false,
            },
            "existing turn",
            InterruptMode::Queue,
        );

        let output = AgentTurnRunner::run(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config(),
            base_input("queued input"),
        )
        .expect("agent turn should succeed");

        assert_ne!(output.outcome, AgentTurnOutcome::Completed);
        assert_eq!(output.assistant_output, None);
    }

    #[test]
    fn agent_turn_provider_resolve_failure_returns_failed_outcome() {
        let memory = MemoryKernel::open(temp_paths("agent-turn-provider-resolve-failed"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();

        let output = AgentTurnRunner::run(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config_missing_auth(),
            base_input("provider resolve failure"),
        )
        .expect("agent turn should return structured failure");

        assert_eq!(output.outcome, AgentTurnOutcome::Failed);
        assert_eq!(
            output.failure_stage,
            Some(AgentTurnFailureStage::ProviderResolve)
        );
    }

    #[test]
    fn agent_turn_provider_resolve_failure_does_not_invoke_provider() {
        let memory = MemoryKernel::open(temp_paths("agent-turn-provider-resolve-no-provider"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();

        let output = AgentTurnRunner::run(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config_missing_auth(),
            base_input("provider resolve failure"),
        )
        .expect("agent turn should return structured failure");

        assert_eq!(output.provider_invoked, false);
        assert_eq!(output.assistant_output_generated, false);
    }

    #[test]
    fn agent_turn_non_chat_auth_returns_provider_resolve_failure() {
        let memory = MemoryKernel::open(temp_paths("agent-turn-non-chat-auth-failed"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();

        let output = AgentTurnRunner::run(
            &mut runtime,
            &memory,
            &auth_pool_search_only(),
            &runtime_config_search_only(),
            base_input("non chat auth"),
        )
        .expect("agent turn should return structured resolve failure");

        assert_eq!(output.outcome, AgentTurnOutcome::Failed);
        assert_eq!(
            output.failure_stage,
            Some(AgentTurnFailureStage::ProviderResolve)
        );
        assert_eq!(output.provider_invoked, false);
    }

    #[test]
    fn agent_turn_non_chat_auth_does_not_append_assistant_output() {
        let memory = MemoryKernel::open(temp_paths("agent-turn-non-chat-auth-no-append"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();
        let initial_event_count = runtime.summary().event_count;

        let output = AgentTurnRunner::run(
            &mut runtime,
            &memory,
            &auth_pool_search_only(),
            &runtime_config_search_only(),
            base_input("non chat auth"),
        )
        .expect("agent turn should return structured resolve failure");

        assert_eq!(output.event_count, initial_event_count + 1);
        assert_eq!(runtime.summary().event_count, initial_event_count + 1);
        assert_eq!(output.assistant_output, None);
    }

    #[test]
    fn agent_turn_non_chat_auth_does_not_leave_turn_running() {
        let memory = MemoryKernel::open(temp_paths("agent-turn-non-chat-auth-no-running"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();

        let output = AgentTurnRunner::run(
            &mut runtime,
            &memory,
            &auth_pool_search_only(),
            &runtime_config_search_only(),
            base_input("non chat auth"),
        )
        .expect("agent turn should return structured resolve failure");

        assert_eq!(output.outcome, AgentTurnOutcome::Failed);
        assert_eq!(runtime.turn_state().active_turn_id, None);
        assert_eq!(
            runtime.summary().conversation_status,
            aicore_kernel::ConversationStatus::Idle
        );
    }

    #[test]
    fn agent_turn_real_provider_unavailable_returns_failed_outcome() {
        let memory = MemoryKernel::open(temp_paths("agent-turn-provider-invoke-failed"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();

        let output = AgentTurnRunner::run(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config_openrouter(),
            base_input("provider invoke failure"),
        )
        .expect("agent turn should return structured failure");

        assert_eq!(output.outcome, AgentTurnOutcome::Failed);
        assert_eq!(
            output.failure_stage,
            Some(AgentTurnFailureStage::ProviderInvoke)
        );
        assert_eq!(output.provider_invoked, false);
        assert_eq!(output.assistant_output, None);
    }

    #[test]
    fn agent_turn_provider_invoke_failure_does_not_append_assistant_output() {
        let memory = MemoryKernel::open(temp_paths("agent-turn-provider-invoke-no-append"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();
        let initial_event_count = runtime.summary().event_count;

        let output = AgentTurnRunner::run(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config_openrouter(),
            base_input("provider invoke failure"),
        )
        .expect("agent turn should return structured failure");

        assert_eq!(output.event_count, initial_event_count + 1);
        assert_eq!(runtime.summary().event_count, initial_event_count + 1);
        assert_eq!(output.assistant_output, None);
    }

    #[test]
    fn agent_turn_provider_invoke_failure_does_not_leave_turn_running() {
        let memory = MemoryKernel::open(temp_paths("agent-turn-provider-invoke-no-running"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();

        let output = AgentTurnRunner::run(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config_openrouter(),
            base_input("provider invoke failure"),
        )
        .expect("agent turn should return structured failure");

        assert_eq!(output.outcome, AgentTurnOutcome::Failed);
        assert_eq!(runtime.turn_state().active_turn_id, None);
        assert_eq!(
            runtime.summary().conversation_status,
            aicore_kernel::ConversationStatus::Idle
        );
    }

    #[test]
    fn agent_turn_provider_invoke_failure_reports_failure_stage() {
        let memory = MemoryKernel::open(temp_paths("agent-turn-provider-invoke-stage"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();

        let output = AgentTurnRunner::run(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config_openrouter(),
            base_input("provider invoke failure"),
        )
        .expect("agent turn should return structured failure");

        assert_eq!(
            output.failure_stage,
            Some(AgentTurnFailureStage::ProviderInvoke)
        );
        assert_eq!(output.provider_kind.as_deref(), Some("openrouter"));
        assert_eq!(output.provider_name.as_deref(), Some("openrouter"));
        assert!(
            output
                .error_message
                .as_deref()
                .expect("failure message should exist")
                .contains("adapter unavailable")
        );
        assert!(
            !output
                .error_message
                .as_deref()
                .expect("failure message should exist")
                .contains("secret://")
        );
    }

    #[test]
    fn agent_turn_provider_resolve_failure_does_not_append_assistant_output() {
        let memory = MemoryKernel::open(temp_paths("agent-turn-provider-resolve-no-append"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();
        let initial_event_count = runtime.summary().event_count;

        let output = AgentTurnRunner::run(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config_missing_auth(),
            base_input("provider resolve failure"),
        )
        .expect("agent turn should return structured failure");

        assert_eq!(output.event_count, initial_event_count + 1);
        assert_eq!(runtime.summary().event_count, initial_event_count + 1);
    }

    #[test]
    fn agent_turn_provider_resolve_failure_does_not_leave_active_turn_running() {
        let memory = MemoryKernel::open(temp_paths("agent-turn-provider-resolve-no-running-turn"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();

        let output = AgentTurnRunner::run(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config_missing_auth(),
            base_input("provider resolve failure"),
        )
        .expect("agent turn should return structured failure");

        assert_eq!(output.outcome, AgentTurnOutcome::Failed);
        assert_eq!(runtime.turn_state().active_turn_id, None);
        assert_eq!(
            runtime.summary().conversation_status,
            aicore_kernel::ConversationStatus::Idle
        );
    }

    #[test]
    fn agent_turn_failed_result_has_no_assistant_output() {
        let memory = MemoryKernel::open(temp_paths("agent-turn-failed-no-output"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();

        let output = AgentTurnRunner::run(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config_missing_auth(),
            base_input("provider resolve failure"),
        )
        .expect("agent turn should return structured failure");

        assert_eq!(output.outcome, AgentTurnOutcome::Failed);
        assert_eq!(output.assistant_output, None);
    }

    #[test]
    fn agent_turn_failed_result_reports_failure_stage() {
        let memory = MemoryKernel::open(temp_paths("agent-turn-failure-stage"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();

        let output = AgentTurnRunner::run(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config_missing_auth(),
            base_input("provider resolve failure"),
        )
        .expect("agent turn should return structured failure");

        assert_eq!(
            output.failure_stage,
            Some(AgentTurnFailureStage::ProviderResolve)
        );
        assert!(
            output
                .error_message
                .as_deref()
                .expect("failure message should exist")
                .contains("auth")
        );
    }

    #[test]
    fn agent_turn_non_generated_debug_prompt_is_none() {
        let memory = MemoryKernel::open(temp_paths("agent-turn-non-generated-no-debug-prompt"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();
        runtime.handle_ingress(
            TransportEnvelope {
                source: GatewaySource::Cli,
                platform: None,
                target_id: None,
                sender_id: None,
                is_group: false,
                mentioned_bot: false,
            },
            "existing turn",
            InterruptMode::Queue,
        );

        let output = AgentTurnRunner::run(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config(),
            base_input("queued input"),
        )
        .expect("agent turn should succeed");

        assert!(
            output
                .debug
                .as_ref()
                .expect("debug metadata should exist")
                .prompt
                .is_none()
        );
    }

    #[test]
    fn conversation_surface_includes_completed_turn() {
        let memory = MemoryKernel::open(temp_paths("surface-completed-turn"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();

        let output = AgentTurnRunner::run(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config(),
            base_input("completed surface"),
        )
        .expect("agent turn should succeed");

        let surface = output.to_conversation_surface();
        assert_eq!(surface.latest_turn.outcome, AgentTurnOutcome::Completed);
        assert!(surface.latest_turn.assistant_output_present);
    }

    #[test]
    fn conversation_surface_includes_failed_turn() {
        let memory = MemoryKernel::open(temp_paths("surface-failed-turn"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();

        let output = AgentTurnRunner::run(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config_missing_auth(),
            base_input("failed surface"),
        )
        .expect("agent turn should return failed outcome");

        let surface = output.to_conversation_surface();
        assert_eq!(surface.latest_turn.outcome, AgentTurnOutcome::Failed);
        assert_eq!(
            surface.latest_turn.failure_stage,
            Some(AgentTurnFailureStage::ProviderResolve)
        );
    }

    #[test]
    fn conversation_surface_includes_queued_turn() {
        let memory = MemoryKernel::open(temp_paths("surface-queued-turn"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();
        runtime.handle_ingress(
            TransportEnvelope {
                source: GatewaySource::Cli,
                platform: None,
                target_id: None,
                sender_id: None,
                is_group: false,
                mentioned_bot: false,
            },
            "existing turn",
            InterruptMode::Queue,
        );

        let output = AgentTurnRunner::run(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config(),
            base_input("queued surface"),
        )
        .expect("agent turn should succeed");

        let surface = output.to_conversation_surface();
        assert_eq!(surface.latest_turn.outcome, AgentTurnOutcome::Queued);
        assert!(!surface.latest_turn.assistant_output_present);
    }

    #[test]
    fn conversation_surface_includes_interrupted_turn() {
        let memory = MemoryKernel::open(temp_paths("surface-interrupted-turn"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();
        runtime.handle_ingress(
            TransportEnvelope {
                source: GatewaySource::Cli,
                platform: None,
                target_id: None,
                sender_id: None,
                is_group: false,
                mentioned_bot: false,
            },
            "existing turn",
            InterruptMode::Queue,
        );

        let output =
            AgentTurnRunner::run(&mut runtime, &memory, &auth_pool(), &runtime_config(), {
                let mut input = base_input("interrupt surface");
                input.interrupt_mode = InterruptMode::SoftInterrupt;
                input
            })
            .expect("agent turn should succeed");

        let surface = output.to_conversation_surface();
        assert_eq!(surface.latest_turn.outcome, AgentTurnOutcome::Interrupted);
        assert_eq!(surface.latest_turn.ingress_decision, "soft_interrupt");
    }

    #[test]
    fn conversation_surface_includes_appended_context_turn() {
        let memory = MemoryKernel::open(temp_paths("surface-appended-context-turn"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();
        runtime.handle_ingress(
            TransportEnvelope {
                source: GatewaySource::Cli,
                platform: None,
                target_id: None,
                sender_id: None,
                is_group: false,
                mentioned_bot: false,
            },
            "existing turn",
            InterruptMode::Queue,
        );

        let output =
            AgentTurnRunner::run(&mut runtime, &memory, &auth_pool(), &runtime_config(), {
                let mut input = base_input("append context");
                input.interrupt_mode = InterruptMode::AppendContext;
                input
            })
            .expect("agent turn should succeed");

        assert_eq!(output.outcome, AgentTurnOutcome::AppendedContext);
        assert!(!output.provider_invoked);
        assert_eq!(output.assistant_output, None);

        let surface = output.to_conversation_surface();
        assert_eq!(
            surface.latest_turn.outcome,
            AgentTurnOutcome::AppendedContext
        );
    }

    #[test]
    fn conversation_surface_does_not_expose_prompt() {
        let memory =
            MemoryKernel::open(temp_paths("surface-no-prompt")).expect("memory kernel should open");
        let mut runtime = default_runtime();

        let output = AgentTurnRunner::run(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config(),
            base_input("no prompt surface"),
        )
        .expect("agent turn should succeed");

        let surface = output.to_conversation_surface();
        let debug_text = format!("{surface:?}");
        assert!(!debug_text.contains("SYSTEM:"));
        assert!(!debug_text.contains("CURRENT USER REQUEST:"));
    }

    #[test]
    fn conversation_surface_does_not_expose_raw_memory() {
        let paths = temp_paths("surface-no-raw-memory");
        let mut memory = MemoryKernel::open(paths).expect("memory kernel should open");
        memory
            .remember_user_explicit(RememberInput {
                memory_type: MemoryType::Core,
                permanence: MemoryPermanence::Standard,
                scope: global_scope(),
                content: "raw memory payload".to_string(),
                localized_summary: "raw memory payload".to_string(),
                state_key: None,
                current_state: None,
            })
            .expect("remember should succeed");

        let mut runtime = default_runtime();
        let output = AgentTurnRunner::run(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config(),
            base_input("raw memory"),
        )
        .expect("agent turn should succeed");

        let surface = output.to_conversation_surface();
        let debug_text = format!("{surface:?}");
        assert!(!debug_text.contains("raw memory payload"));
    }

    #[test]
    fn conversation_surface_reports_failure_stage() {
        let memory = MemoryKernel::open(temp_paths("surface-failure-stage"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();

        let output = AgentTurnRunner::run(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config_missing_auth(),
            base_input("failure stage"),
        )
        .expect("agent turn should return failed outcome");

        let surface = output.to_conversation_surface();
        assert_eq!(
            surface.latest_turn.failure_stage,
            Some(AgentTurnFailureStage::ProviderResolve)
        );
    }

    #[test]
    fn conversation_surface_reports_provider_invoke_failure() {
        let memory = MemoryKernel::open(temp_paths("conversation-surface-provider-invoke"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();

        let output = AgentTurnRunner::run(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config_openrouter(),
            base_input("provider invoke failure"),
        )
        .expect("agent turn should return structured failure");
        let surface = output.to_conversation_surface();

        assert_eq!(
            surface.latest_turn.failure_stage,
            Some(AgentTurnFailureStage::ProviderInvoke)
        );
        assert_eq!(
            surface.latest_turn.provider_kind.as_deref(),
            Some("openrouter")
        );
    }

    #[test]
    fn conversation_surface_reports_provider_metadata_when_invoked() {
        let memory = MemoryKernel::open(temp_paths("surface-provider-metadata"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();

        let output = AgentTurnRunner::run(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config(),
            base_input("provider metadata"),
        )
        .expect("agent turn should succeed");

        let surface = output.to_conversation_surface();
        assert!(surface.latest_turn.provider_invoked);
        assert_eq!(surface.latest_turn.provider_kind.as_deref(), Some("dummy"));
        assert_eq!(surface.latest_turn.provider_name.as_deref(), Some("dummy"));
    }

    #[test]
    fn conversation_surface_reports_no_provider_when_not_invoked() {
        let memory = MemoryKernel::open(temp_paths("surface-no-provider"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();
        runtime.handle_ingress(
            TransportEnvelope {
                source: GatewaySource::Cli,
                platform: None,
                target_id: None,
                sender_id: None,
                is_group: false,
                mentioned_bot: false,
            },
            "existing turn",
            InterruptMode::Queue,
        );

        let output = AgentTurnRunner::run(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config(),
            base_input("queued surface"),
        )
        .expect("agent turn should succeed");

        let surface = output.to_conversation_surface();
        assert!(!surface.latest_turn.provider_invoked);
        assert_eq!(surface.latest_turn.provider_kind, None);
        assert_eq!(surface.latest_turn.provider_name, None);
    }

    #[test]
    fn agent_turn_provider_failure_surface_does_not_expose_secret_ref() {
        let memory = MemoryKernel::open(temp_paths("agent-turn-no-secret-ref"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();

        let output = AgentTurnRunner::run(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config_openrouter(),
            base_input("provider invoke failure"),
        )
        .expect("agent turn should return structured failure");

        let surface = output.to_conversation_surface();
        let rendered = format!("{surface:?}");
        assert!(!rendered.contains("secret://"));
    }

    #[test]
    fn agent_turn_provider_resolve_failure_surface_does_not_expose_secret_ref() {
        let memory = MemoryKernel::open(temp_paths("agent-turn-resolve-no-secret-ref"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();

        let output = AgentTurnRunner::run(
            &mut runtime,
            &memory,
            &auth_pool_search_only(),
            &runtime_config_search_only(),
            base_input("non chat auth"),
        )
        .expect("agent turn should return structured resolve failure");

        let surface = output.to_conversation_surface();
        let rendered = format!("{surface:?}");
        assert!(!rendered.contains("secret://"));
    }

    #[test]
    fn conversation_surface_preserves_conversation_id() {
        let memory = MemoryKernel::open(temp_paths("surface-conversation-id"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();

        let output = AgentTurnRunner::run(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config(),
            base_input("conversation id"),
        )
        .expect("agent turn should succeed");

        let surface = output.to_conversation_surface();
        assert_eq!(surface.conversation_id, output.conversation_id);
        assert_eq!(surface.latest_turn.conversation_id, output.conversation_id);
    }

    #[test]
    fn conversation_surface_uses_runtime_event_count() {
        let memory = MemoryKernel::open(temp_paths("surface-event-count"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();

        let output = AgentTurnRunner::run(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config(),
            base_input("event count"),
        )
        .expect("agent turn should succeed");

        let surface = output.to_conversation_surface();
        assert_eq!(surface.latest_turn.event_count, output.event_count);
    }

    #[test]
    fn agent_turn_completed_can_be_converted_to_surface_entry() {
        let memory = MemoryKernel::open(temp_paths("surface-entry-completed"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();

        let output = AgentTurnRunner::run(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config(),
            base_input("completed entry"),
        )
        .expect("agent turn should succeed");

        let entry = output.to_surface_entry();
        assert_eq!(entry.outcome, AgentTurnOutcome::Completed);
    }

    #[test]
    fn agent_turn_failed_can_be_converted_to_surface_entry() {
        let memory = MemoryKernel::open(temp_paths("surface-entry-failed"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();

        let output = AgentTurnRunner::run(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config_missing_auth(),
            base_input("failed entry"),
        )
        .expect("agent turn should return failed outcome");

        let entry = output.to_surface_entry();
        assert_eq!(entry.outcome, AgentTurnOutcome::Failed);
    }

    #[test]
    fn agent_turn_queued_can_be_converted_to_surface_entry() {
        let memory = MemoryKernel::open(temp_paths("surface-entry-queued"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();
        runtime.handle_ingress(
            TransportEnvelope {
                source: GatewaySource::Cli,
                platform: None,
                target_id: None,
                sender_id: None,
                is_group: false,
                mentioned_bot: false,
            },
            "existing turn",
            InterruptMode::Queue,
        );

        let output = AgentTurnRunner::run(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config(),
            base_input("queued entry"),
        )
        .expect("agent turn should succeed");

        let entry = output.to_surface_entry();
        assert_eq!(entry.outcome, AgentTurnOutcome::Queued);
    }

    #[test]
    fn agent_turn_interrupted_can_be_converted_to_surface_entry() {
        let memory = MemoryKernel::open(temp_paths("surface-entry-interrupted"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();
        runtime.handle_ingress(
            TransportEnvelope {
                source: GatewaySource::Cli,
                platform: None,
                target_id: None,
                sender_id: None,
                is_group: false,
                mentioned_bot: false,
            },
            "existing turn",
            InterruptMode::Queue,
        );

        let output =
            AgentTurnRunner::run(&mut runtime, &memory, &auth_pool(), &runtime_config(), {
                let mut input = base_input("interrupted entry");
                input.interrupt_mode = InterruptMode::SoftInterrupt;
                input
            })
            .expect("agent turn should succeed");

        let entry = output.to_surface_entry();
        assert_eq!(entry.outcome, AgentTurnOutcome::Interrupted);
    }

    #[test]
    fn agent_turn_appended_context_can_be_converted_to_surface_entry() {
        let memory = MemoryKernel::open(temp_paths("surface-entry-appended-context"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();
        runtime.handle_ingress(
            TransportEnvelope {
                source: GatewaySource::Cli,
                platform: None,
                target_id: None,
                sender_id: None,
                is_group: false,
                mentioned_bot: false,
            },
            "existing turn",
            InterruptMode::Queue,
        );

        let output =
            AgentTurnRunner::run(&mut runtime, &memory, &auth_pool(), &runtime_config(), {
                let mut input = base_input("append context entry");
                input.interrupt_mode = InterruptMode::AppendContext;
                input
            })
            .expect("agent turn should succeed");

        let entry = output.to_surface_entry();
        assert_eq!(entry.outcome, AgentTurnOutcome::AppendedContext);
    }

    #[test]
    fn agent_turn_does_not_auto_accept_memory_proposals() {
        let paths = temp_paths("agent-turn-no-auto-accept");
        let mut memory = MemoryKernel::open(paths).expect("memory kernel should open");
        memory
            .submit_assistant_summary(global_scope(), "proposal only memory")
            .expect("proposal should be created");
        let proposal_count = memory.proposals().len();
        let record_count = memory.records().len();

        let mut runtime = default_runtime();
        let _ = AgentTurnRunner::run(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config(),
            base_input("proposal should stay open"),
        )
        .expect("agent turn should succeed");

        assert_eq!(memory.proposals().len(), proposal_count);
        assert_eq!(memory.records().len(), record_count);
    }

    #[test]
    fn agent_session_runs_two_completed_turns() {
        let memory = MemoryKernel::open(temp_paths("agent-session-two-completed"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();
        let session = AgentSessionRunner::run(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config(),
            vec![base_input("first request"), base_input("second request")],
        )
        .expect("session should succeed");

        assert_eq!(session.surface.turn_count, 2);
        assert_eq!(session.surface.turns.len(), 2);
        assert_eq!(
            session.surface.turns[0].outcome,
            AgentTurnOutcome::Completed
        );
        assert_eq!(
            session.surface.turns[1].outcome,
            AgentTurnOutcome::Completed
        );
        assert!(session.surface.completed_all_inputs);
        assert_eq!(session.surface.stop_reason, None);
    }

    #[test]
    fn agent_session_follow_up_turn_uses_same_conversation_id() {
        let memory = MemoryKernel::open(temp_paths("agent-session-same-conversation"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();
        let session = AgentSessionRunner::run(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config(),
            vec![base_input("first"), base_input("second")],
        )
        .expect("session should succeed");

        assert_eq!(
            session.turn_outputs[0].conversation_id,
            session.turn_outputs[1].conversation_id
        );
    }

    #[test]
    fn agent_session_follow_up_turn_starts_after_completed_turn() {
        let memory = MemoryKernel::open(temp_paths("agent-session-follow-up-start"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();
        let session = AgentSessionRunner::run(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config(),
            vec![base_input("first"), base_input("second")],
        )
        .expect("session should succeed");

        assert_ne!(
            session.turn_outputs[0].active_turn_id,
            session.turn_outputs[1].active_turn_id
        );
        assert_eq!(runtime.turn_state().active_turn_id, None);
    }

    #[test]
    fn agent_session_event_count_increases_across_turns() {
        let memory = MemoryKernel::open(temp_paths("agent-session-event-count"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();
        let session = AgentSessionRunner::run(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config(),
            vec![base_input("first"), base_input("second")],
        )
        .expect("session should succeed");

        assert!(session.turn_outputs[1].event_count > session.turn_outputs[0].event_count);
    }

    #[test]
    fn agent_session_records_turn_history_entries() {
        let memory = MemoryKernel::open(temp_paths("agent-session-history"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();
        let session = AgentSessionRunner::run(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config(),
            vec![base_input("first"), base_input("second")],
        )
        .expect("session should succeed");

        assert_eq!(session.surface.turns.len(), 2);
    }

    #[test]
    fn agent_session_latest_turn_points_to_second_turn() {
        let memory = MemoryKernel::open(temp_paths("agent-session-latest-turn"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();
        let session = AgentSessionRunner::run(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config(),
            vec![base_input("first"), base_input("second")],
        )
        .expect("session should succeed");

        assert_eq!(
            session
                .surface
                .latest_turn
                .as_ref()
                .expect("latest turn")
                .turn_id,
            session.turn_outputs[1].active_turn_id
        );
    }

    #[test]
    fn agent_session_surface_is_public_read_contract() {
        let memory = MemoryKernel::open(temp_paths("agent-session-public-contract"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();
        let session = AgentSessionRunner::run(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config(),
            vec![base_input("first"), base_input("second")],
        )
        .expect("session should succeed");

        assert_eq!(session.surface().turn_count, 2);
        assert_eq!(session.surface().turns.len(), 2);
    }

    #[test]
    fn agent_session_output_turn_outputs_are_internal_debug_result() {
        let memory = MemoryKernel::open(temp_paths("agent-session-debug-result"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();
        let session = AgentSessionRunner::run(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config(),
            vec![base_input("first"), base_input("second")],
        )
        .expect("session should succeed");

        assert_eq!(session.debug_turn_outputs().len(), 2);
        assert!(
            session.debug_turn_outputs()[0]
                .debug
                .as_ref()
                .and_then(|debug| debug.prompt.as_ref())
                .is_some()
        );
    }

    #[test]
    fn agent_session_surface_does_not_expose_prompt() {
        let memory = MemoryKernel::open(temp_paths("agent-session-no-prompt"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();
        let session = AgentSessionRunner::run(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config(),
            vec![base_input("first"), base_input("second")],
        )
        .expect("session should succeed");

        let debug_text = format!("{:?}", session.surface);
        assert!(!debug_text.contains("SYSTEM:"));
        assert!(!debug_text.contains("CURRENT USER REQUEST:"));
    }

    #[test]
    fn agent_session_surface_does_not_expose_debug_prompt_even_when_requested() {
        let memory = MemoryKernel::open(temp_paths("agent-session-no-debug-prompt"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();
        let session = AgentSessionRunner::run(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config(),
            vec![base_input("first"), base_input("second")],
        )
        .expect("session should succeed");

        assert!(
            session.debug_turn_outputs()[0]
                .debug
                .as_ref()
                .and_then(|debug| debug.prompt.as_ref())
                .is_some()
        );
        let public_text = format!("{:?}", session.surface());
        assert!(!public_text.contains("SYSTEM:"));
        assert!(!public_text.contains("CURRENT USER REQUEST:"));
    }

    #[test]
    fn agent_session_surface_does_not_expose_raw_memory() {
        let paths = temp_paths("agent-session-no-raw-memory");
        let mut memory = MemoryKernel::open(paths).expect("memory kernel should open");
        memory
            .remember_user_explicit(RememberInput {
                memory_type: MemoryType::Core,
                permanence: MemoryPermanence::Standard,
                scope: global_scope(),
                content: "session raw memory".to_string(),
                localized_summary: "session raw memory".to_string(),
                state_key: None,
                current_state: None,
            })
            .expect("remember should succeed");
        let mut runtime = default_runtime();
        let session = AgentSessionRunner::run(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config(),
            vec![base_input("first"), base_input("second")],
        )
        .expect("session should succeed");

        let debug_text = format!("{:?}", session.surface);
        assert!(!debug_text.contains("session raw memory"));
    }

    #[test]
    fn agent_session_default_policy_continues_all_inputs() {
        let memory = MemoryKernel::open(temp_paths("agent-session-default-policy"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();
        let session = AgentSessionRunner::run(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config_missing_auth(),
            vec![base_input("failed one"), base_input("failed two")],
        )
        .expect("session should succeed");

        assert_eq!(session.surface().turn_count, 2);
        assert!(session.surface().completed_all_inputs);
        assert_eq!(session.surface().stop_reason, None);
    }

    #[test]
    fn agent_session_continue_all_records_failed_then_next_turn() {
        let memory = MemoryKernel::open(temp_paths("agent-session-continue-all"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();
        let session = AgentSessionRunner::run_with_policy(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config_missing_auth(),
            vec![base_input("failed one"), base_input("failed two")],
            AgentSessionContinuationPolicy::ContinueAll,
        )
        .expect("session should succeed");

        assert_eq!(session.surface().turn_count, 2);
        assert_eq!(session.surface().turns[0].outcome, AgentTurnOutcome::Failed);
        assert_eq!(session.surface().turns[1].outcome, AgentTurnOutcome::Failed);
    }

    #[test]
    fn agent_session_stop_on_failed_stops_after_failure() {
        let memory = MemoryKernel::open(temp_paths("agent-session-stop-on-failed"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();
        let session = AgentSessionRunner::run_with_policy(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config_missing_auth(),
            vec![base_input("failed one"), base_input("failed two")],
            AgentSessionContinuationPolicy::StopOnFailed,
        )
        .expect("session should succeed");

        assert_eq!(session.surface().turn_count, 1);
        assert!(!session.surface().completed_all_inputs);
        assert_eq!(
            session.surface().stop_reason,
            Some(AgentSessionStopReason::Failed)
        );
    }

    #[test]
    fn agent_session_stop_on_non_completed_stops_on_queued() {
        let memory = MemoryKernel::open(temp_paths("agent-session-stop-on-queued"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();
        runtime.handle_ingress(
            TransportEnvelope {
                source: GatewaySource::Cli,
                platform: None,
                target_id: None,
                sender_id: None,
                is_group: false,
                mentioned_bot: false,
            },
            "existing turn",
            InterruptMode::Queue,
        );
        let session = AgentSessionRunner::run_with_policy(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config(),
            vec![base_input("queued"), base_input("should not run")],
            AgentSessionContinuationPolicy::StopOnNonCompleted,
        )
        .expect("session should succeed");

        assert_eq!(session.surface().turn_count, 1);
        assert_eq!(session.surface().turns[0].outcome, AgentTurnOutcome::Queued);
        assert!(!session.surface().completed_all_inputs);
        assert_eq!(
            session.surface().stop_reason,
            Some(AgentSessionStopReason::Queued)
        );
    }

    #[test]
    fn agent_session_stop_on_non_completed_stops_on_interrupted() {
        let memory = MemoryKernel::open(temp_paths("agent-session-stop-on-interrupted"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();
        runtime.handle_ingress(
            TransportEnvelope {
                source: GatewaySource::Cli,
                platform: None,
                target_id: None,
                sender_id: None,
                is_group: false,
                mentioned_bot: false,
            },
            "existing turn",
            InterruptMode::Queue,
        );
        let session = AgentSessionRunner::run_with_policy(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config(),
            vec![
                {
                    let mut input = base_input("interrupt");
                    input.interrupt_mode = InterruptMode::SoftInterrupt;
                    input
                },
                base_input("should not run"),
            ],
            AgentSessionContinuationPolicy::StopOnNonCompleted,
        )
        .expect("session should succeed");

        assert_eq!(session.surface().turn_count, 1);
        assert_eq!(
            session.surface().turns[0].outcome,
            AgentTurnOutcome::Interrupted
        );
        assert!(!session.surface().completed_all_inputs);
        assert_eq!(
            session.surface().stop_reason,
            Some(AgentSessionStopReason::Interrupted)
        );
    }

    #[test]
    fn agent_session_surface_reports_completed_all_inputs() {
        let memory = MemoryKernel::open(temp_paths("agent-session-completed-all"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();
        let session = AgentSessionRunner::run(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config(),
            vec![base_input("first"), base_input("second")],
        )
        .expect("session should succeed");

        assert!(session.surface().completed_all_inputs);
    }

    #[test]
    fn agent_session_surface_reports_stop_reason() {
        let memory = MemoryKernel::open(temp_paths("agent-session-stop-reason"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();
        let session = AgentSessionRunner::run_with_policy(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config_missing_auth(),
            vec![base_input("failed"), base_input("next")],
            AgentSessionContinuationPolicy::StopOnFailed,
        )
        .expect("session should succeed");

        assert_eq!(
            session.surface().stop_reason,
            Some(AgentSessionStopReason::Failed)
        );
    }

    #[test]
    fn agent_session_surface_turn_count_matches_recorded_turns() {
        let memory = MemoryKernel::open(temp_paths("agent-session-turn-count-match"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();
        let session = AgentSessionRunner::run(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config(),
            vec![base_input("first"), base_input("second")],
        )
        .expect("session should succeed");

        assert_eq!(session.surface().turn_count, session.surface().turns.len());
    }

    #[test]
    fn agent_session_surface_latest_turn_matches_last_recorded_turn() {
        let memory = MemoryKernel::open(temp_paths("agent-session-latest-match"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();
        let session = AgentSessionRunner::run(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config(),
            vec![base_input("first"), base_input("second")],
        )
        .expect("session should succeed");

        assert_eq!(
            session.surface().latest_turn,
            session.surface().turns.last().cloned()
        );
    }

    #[test]
    fn agent_session_failed_turn_enters_history() {
        let memory = MemoryKernel::open(temp_paths("agent-session-failed-history"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();
        let session = AgentSessionRunner::run(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config_missing_auth(),
            vec![base_input("failed")],
        )
        .expect("session should succeed");

        assert_eq!(session.surface.turns[0].outcome, AgentTurnOutcome::Failed);
    }

    #[test]
    fn session_surface_records_provider_invoke_failure() {
        let memory = MemoryKernel::open(temp_paths("session-surface-provider-invoke"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();
        let session = AgentSessionRunner::run(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config_openrouter(),
            vec![base_input("provider invoke failure")],
        )
        .expect("session should succeed");

        assert_eq!(session.surface().turns[0].outcome, AgentTurnOutcome::Failed);
        assert_eq!(
            session.surface().turns[0].failure_stage,
            Some(AgentTurnFailureStage::ProviderInvoke)
        );
    }

    #[test]
    fn session_surface_provider_failure_does_not_expose_internal_request() {
        let memory = MemoryKernel::open(temp_paths("session-surface-no-internal-request"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();
        let session = AgentSessionRunner::run(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config_openrouter(),
            vec![base_input("provider invoke failure")],
        )
        .expect("session should succeed");

        let rendered = format!("{:?}", session.surface());
        assert!(!rendered.contains("RELEVANT MEMORY:"));
        assert!(!rendered.contains("CURRENT USER REQUEST:"));
        assert!(!rendered.contains("prompt"));
    }

    #[test]
    fn session_surface_provider_resolve_failure_does_not_expose_internal_request() {
        let memory = MemoryKernel::open(temp_paths("session-surface-resolve-no-internal-request"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();
        let session = AgentSessionRunner::run(
            &mut runtime,
            &memory,
            &auth_pool_search_only(),
            &runtime_config_search_only(),
            vec![base_input("non chat auth")],
        )
        .expect("session should succeed");

        let rendered = format!("{:?}", session.surface());
        assert!(!rendered.contains("RELEVANT MEMORY:"));
        assert!(!rendered.contains("CURRENT USER REQUEST:"));
        assert!(!rendered.contains("prompt"));
        assert!(!rendered.contains("secret://"));
    }

    #[test]
    fn agent_session_queued_turn_enters_history() {
        let memory = MemoryKernel::open(temp_paths("agent-session-queued-history"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();
        runtime.handle_ingress(
            TransportEnvelope {
                source: GatewaySource::Cli,
                platform: None,
                target_id: None,
                sender_id: None,
                is_group: false,
                mentioned_bot: false,
            },
            "existing turn",
            InterruptMode::Queue,
        );
        let session = AgentSessionRunner::run(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config(),
            vec![base_input("queued")],
        )
        .expect("session should succeed");

        assert_eq!(session.surface.turns[0].outcome, AgentTurnOutcome::Queued);
    }

    #[test]
    fn agent_session_interrupted_turn_enters_history() {
        let memory = MemoryKernel::open(temp_paths("agent-session-interrupted-history"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();
        runtime.handle_ingress(
            TransportEnvelope {
                source: GatewaySource::Cli,
                platform: None,
                target_id: None,
                sender_id: None,
                is_group: false,
                mentioned_bot: false,
            },
            "existing turn",
            InterruptMode::Queue,
        );
        let session = AgentSessionRunner::run(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config(),
            vec![{
                let mut input = base_input("interrupt");
                input.interrupt_mode = InterruptMode::SoftInterrupt;
                input
            }],
        )
        .expect("session should succeed");

        assert_eq!(
            session.surface.turns[0].outcome,
            AgentTurnOutcome::Interrupted
        );
    }

    #[test]
    fn follow_up_turn_keeps_memory_as_background_context() {
        let paths = temp_paths("agent-session-followup-memory-background");
        let mut memory = MemoryKernel::open(paths).expect("memory kernel should open");
        memory
            .remember_user_explicit(RememberInput {
                memory_type: MemoryType::Core,
                permanence: MemoryPermanence::Standard,
                scope: global_scope(),
                content: "background memory item".to_string(),
                localized_summary: "background memory item".to_string(),
                state_key: None,
                current_state: None,
            })
            .expect("remember should succeed");
        let mut runtime = default_runtime();
        let session = AgentSessionRunner::run(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config(),
            vec![base_input("first"), {
                let mut input = base_input("second");
                input.memory_query = Some("background memory".to_string());
                input
            }],
        )
        .expect("session should succeed");

        let prompt = session.turn_outputs[1]
            .debug
            .as_ref()
            .and_then(|debug| debug.prompt.as_ref())
            .expect("debug prompt should exist");
        assert!(prompt.contains("background context only"));
        assert!(prompt.contains("not the current user instruction"));
    }

    #[test]
    fn follow_up_turn_current_user_request_still_last_in_debug_prompt() {
        let memory = MemoryKernel::open(temp_paths("agent-session-followup-request-last"))
            .expect("memory kernel should open");
        let mut runtime = default_runtime();
        let session = AgentSessionRunner::run(
            &mut runtime,
            &memory,
            &auth_pool(),
            &runtime_config(),
            vec![base_input("first"), base_input("second request final")],
        )
        .expect("session should succeed");

        let prompt = session.turn_outputs[1]
            .debug
            .as_ref()
            .and_then(|debug| debug.prompt.as_ref())
            .expect("debug prompt should exist");
        assert!(prompt.ends_with("second request final"));
    }
}
