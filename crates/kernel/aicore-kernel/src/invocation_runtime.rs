use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

use crate::{
    InstalledManifestRegistry, KernelEventEnvelope, KernelEventPayload, KernelEventType,
    KernelInvocationEnvelope, KernelInvocationLedger, KernelInvocationLedgerRecord,
    KernelRouteRuntime, KernelRouteRuntimeError, KernelRouteRuntimeInput, KernelRouteRuntimeOutput,
    Visibility, format_contract,
};

pub type KernelHandlerFn = Arc<
    dyn Fn(
            &KernelInvocationEnvelope,
            &KernelRouteRuntimeOutput,
        ) -> Result<KernelHandlerResult, KernelHandlerError>
        + Send
        + Sync,
>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelHandlerResult {
    pub summary: String,
    pub result_kind: Option<String>,
    pub public_fields: BTreeMap<String, String>,
}

impl KernelHandlerResult {
    pub fn summary(summary: impl Into<String>) -> Self {
        Self {
            summary: summary.into(),
            result_kind: Some("summary".to_string()),
            public_fields: BTreeMap::new(),
        }
    }

    pub fn structured(
        result_kind: impl Into<String>,
        public_fields: BTreeMap<String, String>,
        summary: impl Into<String>,
    ) -> Self {
        Self {
            summary: summary.into(),
            result_kind: Some(result_kind.into()),
            public_fields,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelHandlerError {
    pub message: String,
}

impl KernelHandlerError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for KernelHandlerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.message)
    }
}

impl std::error::Error for KernelHandlerError {}

#[derive(Clone)]
pub struct KernelHandlerRegistry {
    handlers: BTreeMap<String, KernelHandlerFn>,
}

impl KernelHandlerRegistry {
    pub fn new() -> Self {
        Self {
            handlers: BTreeMap::new(),
        }
    }

    pub fn with_handler<F>(mut self, operation: impl Into<String>, handler: F) -> Self
    where
        F: Fn(
                &KernelInvocationEnvelope,
                &KernelRouteRuntimeOutput,
            ) -> Result<KernelHandlerResult, KernelHandlerError>
            + Send
            + Sync
            + 'static,
    {
        self.register(operation, handler);
        self
    }

    pub fn register<F>(&mut self, operation: impl Into<String>, handler: F)
    where
        F: Fn(
                &KernelInvocationEnvelope,
                &KernelRouteRuntimeOutput,
            ) -> Result<KernelHandlerResult, KernelHandlerError>
            + Send
            + Sync
            + 'static,
    {
        self.handlers.insert(operation.into(), Arc::new(handler));
    }

    pub fn get(&self, operation: &str) -> Option<KernelHandlerFn> {
        self.handlers.get(operation).cloned()
    }
}

impl Default for KernelHandlerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

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
    pub ledger_appended: bool,
    pub ledger_path: Option<String>,
    pub ledger_record_count: usize,
}

#[derive(Clone)]
pub struct KernelInvocationRuntime {
    route_runtime: KernelRouteRuntime,
    handlers: KernelHandlerRegistry,
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

    fn route_failure(
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
            ledger_appended: false,
            ledger_path: None,
            ledger_record_count: 0,
        }
    }

    fn missing_handler(
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
            ledger_appended: false,
            ledger_path: None,
            ledger_record_count: 0,
        }
    }

    fn handler_failure(
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
            ledger_appended: false,
            ledger_path: None,
            ledger_record_count: 0,
        }
    }

    fn completed(
        envelope: KernelInvocationEnvelope,
        route: KernelRouteRuntimeOutput,
        result: KernelHandlerResult,
    ) -> KernelInvocationRuntimeOutput {
        let (event, result) = Self::event_from_result(&envelope, &route, result);
        Self::completed_from_event(route, event, Some(result))
    }

    fn event_from_result(
        envelope: &KernelInvocationEnvelope,
        route: &KernelRouteRuntimeOutput,
        result: KernelHandlerResult,
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
        let result = Self::success_result(envelope, route, result);
        (event, result)
    }

    fn completed_from_event(
        route: KernelRouteRuntimeOutput,
        event: KernelEventEnvelope,
        result: Option<KernelInvocationResultEnvelope>,
    ) -> KernelInvocationRuntimeOutput {
        KernelInvocationRuntimeOutput {
            status: KernelInvocationStatus::Completed,
            route: Some(route),
            event: Some(event),
            result,
            route_decision_made: true,
            handler_executed: true,
            event_generated: true,
            handler_kind: Some("in_process".to_string()),
            failure_stage: None,
            failure_reason: None,
            spawned_process: false,
            called_real_component: false,
            ledger_appended: false,
            ledger_path: None,
            ledger_record_count: 0,
        }
    }

    fn success_result(
        envelope: &KernelInvocationEnvelope,
        route: &KernelRouteRuntimeOutput,
        result: KernelHandlerResult,
    ) -> KernelInvocationResultEnvelope {
        KernelInvocationResultEnvelope {
            invocation_id: envelope.invocation_id.clone(),
            trace_id: envelope.trace_context.trace_id.clone(),
            operation: envelope.operation.clone(),
            status: KernelInvocationStatus::Completed,
            route: Some(Self::result_route(route)),
            handler_kind: Some("in_process".to_string()),
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

    fn failure_result(
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
            public_fields: BTreeMap::new(),
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

    fn with_ledger(
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

    fn ledger_failure(
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
            ledger_appended: false,
            ledger_path: Some(ledger.path().display().to_string()),
            ledger_record_count,
        }
    }

    fn completed_ledger_failure(
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
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use aicore_foundation::AicoreLayout;

    use crate::{
        InstalledManifestRegistry, KernelEventPayload, KernelEventType, KernelHandlerError,
        KernelHandlerRegistry, KernelHandlerResult, KernelInvocationEnvelope,
        KernelInvocationLedger, KernelInvocationRuntime, KernelInvocationStatus, KernelPayload,
        runtime_status_handler_for_layout,
    };

    #[test]
    fn invocation_runtime_routes_before_handler_execution() {
        let registry = registry_with_manifest(&[("memory.search", "memory.search")]);
        let runtime = KernelInvocationRuntime::new(
            InstalledManifestRegistry::load_from_dir(&registry).expect("registry"),
            KernelHandlerRegistry::new().with_handler("memory.search", smoke_handler),
        );

        let output = runtime.invoke(envelope("memory.search"));

        assert_eq!(output.status, KernelInvocationStatus::Completed);
        assert_eq!(
            output
                .route
                .as_ref()
                .expect("route should exist")
                .component_id,
            "aicore-cli"
        );
        assert!(output.route_decision_made);
        assert!(output.handler_executed);
    }

    #[test]
    fn invocation_runtime_executes_registered_in_process_handler() {
        let registry = registry_with_manifest(&[("memory.search", "memory.search")]);
        let runtime = KernelInvocationRuntime::new(
            InstalledManifestRegistry::load_from_dir(&registry).expect("registry"),
            KernelHandlerRegistry::new().with_handler("memory.search", smoke_handler),
        );

        let output = runtime.invoke(envelope("memory.search"));

        assert_eq!(output.status, KernelInvocationStatus::Completed);
        assert!(output.handler_executed);
        assert_eq!(output.handler_kind.as_deref(), Some("in_process"));
        assert_eq!(output.failure_reason, None);
    }

    #[test]
    fn invocation_runtime_returns_event_envelope() {
        let registry = registry_with_manifest(&[("memory.search", "memory.search")]);
        let runtime = KernelInvocationRuntime::new(
            InstalledManifestRegistry::load_from_dir(&registry).expect("registry"),
            KernelHandlerRegistry::new().with_handler("memory.search", smoke_handler),
        );

        let envelope = envelope("memory.search");
        let expected_invocation_id = envelope.invocation_id.clone();
        let output = runtime.invoke(envelope);
        let event = output.event.expect("event should be generated");

        assert_eq!(event.event_type, KernelEventType::InvocationCompleted);
        assert_eq!(event.app_id, "aicore-cli");
        assert_eq!(event.instance_id, "global-main");
        assert_eq!(event.invocation_id, expected_invocation_id);
        assert_eq!(event.trace_context.trace_id, "trace.default");
        assert_eq!(
            event.payload,
            KernelEventPayload::Summary("smoke handled memory.search".to_string())
        );
    }

    #[test]
    fn kernel_invocation_runtime_status_returns_structured_result() {
        let registry = registry_with_manifest(&[("runtime.status", "runtime.status")]);
        let runtime = KernelInvocationRuntime::new(
            InstalledManifestRegistry::load_from_dir(&registry).expect("registry"),
            KernelHandlerRegistry::new().with_handler("runtime.status", structured_status_handler),
        );

        let output = runtime.invoke(envelope("runtime.status"));
        let result = output.result.expect("result envelope should exist");

        assert_eq!(result.result_kind.as_deref(), Some("runtime.status"));
        assert_eq!(
            result.public_fields.get("foundation_installed"),
            Some(&"yes".to_string())
        );
        assert_eq!(
            result.public_fields.get("manifest_count"),
            Some(&"3".to_string())
        );
    }

    #[test]
    fn runtime_status_handler_builds_structured_result_from_supplied_layout() {
        let home = temp_dir("runtime-status-handler-home");
        seed_runtime_status_layout(&home, 2);
        let layout = AicoreLayout::new(&home);
        let registry = registry_with_manifest(&[("runtime.status", "runtime.status")]);
        let runtime = KernelInvocationRuntime::new(
            InstalledManifestRegistry::load_from_dir(&registry).expect("registry"),
            KernelHandlerRegistry::new()
                .with_handler("runtime.status", runtime_status_handler_for_layout(layout)),
        );

        let output = runtime.invoke(envelope("runtime.status"));
        let result = output.result.expect("runtime status result envelope");

        assert_eq!(result.result_kind.as_deref(), Some("runtime.status"));
        assert_eq!(
            result.public_fields.get("global_root"),
            Some(&home.join(".aicore").display().to_string())
        );
        assert_eq!(
            result.public_fields.get("foundation_installed"),
            Some(&"yes".to_string())
        );
        assert_eq!(
            result.public_fields.get("kernel_installed"),
            Some(&"yes".to_string())
        );
        assert_eq!(
            result.public_fields.get("manifest_count"),
            Some(&"1".to_string())
        );
        assert_eq!(
            result.public_fields.get("capability_count"),
            Some(&"2".to_string())
        );
    }

    #[test]
    fn runtime_status_handler_uses_supplied_layout_not_home() {
        let home = temp_dir("runtime-status-handler-layout");
        seed_runtime_status_layout(&home, 1);
        let layout = AicoreLayout::new(&home);
        let registry = registry_with_manifest(&[("runtime.status", "runtime.status")]);
        let runtime = KernelInvocationRuntime::new(
            InstalledManifestRegistry::load_from_dir(&registry).expect("registry"),
            KernelHandlerRegistry::new()
                .with_handler("runtime.status", runtime_status_handler_for_layout(layout)),
        );

        let output = runtime.invoke(envelope("runtime.status"));

        let result = output.result.expect("runtime status result envelope");
        assert_eq!(
            result.public_fields.get("global_root"),
            Some(&home.join(".aicore").display().to_string())
        );
        assert_ne!(
            result.public_fields.get("global_root"),
            Some(
                &AicoreLayout::from_system_home()
                    .state_root
                    .display()
                    .to_string()
            )
        );
    }

    #[test]
    fn kernel_invocation_output_contains_result_envelope() {
        let registry = registry_with_manifest(&[("runtime.status", "runtime.status")]);
        let runtime = KernelInvocationRuntime::new(
            InstalledManifestRegistry::load_from_dir(&registry).expect("registry"),
            KernelHandlerRegistry::new().with_handler("runtime.status", structured_status_handler),
        );

        let output = runtime.invoke(envelope("runtime.status"));
        let result = output.result.expect("result envelope should exist");

        assert_eq!(result.operation, "runtime.status");
        assert_eq!(result.status, KernelInvocationStatus::Completed);
        assert_eq!(result.handler_kind.as_deref(), Some("in_process"));
        assert!(result.handler_executed);
        assert!(result.event_generated);
    }

    #[test]
    fn kernel_invocation_result_preserves_invocation_id() {
        let registry = registry_with_manifest(&[("runtime.status", "runtime.status")]);
        let runtime = KernelInvocationRuntime::new(
            InstalledManifestRegistry::load_from_dir(&registry).expect("registry"),
            KernelHandlerRegistry::new().with_handler("runtime.status", structured_status_handler),
        );
        let envelope = envelope("runtime.status");
        let expected_invocation_id = envelope.invocation_id.clone();

        let output = runtime.invoke(envelope);

        assert_eq!(
            output.result.expect("result envelope").invocation_id,
            expected_invocation_id
        );
    }

    #[test]
    fn kernel_invocation_result_preserves_route_metadata() {
        let registry = registry_with_manifest(&[("runtime.status", "runtime.status")]);
        let runtime = KernelInvocationRuntime::new(
            InstalledManifestRegistry::load_from_dir(&registry).expect("registry"),
            KernelHandlerRegistry::new().with_handler("runtime.status", structured_status_handler),
        );

        let output = runtime.invoke(envelope("runtime.status"));
        let result = output.result.expect("result envelope");
        let route = result.route.expect("route metadata");

        assert_eq!(route.component_id, "aicore-cli");
        assert_eq!(route.app_id, "aicore-cli");
        assert_eq!(route.capability_id, "runtime.status");
        assert_eq!(route.contract_version, "kernel.app.v1");
    }

    #[test]
    fn kernel_invocation_result_summary_is_derived_from_structured_result() {
        let registry = registry_with_manifest(&[("runtime.status", "runtime.status")]);
        let runtime = KernelInvocationRuntime::new(
            InstalledManifestRegistry::load_from_dir(&registry).expect("registry"),
            KernelHandlerRegistry::new().with_handler("runtime.status", structured_status_handler),
        );

        let output = runtime.invoke(envelope("runtime.status"));
        let result = output.result.expect("result envelope");

        assert_eq!(
            result.summary,
            "foundation_installed=yes | kernel_installed=yes | manifest_count=3"
        );
        assert_eq!(
            result.public_fields.get("kernel_installed"),
            Some(&"yes".to_string())
        );
    }

    #[test]
    fn invocation_runtime_missing_operation_does_not_execute_handler() {
        let registry = registry_with_manifest(&[("memory.search", "memory.search")]);
        let runtime = KernelInvocationRuntime::new(
            InstalledManifestRegistry::load_from_dir(&registry).expect("registry"),
            KernelHandlerRegistry::new().with_handler("unknown.operation", smoke_handler),
        );

        let output = runtime.invoke(envelope("unknown.operation"));

        assert_eq!(output.status, KernelInvocationStatus::Failed);
        assert!(!output.route_decision_made);
        assert!(!output.handler_executed);
        assert!(
            output
                .failure_reason
                .as_deref()
                .expect("failure reason")
                .contains("missing capability")
        );
    }

    #[test]
    fn invocation_runtime_missing_handler_returns_structured_failure() {
        let registry = registry_with_manifest(&[("memory.search", "memory.search")]);
        let runtime = KernelInvocationRuntime::new(
            InstalledManifestRegistry::load_from_dir(&registry).expect("registry"),
            KernelHandlerRegistry::new(),
        );

        let output = runtime.invoke(envelope("memory.search"));

        assert_eq!(output.status, KernelInvocationStatus::Failed);
        assert!(output.route_decision_made);
        assert!(!output.handler_executed);
        assert_eq!(output.failure_stage.as_deref(), Some("handler_lookup"));
        assert!(
            output
                .failure_reason
                .as_deref()
                .expect("failure reason")
                .contains("missing handler")
        );
    }

    #[test]
    fn invocation_runtime_ambiguous_route_does_not_execute_handler() {
        let root = temp_dir("ambiguous-route");
        write_manifest(
            &root,
            "aicore-cli.toml",
            "aicore-cli",
            &[("memory.search", "memory.search")],
        );
        write_manifest(
            &root,
            "aicore-memory.toml",
            "aicore-memory",
            &[("memory.search", "memory.search")],
        );
        let runtime = KernelInvocationRuntime::new(
            InstalledManifestRegistry::load_from_dir(&root).expect("registry"),
            KernelHandlerRegistry::new().with_handler("memory.search", smoke_handler),
        );

        let output = runtime.invoke(envelope("memory.search"));

        assert_eq!(output.status, KernelInvocationStatus::Failed);
        assert!(!output.handler_executed);
        assert!(
            output
                .failure_reason
                .as_deref()
                .expect("failure reason")
                .contains("ambiguous route")
        );
    }

    #[test]
    fn invocation_runtime_contract_mismatch_does_not_execute_handler() {
        let root = temp_dir("contract-mismatch");
        write_manifest_with_contract(
            &root,
            "aicore-cli.toml",
            "aicore-cli",
            "kernel.app.v2",
            &[("memory.search", "memory.search")],
        );
        let runtime = KernelInvocationRuntime::new(
            InstalledManifestRegistry::load_from_dir(&root).expect("registry"),
            KernelHandlerRegistry::new().with_handler("memory.search", smoke_handler),
        );

        let output = runtime.invoke(envelope("memory.search"));

        assert_eq!(output.status, KernelInvocationStatus::Failed);
        assert!(!output.handler_executed);
        assert!(
            output
                .failure_reason
                .as_deref()
                .expect("failure reason")
                .contains("contract version mismatch")
        );
    }

    #[test]
    fn invocation_runtime_handler_failure_returns_structured_failure() {
        let registry = registry_with_manifest(&[("memory.search", "memory.search")]);
        let runtime = KernelInvocationRuntime::new(
            InstalledManifestRegistry::load_from_dir(&registry).expect("registry"),
            KernelHandlerRegistry::new().with_handler("memory.search", failing_handler),
        );

        let output = runtime.invoke(envelope("memory.search"));

        assert_eq!(output.status, KernelInvocationStatus::Failed);
        assert!(output.route_decision_made);
        assert!(output.handler_executed);
        assert_eq!(output.failure_stage.as_deref(), Some("handler_execute"));
        assert!(
            output
                .failure_reason
                .as_deref()
                .expect("failure reason")
                .contains("smoke handler failed")
        );
    }

    #[test]
    fn invocation_runtime_output_marks_handler_executed() {
        let registry = registry_with_manifest(&[("memory.search", "memory.search")]);
        let runtime = KernelInvocationRuntime::new(
            InstalledManifestRegistry::load_from_dir(&registry).expect("registry"),
            KernelHandlerRegistry::new().with_handler("memory.search", smoke_handler),
        );

        let output = runtime.invoke(envelope("memory.search"));

        assert!(output.handler_executed);
        assert!(output.event_generated);
        assert!(!output.spawned_process);
        assert!(!output.called_real_component);
    }

    #[test]
    fn invocation_ledger_appends_accepted_and_completed_records() {
        let registry = registry_with_manifest(&[("memory.search", "memory.search")]);
        let ledger_path = temp_dir("ledger-success").join("invocation-ledger.jsonl");
        let ledger = KernelInvocationLedger::new(&ledger_path);
        let runtime = KernelInvocationRuntime::new(
            InstalledManifestRegistry::load_from_dir(&registry).expect("registry"),
            KernelHandlerRegistry::new().with_handler("memory.search", smoke_handler),
        );

        let output = runtime.invoke_with_ledger(envelope("memory.search"), &ledger);
        let records = read_ledger_records(&ledger_path);

        assert_eq!(output.status, KernelInvocationStatus::Completed);
        assert!(output.ledger_appended);
        assert_eq!(output.ledger_record_count, 5);
        assert_eq!(
            ledger_stages(&records),
            vec![
                "accepted",
                "route_decision_made",
                "handler_executed",
                "event_generated",
                "invocation_completed",
            ]
        );
    }

    #[test]
    fn invocation_ledger_appends_route_failure_record() {
        let registry = registry_with_manifest(&[("memory.search", "memory.search")]);
        let ledger_path = temp_dir("ledger-route-failure").join("invocation-ledger.jsonl");
        let ledger = KernelInvocationLedger::new(&ledger_path);
        let runtime = KernelInvocationRuntime::new(
            InstalledManifestRegistry::load_from_dir(&registry).expect("registry"),
            KernelHandlerRegistry::new().with_handler("memory.search", smoke_handler),
        );

        let output = runtime.invoke_with_ledger(envelope("unknown.operation"), &ledger);
        let records = read_ledger_records(&ledger_path);

        assert_eq!(output.status, KernelInvocationStatus::Failed);
        assert!(!output.handler_executed);
        assert_eq!(
            ledger_stages(&records),
            vec!["accepted", "route_failed", "invocation_failed"]
        );
        assert!(
            records
                .iter()
                .any(|record| record.contains("missing capability"))
        );
    }

    #[test]
    fn invocation_ledger_appends_missing_handler_failure_record() {
        let registry = registry_with_manifest(&[("provider.smoke", "provider.smoke")]);
        let ledger_path = temp_dir("ledger-missing-handler").join("invocation-ledger.jsonl");
        let ledger = KernelInvocationLedger::new(&ledger_path);
        let runtime = KernelInvocationRuntime::new(
            InstalledManifestRegistry::load_from_dir(&registry).expect("registry"),
            KernelHandlerRegistry::new(),
        );

        let output = runtime.invoke_with_ledger(envelope("provider.smoke"), &ledger);
        let records = read_ledger_records(&ledger_path);

        assert_eq!(output.status, KernelInvocationStatus::Failed);
        assert_eq!(
            ledger_stages(&records),
            vec![
                "accepted",
                "route_decision_made",
                "handler_lookup_failed",
                "invocation_failed",
            ]
        );
    }

    #[test]
    fn invocation_ledger_appends_handler_failure_record() {
        let registry = registry_with_manifest(&[("memory.search", "memory.search")]);
        let ledger_path = temp_dir("ledger-handler-failure").join("invocation-ledger.jsonl");
        let ledger = KernelInvocationLedger::new(&ledger_path);
        let runtime = KernelInvocationRuntime::new(
            InstalledManifestRegistry::load_from_dir(&registry).expect("registry"),
            KernelHandlerRegistry::new().with_handler("memory.search", failing_handler),
        );

        let output = runtime.invoke_with_ledger(envelope("memory.search"), &ledger);
        let records = read_ledger_records(&ledger_path);

        assert_eq!(output.status, KernelInvocationStatus::Failed);
        assert!(output.handler_executed);
        assert_eq!(
            ledger_stages(&records),
            vec![
                "accepted",
                "route_decision_made",
                "handler_failed",
                "invocation_failed",
            ]
        );
    }

    #[test]
    fn invocation_ledger_records_trace_and_invocation_ids() {
        let registry = registry_with_manifest(&[("memory.search", "memory.search")]);
        let ledger_path = temp_dir("ledger-trace").join("invocation-ledger.jsonl");
        let ledger = KernelInvocationLedger::new(&ledger_path);
        let runtime = KernelInvocationRuntime::new(
            InstalledManifestRegistry::load_from_dir(&registry).expect("registry"),
            KernelHandlerRegistry::new().with_handler("memory.search", smoke_handler),
        );

        runtime.invoke_with_ledger(envelope("memory.search"), &ledger);
        let joined = read_ledger_records(&ledger_path).join("\n");

        assert!(joined.contains("\"trace_id\":\"trace.default\""));
        assert!(joined.contains("\"invocation_id\":\"invoke."));
        assert!(!joined.contains("\"invocation_id\":\"invoke.memory.search\""));
        assert!(joined.contains("\"instance_id\":\"global-main\""));
    }

    #[test]
    fn invocation_ledger_uses_same_invocation_id_for_one_invocation() {
        let registry = registry_with_manifest(&[("memory.search", "memory.search")]);
        let ledger_path = temp_dir("ledger-same-invocation-id").join("invocation-ledger.jsonl");
        let ledger = KernelInvocationLedger::new(&ledger_path);
        let runtime = KernelInvocationRuntime::new(
            InstalledManifestRegistry::load_from_dir(&registry).expect("registry"),
            KernelHandlerRegistry::new().with_handler("memory.search", smoke_handler),
        );

        runtime.invoke_with_ledger(envelope("memory.search"), &ledger);
        let ids = ledger_invocation_ids(&read_ledger_records(&ledger_path));

        assert_eq!(ids.len(), 5);
        assert!(ids.iter().all(|id| id == &ids[0]));
        assert_ne!(ids[0], "invoke.memory.search");
    }

    #[test]
    fn invocation_ledger_uses_distinct_invocation_id_for_repeated_same_operation() {
        let registry = registry_with_manifest(&[("memory.search", "memory.search")]);
        let ledger_path = temp_dir("ledger-distinct-invocation-id").join("invocation-ledger.jsonl");
        let ledger = KernelInvocationLedger::new(&ledger_path);
        let runtime = KernelInvocationRuntime::new(
            InstalledManifestRegistry::load_from_dir(&registry).expect("registry"),
            KernelHandlerRegistry::new().with_handler("memory.search", smoke_handler),
        );

        runtime.invoke_with_ledger(envelope("memory.search"), &ledger);
        runtime.invoke_with_ledger(envelope("memory.search"), &ledger);
        let ids = ledger_invocation_ids(&read_ledger_records(&ledger_path));

        assert_eq!(ids.len(), 10);
        assert_eq!(ids[0], ids[4]);
        assert_eq!(ids[5], ids[9]);
        assert_ne!(ids[0], ids[5]);
    }

    #[test]
    fn invocation_event_uses_same_invocation_id_as_envelope() {
        let registry = registry_with_manifest(&[("memory.search", "memory.search")]);
        let runtime = KernelInvocationRuntime::new(
            InstalledManifestRegistry::load_from_dir(&registry).expect("registry"),
            KernelHandlerRegistry::new().with_handler("memory.search", smoke_handler),
        );
        let envelope = envelope("memory.search");
        let expected_invocation_id = envelope.invocation_id.clone();

        let output = runtime.invoke(envelope);

        assert_eq!(
            output
                .event
                .expect("event should be generated")
                .invocation_id,
            expected_invocation_id
        );
    }

    #[test]
    fn invocation_route_failure_records_share_same_invocation_id() {
        let registry = registry_with_manifest(&[("memory.search", "memory.search")]);
        let ledger_path = temp_dir("ledger-route-failure-id").join("invocation-ledger.jsonl");
        let ledger = KernelInvocationLedger::new(&ledger_path);
        let runtime = KernelInvocationRuntime::new(
            InstalledManifestRegistry::load_from_dir(&registry).expect("registry"),
            KernelHandlerRegistry::new().with_handler("memory.search", smoke_handler),
        );

        runtime.invoke_with_ledger(envelope("unknown.operation"), &ledger);
        let ids = ledger_invocation_ids(&read_ledger_records(&ledger_path));

        assert_eq!(ids.len(), 3);
        assert!(ids.iter().all(|id| id == &ids[0]));
        assert_ne!(ids[0], "invoke.unknown.operation");
    }

    #[test]
    fn invocation_missing_handler_records_share_same_invocation_id() {
        let registry = registry_with_manifest(&[("provider.smoke", "provider.smoke")]);
        let ledger_path = temp_dir("ledger-missing-handler-id").join("invocation-ledger.jsonl");
        let ledger = KernelInvocationLedger::new(&ledger_path);
        let runtime = KernelInvocationRuntime::new(
            InstalledManifestRegistry::load_from_dir(&registry).expect("registry"),
            KernelHandlerRegistry::new(),
        );

        runtime.invoke_with_ledger(envelope("provider.smoke"), &ledger);
        let ids = ledger_invocation_ids(&read_ledger_records(&ledger_path));

        assert_eq!(ids.len(), 4);
        assert!(ids.iter().all(|id| id == &ids[0]));
        assert_ne!(ids[0], "invoke.provider.smoke");
    }

    #[test]
    fn invocation_handler_failure_records_share_same_invocation_id() {
        let registry = registry_with_manifest(&[("memory.search", "memory.search")]);
        let ledger_path = temp_dir("ledger-handler-failure-id").join("invocation-ledger.jsonl");
        let ledger = KernelInvocationLedger::new(&ledger_path);
        let runtime = KernelInvocationRuntime::new(
            InstalledManifestRegistry::load_from_dir(&registry).expect("registry"),
            KernelHandlerRegistry::new().with_handler("memory.search", failing_handler),
        );

        runtime.invoke_with_ledger(envelope("memory.search"), &ledger);
        let ids = ledger_invocation_ids(&read_ledger_records(&ledger_path));

        assert_eq!(ids.len(), 4);
        assert!(ids.iter().all(|id| id == &ids[0]));
        assert_ne!(ids[0], "invoke.memory.search");
    }

    #[test]
    fn invocation_ledger_is_json_lines() {
        let registry = registry_with_manifest(&[("memory.search", "memory.search")]);
        let ledger_path = temp_dir("ledger-jsonl").join("invocation-ledger.jsonl");
        let ledger = KernelInvocationLedger::new(&ledger_path);
        let runtime = KernelInvocationRuntime::new(
            InstalledManifestRegistry::load_from_dir(&registry).expect("registry"),
            KernelHandlerRegistry::new().with_handler("memory.search", smoke_handler),
        );

        runtime.invoke_with_ledger(envelope("memory.search"), &ledger);

        for record in read_ledger_records(&ledger_path) {
            assert!(record.starts_with('{'));
            assert!(record.ends_with('}'));
            assert!(record.contains("\"schema_version\":\"aicore.kernel.invocation_ledger.v1\""));
        }
    }

    #[test]
    fn invocation_ledger_is_append_only() {
        let registry = registry_with_manifest(&[("memory.search", "memory.search")]);
        let ledger_path = temp_dir("ledger-append-only").join("invocation-ledger.jsonl");
        let ledger = KernelInvocationLedger::new(&ledger_path);
        let runtime = KernelInvocationRuntime::new(
            InstalledManifestRegistry::load_from_dir(&registry).expect("registry"),
            KernelHandlerRegistry::new().with_handler("memory.search", smoke_handler),
        );

        runtime.invoke_with_ledger(envelope("memory.search"), &ledger);
        runtime.invoke_with_ledger(envelope("memory.search"), &ledger);

        assert_eq!(read_ledger_records(&ledger_path).len(), 10);
    }

    #[test]
    fn invocation_ledger_does_not_record_raw_payload() {
        let registry = registry_with_manifest(&[("memory.search", "memory.search")]);
        let ledger_path = temp_dir("ledger-no-payload").join("invocation-ledger.jsonl");
        let ledger = KernelInvocationLedger::new(&ledger_path);
        let runtime = KernelInvocationRuntime::new(
            InstalledManifestRegistry::load_from_dir(&registry).expect("registry"),
            KernelHandlerRegistry::new().with_handler("memory.search", smoke_handler),
        );

        runtime.invoke_with_ledger(
            KernelInvocationEnvelope::new(
                "global-main",
                "memory.search",
                "memory.search",
                KernelPayload::Text("raw memory content should not be written".to_string()),
            ),
            &ledger,
        );
        let joined = read_ledger_records(&ledger_path).join("\n");

        assert!(!joined.contains("raw memory content should not be written"));
        assert!(!joined.contains("Text("));
    }

    #[test]
    fn kernel_invocation_ledger_does_not_record_raw_result_payload() {
        let registry = registry_with_manifest(&[("runtime.status", "runtime.status")]);
        let ledger_path = temp_dir("ledger-no-result-payload").join("invocation-ledger.jsonl");
        let ledger = KernelInvocationLedger::new(&ledger_path);
        let runtime = KernelInvocationRuntime::new(
            InstalledManifestRegistry::load_from_dir(&registry).expect("registry"),
            KernelHandlerRegistry::new()
                .with_handler("runtime.status", structured_secret_result_handler),
        );

        runtime.invoke_with_ledger(envelope("runtime.status"), &ledger);
        let joined = read_ledger_records(&ledger_path).join("\n");

        assert!(!joined.contains("structured-secret-field-value"));
        assert!(!joined.contains("raw result payload"));
        assert!(!joined.contains("secret_ref"));
    }

    #[test]
    fn invocation_ledger_redacts_secret_like_values() {
        let registry = registry_with_manifest(&[("memory.search", "memory.search")]);
        let ledger_path = temp_dir("ledger-redaction").join("invocation-ledger.jsonl");
        let ledger = KernelInvocationLedger::new(&ledger_path);
        let runtime = KernelInvocationRuntime::new(
            InstalledManifestRegistry::load_from_dir(&registry).expect("registry"),
            KernelHandlerRegistry::new().with_handler("memory.search", secret_failing_handler),
        );

        runtime.invoke_with_ledger(envelope("memory.search"), &ledger);
        let joined = read_ledger_records(&ledger_path).join("\n");

        assert!(!joined.contains("sk-live-secret-value"));
        assert!(!joined.contains("secret://auth.openai.main"));
        assert!(!joined.contains("token=abc123"));
        assert!(joined.contains("[redacted"));
    }

    #[test]
    fn invocation_ledger_append_failure_before_route_does_not_route_or_execute_handler() {
        let registry = registry_with_manifest(&[("memory.search", "memory.search")]);
        let ledger_path = temp_dir("ledger-fail-before-route").join("invocation-ledger.jsonl");
        let ledger = KernelInvocationLedger::failing_for_test(&ledger_path, "accepted");
        let runtime = KernelInvocationRuntime::new(
            InstalledManifestRegistry::load_from_dir(&registry).expect("registry"),
            KernelHandlerRegistry::new().with_handler("memory.search", smoke_handler),
        );

        let output = runtime.invoke_with_ledger(envelope("memory.search"), &ledger);

        assert_eq!(output.status, KernelInvocationStatus::Failed);
        assert_eq!(output.failure_stage.as_deref(), Some("ledger_append"));
        assert!(!output.route_decision_made);
        assert!(!output.handler_executed);
        assert!(!output.ledger_appended);
    }

    #[test]
    fn invocation_runtime_returns_failure_when_ledger_append_fails() {
        let registry = registry_with_manifest(&[("memory.search", "memory.search")]);
        let ledger_path = temp_dir("ledger-fail-after-route").join("invocation-ledger.jsonl");
        let ledger = KernelInvocationLedger::failing_for_test(&ledger_path, "handler_executed");
        let runtime = KernelInvocationRuntime::new(
            InstalledManifestRegistry::load_from_dir(&registry).expect("registry"),
            KernelHandlerRegistry::new().with_handler("memory.search", smoke_handler),
        );

        let output = runtime.invoke_with_ledger(envelope("memory.search"), &ledger);

        assert_eq!(output.status, KernelInvocationStatus::Failed);
        assert_eq!(output.failure_stage.as_deref(), Some("ledger_append"));
        assert!(output.route_decision_made);
        assert!(output.handler_executed);
        assert!(!output.event_generated);
        assert!(!output.ledger_appended);
    }

    #[test]
    fn invocation_runtime_completed_ledger_append_failure_reports_action_happened() {
        let registry = registry_with_manifest(&[("memory.search", "memory.search")]);
        let ledger_path = temp_dir("ledger-fail-completed").join("invocation-ledger.jsonl");
        let ledger = KernelInvocationLedger::failing_for_test(&ledger_path, "invocation_completed");
        let runtime = KernelInvocationRuntime::new(
            InstalledManifestRegistry::load_from_dir(&registry).expect("registry"),
            KernelHandlerRegistry::new().with_handler("memory.search", smoke_handler),
        );

        let output = runtime.invoke_with_ledger(envelope("memory.search"), &ledger);

        assert_eq!(output.status, KernelInvocationStatus::Failed);
        assert_eq!(output.failure_stage.as_deref(), Some("ledger_append"));
        assert!(output.handler_executed);
        assert!(output.event_generated);
        assert!(!output.ledger_appended);
        assert!(
            output
                .failure_reason
                .as_deref()
                .expect("failure reason")
                .contains("audit close failed after action happened")
        );
    }

    fn smoke_handler(
        envelope: &KernelInvocationEnvelope,
        _route: &crate::KernelRouteRuntimeOutput,
    ) -> Result<KernelHandlerResult, KernelHandlerError> {
        Ok(KernelHandlerResult::summary(format!(
            "smoke handled {}",
            envelope.operation
        )))
    }

    fn structured_status_handler(
        _envelope: &KernelInvocationEnvelope,
        _route: &crate::KernelRouteRuntimeOutput,
    ) -> Result<KernelHandlerResult, KernelHandlerError> {
        let mut fields = BTreeMap::new();
        fields.insert("foundation_installed".to_string(), "yes".to_string());
        fields.insert("kernel_installed".to_string(), "yes".to_string());
        fields.insert("manifest_count".to_string(), "3".to_string());

        Ok(KernelHandlerResult::structured(
            "runtime.status",
            fields,
            "foundation_installed=yes | kernel_installed=yes | manifest_count=3",
        ))
    }

    fn failing_handler(
        _envelope: &KernelInvocationEnvelope,
        _route: &crate::KernelRouteRuntimeOutput,
    ) -> Result<KernelHandlerResult, KernelHandlerError> {
        Err(KernelHandlerError::new("smoke handler failed"))
    }

    fn secret_failing_handler(
        _envelope: &KernelInvocationEnvelope,
        _route: &crate::KernelRouteRuntimeOutput,
    ) -> Result<KernelHandlerResult, KernelHandlerError> {
        Err(KernelHandlerError::new(
            "failed with sk-live-secret-value secret://auth.openai.main token=abc123",
        ))
    }

    fn structured_secret_result_handler(
        _envelope: &KernelInvocationEnvelope,
        _route: &crate::KernelRouteRuntimeOutput,
    ) -> Result<KernelHandlerResult, KernelHandlerError> {
        let mut fields = BTreeMap::new();
        fields.insert(
            "unsafe_detail".to_string(),
            "structured-secret-field-value secret_ref".to_string(),
        );

        Ok(KernelHandlerResult::structured(
            "runtime.status",
            fields,
            "raw result payload structured-secret-field-value",
        ))
    }

    fn envelope(operation: &str) -> KernelInvocationEnvelope {
        KernelInvocationEnvelope::new("global-main", operation, operation, KernelPayload::Empty)
    }

    fn registry_with_manifest(capabilities: &[(&str, &str)]) -> PathBuf {
        let root = temp_dir("invocation-registry");
        write_manifest(&root, "aicore-cli.toml", "aicore-cli", capabilities);
        root
    }

    fn write_manifest(
        root: &PathBuf,
        file_name: &str,
        app_id: &str,
        capabilities: &[(&str, &str)],
    ) {
        write_manifest_with_contract(root, file_name, app_id, "kernel.app.v1", capabilities);
    }

    fn write_manifest_with_contract(
        root: &PathBuf,
        file_name: &str,
        app_id: &str,
        contract_version: &str,
        capabilities: &[(&str, &str)],
    ) {
        let mut content = format!(
            "component_id = \"{app_id}\"\napp_id = \"{app_id}\"\nkind = \"app\"\nentrypoint = \"/home/demo/.aicore/bin/{app_id}\"\ncontract_version = \"{contract_version}\"\n"
        );
        for (id, operation) in capabilities {
            content.push_str(&format!(
                "\n[[capabilities]]\nid = \"{id}\"\noperation = \"{operation}\"\nvisibility = \"user\"\n"
            ));
        }
        fs::write(root.join(file_name), content).expect("write manifest");
    }

    fn temp_dir(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "aicore-kernel-{name}-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("create temp dir");
        path
    }

    fn seed_runtime_status_layout(home: &PathBuf, capability_count: usize) {
        let foundation = home.join(".aicore/runtime/foundation");
        let kernel = home.join(".aicore/runtime/kernel");
        let manifests = home.join(".aicore/share/manifests");
        fs::create_dir_all(&foundation).expect("foundation metadata dir");
        fs::create_dir_all(&kernel).expect("kernel metadata dir");
        fs::create_dir_all(&manifests).expect("manifest dir");
        fs::write(foundation.join("install.toml"), "status = \"installed\"\n")
            .expect("foundation install metadata");
        fs::write(kernel.join("install.toml"), "status = \"installed\"\n")
            .expect("kernel install metadata");
        fs::write(
            manifests.join("aicore.toml"),
            manifest_with_capability_count(capability_count),
        )
        .expect("manifest metadata");
    }

    fn manifest_with_capability_count(count: usize) -> String {
        let mut content = r#"
component_id = "aicore"
app_id = "aicore"
kind = "app"
entrypoint = "/tmp/aicore"
contract_version = "kernel.app.v1"
"#
        .to_string();
        for index in 0..count {
            content.push_str(&format!(
                "\n[[capabilities]]\nid = \"runtime.status.{index}\"\noperation = \"runtime.status.{index}\"\nvisibility = \"user\"\n"
            ));
        }
        content
    }

    fn read_ledger_records(path: &PathBuf) -> Vec<String> {
        fs::read_to_string(path)
            .expect("ledger should be readable")
            .lines()
            .map(ToOwned::to_owned)
            .collect()
    }

    fn ledger_stages(records: &[String]) -> Vec<&'static str> {
        let all_stages = [
            "accepted",
            "route_decision_made",
            "route_failed",
            "handler_lookup_failed",
            "handler_failed",
            "handler_executed",
            "event_generated",
            "invocation_completed",
            "invocation_failed",
        ];
        records
            .iter()
            .map(|record| {
                all_stages
                    .iter()
                    .copied()
                    .find(|stage| record.contains(&format!("\"stage\":\"{stage}\"")))
                    .expect("known stage should exist")
            })
            .collect()
    }

    fn ledger_invocation_ids(records: &[String]) -> Vec<String> {
        records
            .iter()
            .map(|record| extract_json_string(record, "invocation_id"))
            .collect()
    }

    fn extract_json_string(record: &str, key: &str) -> String {
        let marker = format!("\"{key}\":\"");
        let start = record.find(&marker).expect("key should exist") + marker.len();
        let tail = &record[start..];
        let end = tail.find('"').expect("value should end");
        tail[..end].to_string()
    }
}
