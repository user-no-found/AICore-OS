use aicore_auth::GlobalAuthPool;
use aicore_config::InstanceRuntimeConfig;
use aicore_memory::{MemoryKernel, MemoryScope, SearchQuery};
use aicore_provider::{
    DummyProvider, ModelRequest, PromptBuildInput, PromptBuilder, ProviderError, ProviderResolver,
};
use aicore_runtime::{
    GatewaySource, IngressResult, InstanceRuntime, InterruptMode, TransportEnvelope,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentTurnInput {
    pub instance_id: String,
    pub scope: MemoryScope,
    pub user_input: String,
    pub memory_query: Option<String>,
    pub memory_limit: Option<usize>,
    pub memory_token_budget: usize,
    pub system_rules: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentTurnOutput {
    pub assistant_output: String,
    pub memory_count: usize,
    pub provider_name: String,
    pub provider_kind: String,
    pub prompt: String,
    pub prompt_builder_ok: bool,
    pub runtime_output_ok: bool,
    pub ingress_decision: String,
    pub conversation_id: String,
    pub active_turn_id: Option<String>,
    pub event_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentTurnError(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentTurnRunner;

impl AgentTurnRunner {
    pub fn run(
        runtime: &mut InstanceRuntime,
        memory_kernel: &MemoryKernel,
        auth_pool: &GlobalAuthPool,
        runtime_config: &InstanceRuntimeConfig,
        input: AgentTurnInput,
    ) -> Result<AgentTurnOutput, AgentTurnError> {
        let ingress =
            runtime.handle_ingress(cli_envelope(), &input.user_input, InterruptMode::Queue);
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
        let resolved = ProviderResolver::resolve_primary(auth_pool, runtime_config)
            .map_err(map_provider_error)?;
        let request = ModelRequest {
            instance_id: input.instance_id,
            conversation_id: runtime.summary().conversation_id.clone(),
            prompt: prompt.prompt.clone(),
            resolved_model: resolved.clone(),
        };
        let response = DummyProvider::generate(&request);
        let outputs = runtime.append_assistant_output(&response.content);
        let runtime_output_ok = outputs
            .events
            .iter()
            .any(|event| event.content == response.content);

        if !runtime_output_ok {
            return Err(AgentTurnError("runtime 未收到 provider 输出".to_string()));
        }

        Ok(AgentTurnOutput {
            assistant_output: response.content,
            memory_count: memory_pack.len(),
            provider_name: resolved.provider,
            provider_kind: provider_kind_name(&resolved.kind).to_string(),
            prompt: prompt.prompt,
            prompt_builder_ok: true,
            runtime_output_ok,
            ingress_decision: ingress_decision_name(&ingress).to_string(),
            conversation_id: runtime.summary().conversation_id,
            active_turn_id: ingress.active_turn_id,
            event_count: runtime.summary().event_count,
        })
    }
}

fn cli_envelope() -> TransportEnvelope {
    TransportEnvelope {
        source: GatewaySource::Cli,
        platform: None,
        target_id: None,
        sender_id: None,
        is_group: false,
        mentioned_bot: false,
    }
}

fn ingress_decision_name(ingress: &IngressResult) -> &'static str {
    match ingress.decision {
        aicore_runtime::InterruptDecision::StartTurn => "start_turn",
        aicore_runtime::InterruptDecision::Queue => "queue",
        aicore_runtime::InterruptDecision::AppendContext => "append_context",
        aicore_runtime::InterruptDecision::SoftInterrupt => "soft_interrupt",
        aicore_runtime::InterruptDecision::HardInterrupt => "hard_interrupt",
    }
}

fn provider_kind_name(kind: &aicore_provider::ProviderKind) -> &'static str {
    match kind {
        aicore_provider::ProviderKind::Dummy => "dummy",
    }
}

fn map_provider_error(error: ProviderError) -> AgentTurnError {
    match error {
        ProviderError::Resolve(message) => AgentTurnError(message),
    }
}

#[cfg(test)]
mod tests {
    use std::{env, fs};

    use aicore_auth::{AuthCapability, AuthEntry, AuthKind, AuthRef, GlobalAuthPool, SecretRef};
    use aicore_config::{InstanceRuntimeConfig, ModelBinding};
    use aicore_memory::{MemoryKernel, MemoryPaths, MemoryPermanence, MemoryType, RememberInput};
    use aicore_runtime::default_runtime;

    use super::{AgentTurnInput, AgentTurnRunner};

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
        GlobalAuthPool::new(vec![AuthEntry {
            auth_ref: AuthRef::new("auth.openrouter.main"),
            provider: "openrouter".to_string(),
            kind: AuthKind::ApiKey,
            secret_ref: SecretRef::new("secret://auth.openrouter.main"),
            capabilities: vec![AuthCapability::Chat],
            enabled: true,
        }])
    }

    fn runtime_config() -> InstanceRuntimeConfig {
        InstanceRuntimeConfig {
            instance_id: "global-main".to_string(),
            primary: ModelBinding {
                auth_ref: AuthRef::new("auth.openrouter.main"),
                model: "openai/gpt-5".to_string(),
            },
            fallback: None,
        }
    }

    fn base_input(user_input: &str) -> AgentTurnInput {
        AgentTurnInput {
            instance_id: "global-main".to_string(),
            scope: global_scope(),
            user_input: user_input.to_string(),
            memory_query: None,
            memory_limit: Some(8),
            memory_token_budget: 128,
            system_rules: "You are the AICore instance runtime.".to_string(),
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

        assert!(output.prompt.contains("RELEVANT MEMORY:"));
        assert!(output.prompt.contains("memory context item"));
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

        assert!(output.prompt.contains("background context only"));
        assert!(output.prompt.contains("not the current user instruction"));
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

        assert!(output.prompt.ends_with("final request section"));
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

        let core_pos = output
            .prompt
            .find("[core]")
            .expect("core memory should exist");
        let decision_pos = output
            .prompt
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

        assert!(!output.prompt.contains("archived context"));
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

        assert_eq!(output.provider_name, "openrouter");
        assert_eq!(output.provider_kind, "dummy");
        assert!(output.assistant_output.contains("dummy provider response"));
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

        assert!(output.runtime_output_ok);
        assert!(output.event_count >= 2);
        assert!(output.active_turn_id.is_some());
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
}
