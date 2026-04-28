use std::collections::BTreeMap;
use std::fmt;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::Arc;

use crate::{
    ComponentInvocationMode, ComponentTransport, InstalledManifestRegistry, KernelEventEnvelope,
    KernelEventPayload, KernelEventType, KernelInvocationEnvelope, KernelInvocationLedger,
    KernelInvocationLedgerRecord, KernelRouteRuntime, KernelRouteRuntimeError,
    KernelRouteRuntimeInput, KernelRouteRuntimeOutput, Visibility, format_contract,
    redact_failure_reason,
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
    pub transport: Option<String>,
    pub process_exit_code: Option<i32>,
    pub ledger_appended: bool,
    pub ledger_path: Option<String>,
    pub ledger_record_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ComponentProcessSuccess {
    result: KernelHandlerResult,
    exit_code: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ComponentProcessFailure {
    stage: String,
    reason: String,
    spawned_process: bool,
    exit_code: Option<i32>,
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
            return match Self::invoke_local_process(&envelope, &route) {
                Ok(success) => {
                    let (event, result) =
                        Self::event_from_process_result(&envelope, &route, success.result);
                    let transport = Some(route.transport.as_str());
                    if let Err(error) = append(
                        KernelInvocationLedgerRecord::new("handler_executed", "ok", &envelope)
                            .with_route(&route)
                            .with_handler(Some("local_process"), true, false, true, false)
                            .with_transport(transport)
                            .with_process_exit_code(success.exit_code),
                        &mut ledger_records,
                    ) {
                        return Self::ledger_failure(
                            Some(route),
                            None,
                            true,
                            false,
                            Some("local_process".to_string()),
                            true,
                            error,
                            ledger,
                            ledger_records,
                        );
                    }
                    if let Err(error) = append(
                        KernelInvocationLedgerRecord::new("event_generated", "ok", &envelope)
                            .with_route(&route)
                            .with_handler(Some("local_process"), true, true, true, false)
                            .with_transport(transport)
                            .with_process_exit_code(success.exit_code),
                        &mut ledger_records,
                    ) {
                        return Self::ledger_failure(
                            Some(route),
                            Some(event),
                            true,
                            true,
                            Some("local_process".to_string()),
                            true,
                            error,
                            ledger,
                            ledger_records,
                        );
                    }
                    if let Err(error) = append(
                        KernelInvocationLedgerRecord::new("invocation_completed", "ok", &envelope)
                            .with_route(&route)
                            .with_handler(Some("local_process"), true, true, true, false)
                            .with_transport(transport)
                            .with_process_exit_code(success.exit_code),
                        &mut ledger_records,
                    ) {
                        return Self::ledger_failure(
                            Some(route),
                            Some(event),
                            true,
                            true,
                            Some("local_process".to_string()),
                            true,
                            format!("audit close failed after action happened: {error}"),
                            ledger,
                            ledger_records,
                        );
                    }
                    Self::with_ledger(
                        Self::completed_from_event_with_metadata(
                            route,
                            event,
                            Some(result),
                            Some("local_process".to_string()),
                            true,
                            false,
                            Some("stdio_jsonl".to_string()),
                            success.exit_code,
                        ),
                        ledger,
                        ledger_records,
                        true,
                    )
                }
                Err(error) => {
                    let transport = Some(route.transport.as_str());
                    if let Err(ledger_error) = append(
                        KernelInvocationLedgerRecord::new("handler_failed", "failed", &envelope)
                            .with_route(&route)
                            .with_failure(&error.stage, &error.reason)
                            .with_handler(
                                Some("local_process"),
                                error.spawned_process,
                                false,
                                error.spawned_process,
                                false,
                            )
                            .with_transport(transport)
                            .with_process_exit_code(error.exit_code),
                        &mut ledger_records,
                    ) {
                        return Self::ledger_failure(
                            Some(route),
                            None,
                            error.spawned_process,
                            false,
                            Some("local_process".to_string()),
                            true,
                            ledger_error,
                            ledger,
                            ledger_records,
                        );
                    }
                    if let Err(ledger_error) = append(
                        KernelInvocationLedgerRecord::new("invocation_failed", "failed", &envelope)
                            .with_route(&route)
                            .with_failure(&error.stage, &error.reason)
                            .with_handler(
                                Some("local_process"),
                                error.spawned_process,
                                false,
                                error.spawned_process,
                                false,
                            )
                            .with_transport(transport)
                            .with_process_exit_code(error.exit_code),
                        &mut ledger_records,
                    ) {
                        return Self::ledger_failure(
                            Some(route),
                            None,
                            error.spawned_process,
                            false,
                            Some("local_process".to_string()),
                            true,
                            ledger_error,
                            ledger,
                            ledger_records,
                        );
                    }
                    Self::with_ledger(
                        Self::process_failure(&envelope, route, error),
                        ledger,
                        ledger_records,
                        true,
                    )
                }
            };
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
            transport: None,
            process_exit_code: None,
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
            transport: Some(route.transport.as_str().to_string()),
            process_exit_code: None,
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
            transport: Some(route.transport.as_str().to_string()),
            process_exit_code: None,
            ledger_appended: false,
            ledger_path: None,
            ledger_record_count: 0,
        }
    }

    fn process_failure(
        envelope: &KernelInvocationEnvelope,
        route: KernelRouteRuntimeOutput,
        error: ComponentProcessFailure,
    ) -> KernelInvocationRuntimeOutput {
        KernelInvocationRuntimeOutput {
            status: KernelInvocationStatus::Failed,
            route: Some(route.clone()),
            event: None,
            result: Some(Self::failure_result(
                envelope,
                Some(&route),
                &error.stage,
                &error.reason,
                error.spawned_process,
                false,
                Some("local_process".to_string()),
            )),
            route_decision_made: true,
            handler_executed: error.spawned_process,
            event_generated: false,
            handler_kind: Some("local_process".to_string()),
            failure_stage: Some(error.stage),
            failure_reason: Some(error.reason),
            spawned_process: error.spawned_process,
            called_real_component: false,
            transport: Some(route.transport.as_str().to_string()),
            process_exit_code: error.exit_code,
            ledger_appended: false,
            ledger_path: None,
            ledger_record_count: 0,
        }
    }

    fn invoke_local_process(
        envelope: &KernelInvocationEnvelope,
        route: &KernelRouteRuntimeOutput,
    ) -> Result<ComponentProcessSuccess, ComponentProcessFailure> {
        if route.transport != ComponentTransport::StdioJsonl {
            return Err(ComponentProcessFailure {
                stage: "transport_unsupported".to_string(),
                reason: format!(
                    "unsupported component transport: {}",
                    route.transport.as_str()
                ),
                spawned_process: false,
                exit_code: None,
            });
        }

        if route.entrypoint.trim().is_empty() || !Path::new(&route.entrypoint).exists() {
            return Err(ComponentProcessFailure {
                stage: "missing_entrypoint".to_string(),
                reason: format!("component entrypoint is missing: {}", route.entrypoint),
                spawned_process: false,
                exit_code: None,
            });
        }

        let mut command = Command::new(&route.entrypoint);
        command.args(&route.args);
        if let Some(working_dir) = &route.working_dir {
            command.current_dir(working_dir);
        }
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = command.spawn().map_err(|error| ComponentProcessFailure {
            stage: "process_spawn".to_string(),
            reason: sanitize_process_diagnostic(&format!(
                "component process spawn failed: {error}"
            )),
            spawned_process: false,
            exit_code: None,
        })?;

        let request = local_ipc_request_json(envelope, route);
        if let Some(stdin) = child.stdin.as_mut() {
            if let Err(error) = stdin.write_all(request.as_bytes()) {
                let _ = child.kill();
                return Err(ComponentProcessFailure {
                    stage: "ipc_write".to_string(),
                    reason: sanitize_process_diagnostic(&format!(
                        "component ipc write failed: {error}"
                    )),
                    spawned_process: true,
                    exit_code: None,
                });
            }
            if let Err(error) = stdin.write_all(b"\n") {
                let _ = child.kill();
                return Err(ComponentProcessFailure {
                    stage: "ipc_write".to_string(),
                    reason: sanitize_process_diagnostic(&format!(
                        "component ipc write failed: {error}"
                    )),
                    spawned_process: true,
                    exit_code: None,
                });
            }
        }
        drop(child.stdin.take());

        let output = child
            .wait_with_output()
            .map_err(|error| ComponentProcessFailure {
                stage: "ipc_read".to_string(),
                reason: sanitize_process_diagnostic(&format!(
                    "component process output read failed: {error}"
                )),
                spawned_process: true,
                exit_code: None,
            })?;

        let exit_code = output.status.code();
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ComponentProcessFailure {
                stage: "process_exit".to_string(),
                reason: sanitize_process_diagnostic(&format!(
                    "component process exited with code {:?}: {}",
                    exit_code,
                    stderr.trim()
                )),
                spawned_process: true,
                exit_code,
            });
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let Some(line) = stdout.lines().find(|line| !line.trim().is_empty()) else {
            return Err(ComponentProcessFailure {
                stage: "ipc_read".to_string(),
                reason: "component process returned empty stdio_jsonl result".to_string(),
                spawned_process: true,
                exit_code,
            });
        };
        let value: serde_json::Value =
            serde_json::from_str(line).map_err(|error| ComponentProcessFailure {
                stage: "ipc_read".to_string(),
                reason: sanitize_process_diagnostic(&format!(
                    "component process returned invalid JSON result: {error}"
                )),
                spawned_process: true,
                exit_code,
            })?;

        let result_kind = value
            .get("result_kind")
            .and_then(|value| value.as_str())
            .unwrap_or("component.process.result")
            .to_string();
        let summary = value
            .get("summary")
            .and_then(|value| value.as_str())
            .unwrap_or("component process completed")
            .to_string();
        let mut fields = BTreeMap::new();
        if let Some(object) = value.get("fields").and_then(|value| value.as_object()) {
            for (key, value) in object {
                fields.insert(key.clone(), json_value_to_public_string(value));
            }
        }
        fields
            .entry("transport".to_string())
            .or_insert_with(|| route.transport.as_str().to_string());

        Ok(ComponentProcessSuccess {
            result: KernelHandlerResult::structured(result_kind, fields, summary),
            exit_code,
        })
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
        Self::event_from_result_with_handler_kind(envelope, route, result, "in_process")
    }

    fn event_from_process_result(
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

    fn completed_from_event(
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

    fn completed_from_event_with_metadata(
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

fn local_ipc_request_json(
    envelope: &KernelInvocationEnvelope,
    route: &KernelRouteRuntimeOutput,
) -> String {
    serde_json::json!({
        "schema_version": "aicore.local_ipc.invocation.v1",
        "invocation_id": envelope.invocation_id,
        "trace_id": envelope.trace_context.trace_id,
        "instance_id": envelope.instance_id,
        "operation": envelope.operation,
        "route": {
            "component_id": route.component_id,
            "app_id": route.app_id,
            "capability_id": route.capability_id,
            "contract_version": format_contract(&route.contract_version),
            "invocation_mode": route.invocation_mode.as_str(),
            "transport": route.transport.as_str(),
        }
    })
    .to_string()
}

fn json_value_to_public_string(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::Bool(value) => value.to_string(),
        serde_json::Value::Number(value) => value.to_string(),
        serde_json::Value::String(value) => value.clone(),
        value => value.to_string(),
    }
}

fn sanitize_process_diagnostic(value: &str) -> String {
    let without_control = value
        .chars()
        .filter(|character| !character.is_control() || matches!(character, '\n' | '\t'))
        .collect::<String>();
    let redacted = redact_failure_reason(&without_control);
    let mut summary = redacted.replace('\n', " ");
    if summary.chars().count() > 240 {
        summary = summary.chars().take(240).collect::<String>();
        summary.push_str("...");
    }
    summary
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
    fn in_process_runtime_status_still_works() {
        let registry = registry_with_manifest(&[("runtime.status", "runtime.status")]);
        let runtime = KernelInvocationRuntime::new(
            InstalledManifestRegistry::load_from_dir(&registry).expect("registry"),
            KernelHandlerRegistry::new().with_handler("runtime.status", structured_status_handler),
        );

        let output = runtime.invoke(envelope("runtime.status"));

        assert_eq!(output.status, KernelInvocationStatus::Completed);
        assert_eq!(output.handler_kind.as_deref(), Some("in_process"));
        assert!(!output.spawned_process);
        assert_eq!(
            output.result.expect("result").result_kind.as_deref(),
            Some("runtime.status")
        );
    }

    #[test]
    fn component_process_smoke_invokes_stdio_jsonl_child() {
        let root = temp_dir("process-smoke-success");
        let script = process_fixture_script(
            &root,
            "process-smoke-success.sh",
            r#"read line
printf '{"result_kind":"component.process.smoke","summary":"process smoke ok","fields":{"operation":"component.process.smoke","ipc":"stdio_jsonl"}}\n'
"#,
        );
        write_process_manifest(&root, &script, "stdio_jsonl", &[]);
        let ledger_path = temp_dir("process-smoke-success-ledger").join("invocation-ledger.jsonl");
        let ledger = KernelInvocationLedger::new(&ledger_path);
        let runtime = KernelInvocationRuntime::new(
            InstalledManifestRegistry::load_from_dir(&root).expect("registry"),
            KernelHandlerRegistry::new(),
        );

        let output = runtime.invoke_with_ledger(envelope("component.process.smoke"), &ledger);

        assert_eq!(output.status, KernelInvocationStatus::Completed);
        assert!(output.handler_executed);
        assert!(output.event_generated);
        assert_eq!(output.handler_kind.as_deref(), Some("local_process"));
        assert_eq!(output.transport.as_deref(), Some("stdio_jsonl"));
        assert!(output.spawned_process);
        assert_eq!(output.process_exit_code, Some(0));
        let result = output.result.expect("process result envelope");
        assert_eq!(
            result.result_kind.as_deref(),
            Some("component.process.smoke")
        );
        assert_eq!(
            result.public_fields.get("ipc"),
            Some(&"stdio_jsonl".to_string())
        );
    }

    #[test]
    fn component_process_smoke_writes_invocation_ledger() {
        let root = temp_dir("process-smoke-ledger");
        let script = process_fixture_script(
            &root,
            "process-smoke-ledger.sh",
            r#"read line
printf '{"result_kind":"component.process.smoke","summary":"process smoke ok","fields":{"operation":"component.process.smoke"}}\n'
"#,
        );
        write_process_manifest(&root, &script, "stdio_jsonl", &[]);
        let ledger_path = temp_dir("process-smoke-ledger-path").join("invocation-ledger.jsonl");
        let ledger = KernelInvocationLedger::new(&ledger_path);
        let runtime = KernelInvocationRuntime::new(
            InstalledManifestRegistry::load_from_dir(&root).expect("registry"),
            KernelHandlerRegistry::new(),
        );

        let output = runtime.invoke_with_ledger(envelope("component.process.smoke"), &ledger);
        let joined = read_ledger_records(&ledger_path).join("\n");

        assert_eq!(output.status, KernelInvocationStatus::Completed);
        assert_eq!(output.ledger_record_count, 5);
        assert!(joined.contains("\"handler_kind\":\"local_process\""));
        assert!(joined.contains("\"spawned_process\":true"));
        assert!(joined.contains("\"transport\":\"stdio_jsonl\""));
        assert!(!joined.contains("process smoke ok"));
    }

    #[test]
    fn component_process_unsupported_transport_returns_structured_failure() {
        let root = temp_dir("process-unsupported-transport");
        write_process_manifest(&root, "/bin/sh", "unix_socket", &[]);
        let runtime = KernelInvocationRuntime::new(
            InstalledManifestRegistry::load_from_dir(&root).expect("registry"),
            KernelHandlerRegistry::new(),
        );

        let output = runtime.invoke(envelope("component.process.smoke"));

        assert_eq!(output.status, KernelInvocationStatus::Failed);
        assert_eq!(
            output.failure_stage.as_deref(),
            Some("transport_unsupported")
        );
        assert_eq!(output.handler_kind.as_deref(), Some("local_process"));
        assert!(!output.spawned_process);
        assert!(!output.event_generated);
    }

    #[test]
    fn component_process_missing_entrypoint_returns_structured_failure() {
        let root = temp_dir("process-missing-entrypoint");
        write_process_manifest(
            &root,
            root.join("missing-component")
                .display()
                .to_string()
                .as_str(),
            "stdio_jsonl",
            &[],
        );
        let runtime = KernelInvocationRuntime::new(
            InstalledManifestRegistry::load_from_dir(&root).expect("registry"),
            KernelHandlerRegistry::new(),
        );

        let output = runtime.invoke(envelope("component.process.smoke"));

        assert_eq!(output.status, KernelInvocationStatus::Failed);
        assert_eq!(output.failure_stage.as_deref(), Some("missing_entrypoint"));
        assert_eq!(output.handler_kind.as_deref(), Some("local_process"));
        assert!(!output.spawned_process);
    }

    #[test]
    fn component_process_nonzero_exit_returns_structured_failure() {
        let root = temp_dir("process-nonzero");
        let script = process_fixture_script(
            &root,
            "process-nonzero.sh",
            r#"read line
printf 'failed with sk-live-secret-value token=abc123\n' >&2
exit 42
"#,
        );
        write_process_manifest(&root, &script, "stdio_jsonl", &[]);
        let ledger_path = temp_dir("process-nonzero-ledger").join("invocation-ledger.jsonl");
        let ledger = KernelInvocationLedger::new(&ledger_path);
        let runtime = KernelInvocationRuntime::new(
            InstalledManifestRegistry::load_from_dir(&root).expect("registry"),
            KernelHandlerRegistry::new(),
        );

        let output = runtime.invoke_with_ledger(envelope("component.process.smoke"), &ledger);
        let joined = read_ledger_records(&ledger_path).join("\n");

        assert_eq!(output.status, KernelInvocationStatus::Failed);
        assert_eq!(output.failure_stage.as_deref(), Some("process_exit"));
        assert_eq!(output.process_exit_code, Some(42));
        assert!(output.spawned_process);
        assert!(!output.event_generated);
        assert!(!joined.contains("sk-live-secret-value"));
        assert!(!joined.contains("token=abc123"));
        assert!(joined.contains("[redacted"));
    }

    #[test]
    fn component_process_invalid_json_returns_structured_failure() {
        let root = temp_dir("process-invalid-json");
        let script = process_fixture_script(
            &root,
            "process-invalid-json.sh",
            r#"read line
printf 'not json\n'
"#,
        );
        write_process_manifest(&root, &script, "stdio_jsonl", &[]);
        let runtime = KernelInvocationRuntime::new(
            InstalledManifestRegistry::load_from_dir(&root).expect("registry"),
            KernelHandlerRegistry::new(),
        );

        let output = runtime.invoke(envelope("component.process.smoke"));

        assert_eq!(output.status, KernelInvocationStatus::Failed);
        assert_eq!(output.failure_stage.as_deref(), Some("ipc_read"));
        assert_eq!(output.handler_kind.as_deref(), Some("local_process"));
        assert!(output.spawned_process);
        assert!(!output.event_generated);
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

    fn write_process_manifest(root: &PathBuf, entrypoint: &str, transport: &str, args: &[&str]) {
        let args_toml = args
            .iter()
            .map(|arg| format!("\"{}\"", arg.replace('"', "\\\"")))
            .collect::<Vec<_>>()
            .join(", ");
        let content = format!(
            "component_id = \"aicore-component-smoke\"\napp_id = \"aicore-cli\"\nkind = \"app\"\nentrypoint = \"{}\"\ninvocation_mode = \"local_process\"\ntransport = \"{transport}\"\nargs = [{args_toml}]\ncontract_version = \"kernel.app.v1\"\n\n[[capabilities]]\nid = \"component.process.smoke\"\noperation = \"component.process.smoke\"\nvisibility = \"diagnostic\"\n",
            entrypoint.replace('"', "\\\"")
        );
        fs::write(root.join("aicore-component-smoke.toml"), content).expect("write manifest");
    }

    fn process_fixture_script(root: &PathBuf, name: &str, body: &str) -> String {
        let path = root.join(name);
        fs::write(&path, format!("#!/bin/sh\n{body}")).expect("write process fixture");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = fs::metadata(&path).expect("script metadata").permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&path, permissions).expect("script should be executable");
        }
        path.display().to_string()
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
