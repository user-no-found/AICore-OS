use crate::{
    KernelEventEnvelope, KernelInvocationEnvelope, KernelInvocationLedger,
    KernelInvocationResultEnvelope, KernelRouteRuntimeError, KernelRouteRuntimeOutput,
};

use super::{KernelInvocationRuntime, KernelInvocationRuntimeOutput, KernelInvocationStatus};

impl KernelInvocationRuntime {
    pub(super) fn route_failure(
        envelope: &KernelInvocationEnvelope,
        error: KernelRouteRuntimeError,
    ) -> KernelInvocationRuntimeOutput {
        let reason = error.to_string();
        KernelInvocationRuntimeOutput {
            status: KernelInvocationStatus::Failed,
            route: None,
            event: None,
            result: Some(Self::failure_result(
                envelope, None, "route", &reason, false, false, None,
            )),
            route_decision_made: false,
            handler_executed: false,
            event_generated: false,
            handler_kind: None,
            failure_stage: Some("route".to_string()),
            failure_reason: Some(reason),
            spawned_process: false,
            called_real_component: false,
            transport: None,
            process_exit_code: None,
            ledger_appended: false,
            ledger_path: None,
            ledger_record_count: 0,
        }
    }

    pub(super) fn with_ledger(
        mut output: KernelInvocationRuntimeOutput,
        ledger: &KernelInvocationLedger,
        ledger_record_count: usize,
        ledger_appended: bool,
    ) -> KernelInvocationRuntimeOutput {
        output.ledger_appended = ledger_appended;
        output.ledger_path = Some(ledger.path().display().to_string());
        output.ledger_record_count = ledger_record_count;
        if let Some(result) = output.result.as_mut() {
            result.ledger_appended = ledger_appended;
        }
        output
    }

    pub(super) fn ledger_failure(
        route: Option<KernelRouteRuntimeOutput>,
        event: Option<KernelEventEnvelope>,
        handler_executed: bool,
        event_generated: bool,
        handler_kind: Option<String>,
        route_decision_made: bool,
        error: String,
        ledger: &KernelInvocationLedger,
        ledger_record_count: usize,
    ) -> KernelInvocationRuntimeOutput {
        let transport = route
            .as_ref()
            .map(|route| route.transport.as_str().to_string());
        KernelInvocationRuntimeOutput {
            status: KernelInvocationStatus::Failed,
            route,
            event,
            result: None,
            route_decision_made,
            handler_executed,
            event_generated,
            handler_kind,
            failure_stage: Some("ledger_append".to_string()),
            failure_reason: Some(error),
            spawned_process: false,
            called_real_component: false,
            transport,
            process_exit_code: None,
            ledger_appended: false,
            ledger_path: Some(ledger.path().display().to_string()),
            ledger_record_count,
        }
    }

    pub(super) fn completed_ledger_failure(
        route: KernelRouteRuntimeOutput,
        event: KernelEventEnvelope,
        error: String,
        ledger: &KernelInvocationLedger,
        ledger_record_count: usize,
    ) -> KernelInvocationRuntimeOutput {
        Self::ledger_failure(
            Some(route),
            Some(event),
            true,
            true,
            Some("in_process".to_string()),
            true,
            format!("audit close failed after action happened: {error}"),
            ledger,
            ledger_record_count,
        )
    }

    pub(super) fn completed_ledger_failure_with_result(
        route: KernelRouteRuntimeOutput,
        event: KernelEventEnvelope,
        mut result: KernelInvocationResultEnvelope,
        handler_kind: Option<String>,
        spawned_process: bool,
        transport: Option<String>,
        process_exit_code: Option<i32>,
        error: String,
        ledger: &KernelInvocationLedger,
        ledger_record_count: usize,
    ) -> KernelInvocationRuntimeOutput {
        let reason = format!("audit close failed after action happened: {error}");
        result.status = KernelInvocationStatus::Failed;
        result.failure_stage = Some("ledger_append".to_string());
        result.failure_reason = Some(reason.clone());
        result.ledger_appended = false;
        if result.public_fields.get("write_applied").is_some() {
            result
                .public_fields
                .insert("audit_closed".to_string(), "false".to_string());
        }
        KernelInvocationRuntimeOutput {
            status: KernelInvocationStatus::Failed,
            route: Some(route),
            event: Some(event),
            result: Some(result),
            route_decision_made: true,
            handler_executed: true,
            event_generated: true,
            handler_kind,
            failure_stage: Some("ledger_append".to_string()),
            failure_reason: Some(reason),
            spawned_process,
            called_real_component: false,
            transport,
            process_exit_code,
            ledger_appended: false,
            ledger_path: Some(ledger.path().display().to_string()),
            ledger_record_count,
        }
    }
}
