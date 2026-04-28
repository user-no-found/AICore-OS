use crate::{
    KernelEventEnvelope, KernelEventPayload, KernelEventType, KernelInvocationEnvelope,
    KernelInvocationLedger, KernelInvocationLedgerRecord, KernelRouteRuntimeOutput, Visibility,
    format_contract,
};

use super::{
    KernelHandlerError, KernelHandlerResult, KernelInvocationResultEnvelope,
    KernelInvocationResultRoute, KernelInvocationRuntime, KernelInvocationRuntimeOutput,
    KernelInvocationStatus,
};

impl KernelInvocationRuntime {
    pub(super) fn invoke_with_ledger_in_process(
        &self,
        envelope: KernelInvocationEnvelope,
        route: KernelRouteRuntimeOutput,
        ledger: &KernelInvocationLedger,
        mut ledger_records: usize,
    ) -> KernelInvocationRuntimeOutput {
        let append = |record: KernelInvocationLedgerRecord,
                      ledger_records: &mut usize|
         -> Result<(), String> {
            ledger.append(&record)?;
            *ledger_records += 1;
            Ok(())
        };

        let Some(handler) = self.handlers.get(&envelope.operation) else {
            let reason = "missing handler for operation";
            if let Err(error) = append(
                KernelInvocationLedgerRecord::new("handler_lookup_failed", "failed", &envelope)
                    .with_route(&route)
                    .with_failure("handler_lookup", reason),
                &mut ledger_records,
            ) {
                return Self::ledger_failure(
                    Some(route),
                    None,
                    false,
                    false,
                    None,
                    true,
                    error,
                    ledger,
                    ledger_records,
                );
            }
            if let Err(error) = append(
                KernelInvocationLedgerRecord::new("invocation_failed", "failed", &envelope)
                    .with_route(&route)
                    .with_failure("handler_lookup", reason),
                &mut ledger_records,
            ) {
                return Self::ledger_failure(
                    Some(route),
                    None,
                    false,
                    false,
                    None,
                    true,
                    error,
                    ledger,
                    ledger_records,
                );
            }
            return Self::with_ledger(
                Self::missing_handler(&envelope, route),
                ledger,
                ledger_records,
                true,
            );
        };

        let result = match handler(&envelope, &route) {
            Ok(result) => result,
            Err(error) => {
                let reason = error.to_string();
                if let Err(ledger_error) = append(
                    KernelInvocationLedgerRecord::new("handler_failed", "failed", &envelope)
                        .with_route(&route)
                        .with_failure("handler_execute", &reason)
                        .with_handler(Some("in_process"), true, false, false, false),
                    &mut ledger_records,
                ) {
                    return Self::ledger_failure(
                        Some(route),
                        None,
                        true,
                        false,
                        Some("in_process".to_string()),
                        true,
                        ledger_error,
                        ledger,
                        ledger_records,
                    );
                }
                if let Err(ledger_error) = append(
                    KernelInvocationLedgerRecord::new("invocation_failed", "failed", &envelope)
                        .with_route(&route)
                        .with_failure("handler_execute", &reason)
                        .with_handler(Some("in_process"), true, false, false, false),
                    &mut ledger_records,
                ) {
                    return Self::ledger_failure(
                        Some(route),
                        None,
                        true,
                        false,
                        Some("in_process".to_string()),
                        true,
                        ledger_error,
                        ledger,
                        ledger_records,
                    );
                }
                return Self::with_ledger(
                    Self::handler_failure(&envelope, route, error),
                    ledger,
                    ledger_records,
                    true,
                );
            }
        };

        let (event, result) = Self::event_from_result(&envelope, &route, result);

        if let Err(error) = append(
            KernelInvocationLedgerRecord::new("handler_executed", "ok", &envelope)
                .with_route(&route)
                .with_handler(Some("in_process"), true, false, false, false),
            &mut ledger_records,
        ) {
            return Self::ledger_failure(
                Some(route),
                None,
                true,
                false,
                Some("in_process".to_string()),
                true,
                error,
                ledger,
                ledger_records,
            );
        }

        if let Err(error) = append(
            KernelInvocationLedgerRecord::new("event_generated", "ok", &envelope)
                .with_route(&route)
                .with_handler(Some("in_process"), true, true, false, false),
            &mut ledger_records,
        ) {
            return Self::ledger_failure(
                Some(route),
                Some(event),
                true,
                true,
                Some("in_process".to_string()),
                true,
                error,
                ledger,
                ledger_records,
            );
        }

        if let Err(error) = append(
            KernelInvocationLedgerRecord::new("invocation_completed", "ok", &envelope)
                .with_route(&route)
                .with_handler(Some("in_process"), true, true, false, false),
            &mut ledger_records,
        ) {
            return Self::completed_ledger_failure(route, event, error, ledger, ledger_records);
        }

        Self::with_ledger(
            Self::completed_from_event(route, event, Some(result)),
            ledger,
            ledger_records,
            true,
        )
    }

    pub(super) fn missing_handler(
        envelope: &KernelInvocationEnvelope,
        route: KernelRouteRuntimeOutput,
    ) -> KernelInvocationRuntimeOutput {
        let reason = "missing handler for operation";
        KernelInvocationRuntimeOutput {
            status: KernelInvocationStatus::Failed,
            route: Some(route.clone()),
            event: None,
            result: Some(Self::failure_result(
                envelope,
                Some(&route),
                "handler_lookup",
                reason,
                false,
                false,
                None,
            )),
            route_decision_made: true,
            handler_executed: false,
            event_generated: false,
            handler_kind: None,
            failure_stage: Some("handler_lookup".to_string()),
            failure_reason: Some(reason.to_string()),
            spawned_process: false,
            called_real_component: false,
            transport: Some(route.transport.as_str().to_string()),
            process_exit_code: None,
            ledger_appended: false,
            ledger_path: None,
            ledger_record_count: 0,
        }
    }

    pub(super) fn handler_failure(
        envelope: &KernelInvocationEnvelope,
        route: KernelRouteRuntimeOutput,
        error: KernelHandlerError,
    ) -> KernelInvocationRuntimeOutput {
        let reason = error.to_string();
        KernelInvocationRuntimeOutput {
            status: KernelInvocationStatus::Failed,
            route: Some(route.clone()),
            event: None,
            result: Some(Self::failure_result(
                envelope,
                Some(&route),
                "handler_execute",
                &reason,
                true,
                false,
                Some("in_process".to_string()),
            )),
            route_decision_made: true,
            handler_executed: true,
            event_generated: false,
            handler_kind: Some("in_process".to_string()),
            failure_stage: Some("handler_execute".to_string()),
            failure_reason: Some(reason),
            spawned_process: false,
            called_real_component: false,
            transport: Some(route.transport.as_str().to_string()),
            process_exit_code: None,
            ledger_appended: false,
            ledger_path: None,
            ledger_record_count: 0,
        }
    }

    pub(super) fn completed(
        envelope: KernelInvocationEnvelope,
        route: KernelRouteRuntimeOutput,
        result: KernelHandlerResult,
    ) -> KernelInvocationRuntimeOutput {
        let (event, result) = Self::event_from_result(&envelope, &route, result);
        Self::completed_from_event(route, event, Some(result))
    }

    pub(super) fn event_from_result(
        envelope: &KernelInvocationEnvelope,
        route: &KernelRouteRuntimeOutput,
        result: KernelHandlerResult,
    ) -> (KernelEventEnvelope, KernelInvocationResultEnvelope) {
        Self::event_from_result_with_handler_kind(envelope, route, result, "in_process")
    }

    pub(super) fn event_from_process_result(
        envelope: &KernelInvocationEnvelope,
        route: &KernelRouteRuntimeOutput,
        result: KernelHandlerResult,
    ) -> (KernelEventEnvelope, KernelInvocationResultEnvelope) {
        Self::event_from_result_with_handler_kind(envelope, route, result, "local_process")
    }

    fn event_from_result_with_handler_kind(
        envelope: &KernelInvocationEnvelope,
        route: &KernelRouteRuntimeOutput,
        result: KernelHandlerResult,
        handler_kind: &str,
    ) -> (KernelEventEnvelope, KernelInvocationResultEnvelope) {
        let mut event = KernelEventEnvelope::new(
            format!("event.{}", envelope.operation),
            KernelEventType::InvocationCompleted,
            envelope.instance_id.clone(),
            route.app_id.clone(),
            envelope.invocation_id.clone(),
            Visibility::User,
        );
        event.payload = KernelEventPayload::Summary(result.summary.clone());
        event.trace_context = envelope.trace_context.clone();
        let result = Self::success_result(envelope, route, result, handler_kind);
        (event, result)
    }

    pub(super) fn completed_from_event(
        route: KernelRouteRuntimeOutput,
        event: KernelEventEnvelope,
        result: Option<KernelInvocationResultEnvelope>,
    ) -> KernelInvocationRuntimeOutput {
        Self::completed_from_event_with_metadata(
            route,
            event,
            result,
            Some("in_process".to_string()),
            false,
            false,
            None,
            None,
        )
    }

    pub(super) fn completed_from_event_with_metadata(
        route: KernelRouteRuntimeOutput,
        event: KernelEventEnvelope,
        result: Option<KernelInvocationResultEnvelope>,
        handler_kind: Option<String>,
        spawned_process: bool,
        called_real_component: bool,
        transport: Option<String>,
        process_exit_code: Option<i32>,
    ) -> KernelInvocationRuntimeOutput {
        KernelInvocationRuntimeOutput {
            status: KernelInvocationStatus::Completed,
            route: Some(route),
            event: Some(event),
            result,
            route_decision_made: true,
            handler_executed: true,
            event_generated: true,
            handler_kind,
            failure_stage: None,
            failure_reason: None,
            spawned_process,
            called_real_component,
            transport,
            process_exit_code,
            ledger_appended: false,
            ledger_path: None,
            ledger_record_count: 0,
        }
    }

    fn success_result(
        envelope: &KernelInvocationEnvelope,
        route: &KernelRouteRuntimeOutput,
        result: KernelHandlerResult,
        handler_kind: &str,
    ) -> KernelInvocationResultEnvelope {
        KernelInvocationResultEnvelope {
            invocation_id: envelope.invocation_id.clone(),
            trace_id: envelope.trace_context.trace_id.clone(),
            operation: envelope.operation.clone(),
            status: KernelInvocationStatus::Completed,
            route: Some(Self::result_route(route)),
            handler_kind: Some(handler_kind.to_string()),
            result_kind: result.result_kind,
            summary: result.summary,
            public_fields: result.public_fields,
            failure_stage: None,
            failure_reason: None,
            handler_executed: true,
            event_generated: true,
            ledger_appended: false,
        }
    }

    pub(super) fn failure_result(
        envelope: &KernelInvocationEnvelope,
        route: Option<&KernelRouteRuntimeOutput>,
        failure_stage: &str,
        failure_reason: &str,
        handler_executed: bool,
        event_generated: bool,
        handler_kind: Option<String>,
    ) -> KernelInvocationResultEnvelope {
        KernelInvocationResultEnvelope {
            invocation_id: envelope.invocation_id.clone(),
            trace_id: envelope.trace_context.trace_id.clone(),
            operation: envelope.operation.clone(),
            status: KernelInvocationStatus::Failed,
            route: route.map(Self::result_route),
            handler_kind,
            result_kind: None,
            summary: failure_reason.to_string(),
            public_fields: std::collections::BTreeMap::new(),
            failure_stage: Some(failure_stage.to_string()),
            failure_reason: Some(failure_reason.to_string()),
            handler_executed,
            event_generated,
            ledger_appended: false,
        }
    }

    fn result_route(route: &KernelRouteRuntimeOutput) -> KernelInvocationResultRoute {
        KernelInvocationResultRoute {
            component_id: route.component_id.clone(),
            app_id: route.app_id.clone(),
            capability_id: route.capability_id.clone(),
            contract_version: format_contract(&route.contract_version),
        }
    }
}
