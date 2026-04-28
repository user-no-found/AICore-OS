use aicore_auth::GlobalAuthPool;
use aicore_config::InstanceRuntimeConfig;
use aicore_kernel::InstanceRuntime;
use aicore_memory::MemoryKernel;

use crate::session::policy::session_stop_reason;
use crate::session::surface::session_surface_from_outputs;
use crate::session::{AgentSessionContinuationPolicy, AgentSessionSurface};
use crate::turn::{AgentTurnError, AgentTurnInput, AgentTurnOutput, AgentTurnRunner};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentSessionOutput {
    surface: AgentSessionSurface,
    turn_outputs: Vec<AgentTurnOutput>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentSessionRunner;

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
