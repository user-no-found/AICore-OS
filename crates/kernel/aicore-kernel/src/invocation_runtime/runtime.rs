use crate::{
    ComponentInvocationMode, InstalledManifestRegistry, KernelInvocationEnvelope,
    KernelInvocationLedger, KernelInvocationLedgerRecord, KernelRouteRuntime,
    KernelRouteRuntimeInput,
};

use super::{KernelHandlerRegistry, KernelInvocationRuntimeOutput};

#[derive(Clone)]
pub struct KernelInvocationRuntime {
    pub(super) route_runtime: KernelRouteRuntime,
    pub(super) handlers: KernelHandlerRegistry,
}

impl KernelInvocationRuntime {
    pub fn new(registry: InstalledManifestRegistry, handlers: KernelHandlerRegistry) -> Self {
        Self {
            route_runtime: KernelRouteRuntime::from_registry(registry),
            handlers,
        }
    }

    pub fn invoke(&self, envelope: KernelInvocationEnvelope) -> KernelInvocationRuntimeOutput {
        let route = match self.route_runtime.route(
            KernelRouteRuntimeInput::new(envelope.operation.clone())
                .with_instance_id(envelope.instance_id.clone()),
        ) {
            Ok(route) => route,
            Err(error) => return Self::route_failure(&envelope, error),
        };

        if route.invocation_mode == ComponentInvocationMode::LocalProcess {
            return match Self::invoke_local_process(&envelope, &route) {
                Ok(success) => {
                    let (event, result) =
                        Self::event_from_process_result(&envelope, &route, success.result);
                    Self::completed_from_event_with_metadata(
                        route,
                        event,
                        Some(result),
                        Some("local_process".to_string()),
                        true,
                        false,
                        Some("stdio_jsonl".to_string()),
                        success.exit_code,
                    )
                }
                Err(error) => Self::process_failure(&envelope, route, error),
            };
        }

        let Some(handler) = self.handlers.get(&envelope.operation) else {
            return Self::missing_handler(&envelope, route);
        };

        match handler(&envelope, &route) {
            Ok(result) => Self::completed(envelope, route, result),
            Err(error) => Self::handler_failure(&envelope, route, error),
        }
    }

    pub fn invoke_with_ledger(
        &self,
        envelope: KernelInvocationEnvelope,
        ledger: &KernelInvocationLedger,
    ) -> KernelInvocationRuntimeOutput {
        let mut ledger_records = 0usize;
        let append = |record: KernelInvocationLedgerRecord,
                      ledger_records: &mut usize|
         -> Result<(), String> {
            ledger.append(&record)?;
            *ledger_records += 1;
            Ok(())
        };

        if let Err(error) = append(
            KernelInvocationLedgerRecord::new("accepted", "ok", &envelope),
            &mut ledger_records,
        ) {
            return Self::ledger_failure(
                None,
                None,
                false,
                false,
                None,
                false,
                error,
                ledger,
                ledger_records,
            );
        }

        let route = match self.route_runtime.route(
            KernelRouteRuntimeInput::new(envelope.operation.clone())
                .with_instance_id(envelope.instance_id.clone()),
        ) {
            Ok(route) => route,
            Err(error) => {
                let reason = error.to_string();
                if let Err(ledger_error) = append(
                    KernelInvocationLedgerRecord::new("route_failed", "failed", &envelope)
                        .with_failure("route", &reason),
                    &mut ledger_records,
                ) {
                    return Self::ledger_failure(
                        None,
                        None,
                        false,
                        false,
                        None,
                        false,
                        ledger_error,
                        ledger,
                        ledger_records,
                    );
                }
                if let Err(ledger_error) = append(
                    KernelInvocationLedgerRecord::new("invocation_failed", "failed", &envelope)
                        .with_failure("route", &reason),
                    &mut ledger_records,
                ) {
                    return Self::ledger_failure(
                        None,
                        None,
                        false,
                        false,
                        None,
                        false,
                        ledger_error,
                        ledger,
                        ledger_records,
                    );
                }
                return Self::with_ledger(
                    Self::route_failure(&envelope, error),
                    ledger,
                    ledger_records,
                    true,
                );
            }
        };

        if let Err(error) = append(
            KernelInvocationLedgerRecord::new("route_decision_made", "ok", &envelope)
                .with_route(&route),
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

        if route.invocation_mode == ComponentInvocationMode::LocalProcess {
            return Self::invoke_with_ledger_local_process(envelope, route, ledger, ledger_records);
        }

        self.invoke_with_ledger_in_process(envelope, route, ledger, ledger_records)
    }
}
