use std::collections::BTreeMap;

use crate::{KernelEventEnvelope, KernelRouteRuntimeOutput};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KernelInvocationStatus {
    Completed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelInvocationResultRoute {
    pub component_id: String,
    pub app_id: String,
    pub capability_id: String,
    pub contract_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelInvocationResultEnvelope {
    pub invocation_id: String,
    pub trace_id: String,
    pub operation: String,
    pub status: KernelInvocationStatus,
    pub route: Option<KernelInvocationResultRoute>,
    pub handler_kind: Option<String>,
    pub result_kind: Option<String>,
    pub summary: String,
    pub public_fields: BTreeMap<String, String>,
    pub failure_stage: Option<String>,
    pub failure_reason: Option<String>,
    pub handler_executed: bool,
    pub event_generated: bool,
    pub ledger_appended: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelInvocationRuntimeOutput {
    pub status: KernelInvocationStatus,
    pub route: Option<KernelRouteRuntimeOutput>,
    pub event: Option<KernelEventEnvelope>,
    pub result: Option<KernelInvocationResultEnvelope>,
    pub route_decision_made: bool,
    pub handler_executed: bool,
    pub event_generated: bool,
    pub handler_kind: Option<String>,
    pub failure_stage: Option<String>,
    pub failure_reason: Option<String>,
    pub spawned_process: bool,
    pub called_real_component: bool,
    pub transport: Option<String>,
    pub process_exit_code: Option<i32>,
    pub ledger_appended: bool,
    pub ledger_path: Option<String>,
    pub ledger_record_count: usize,
}
