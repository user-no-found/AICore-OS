use aicore_auth::GlobalAuthPool;
use aicore_config::InstanceRuntimeConfig;
use aicore_kernel::{IngressResult, InstanceRuntime};
use aicore_memory::{MemoryKernel, SearchQuery};
use aicore_provider::{
    ModelRequest, PromptBuildInput, PromptBuilder, ProviderInvoker, ProviderResolver,
};

use crate::names::{
    conversation_status_name, gateway_source_name, ingress_decision_name, provider_error_message,
    provider_kind_name, turn_status_name,
};
use crate::turn::{
    AgentTurnDebug, AgentTurnFailureStage, AgentTurnInput, AgentTurnOutcome, AgentTurnOutput,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentTurnRunner;

impl AgentTurnRunner {
    pub fn run(
        runtime: &mut InstanceRuntime,
        memory_kernel: &MemoryKernel,
        auth_pool: &GlobalAuthPool,
        runtime_config: &InstanceRuntimeConfig,
        input: AgentTurnInput,
    ) -> Result<AgentTurnOutput, crate::turn::AgentTurnError> {
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
