use std::collections::BTreeMap;
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::{
    ComponentTransport, KernelInvocationEnvelope, KernelInvocationLedger,
    KernelInvocationLedgerRecord, KernelRouteRuntimeOutput,
};

use super::{KernelHandlerResult, KernelInvocationRuntime, KernelInvocationRuntimeOutput};

mod types;
mod validate;
mod wait;

use types::{ComponentProcessFailure, ComponentProcessOutput, ComponentProcessSuccess};
use validate::parse_component_process_result;
use wait::{timeout_duration, wait_with_timeout};

impl KernelInvocationRuntime {
    pub(super) fn invoke_with_ledger_local_process(
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

        match Self::invoke_local_process(&envelope, &route) {
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
        }
    }

    pub(super) fn invoke_local_process(
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
                result: None,
                spawned_process: false,
                exit_code: None,
            });
        }

        let entrypoint = Path::new(&route.entrypoint);
        if route.entrypoint.trim().is_empty() || !entrypoint.exists() {
            return Err(ComponentProcessFailure {
                stage: "missing_entrypoint".to_string(),
                reason: format!("component entrypoint is missing: {}", route.entrypoint),
                result: None,
                spawned_process: false,
                exit_code: None,
            });
        }
        if !is_executable_file(entrypoint) {
            return Err(ComponentProcessFailure {
                stage: "entrypoint_not_executable".to_string(),
                reason: format!(
                    "component entrypoint is not executable: {}",
                    route.entrypoint
                ),
                result: None,
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

        #[cfg(test)]
        let _process_spawn_guard = crate::test_support::process_spawn_lock();

        let mut child = command.spawn().map_err(|error| ComponentProcessFailure {
            stage: "process_spawn_failed".to_string(),
            reason: super::protocol::sanitize_process_diagnostic(&format!(
                "component process spawn failed: {error}"
            )),
            result: None,
            spawned_process: false,
            exit_code: None,
        })?;

        let request = super::protocol::local_ipc_request_json(envelope, route);
        if let Some(stdin) = child.stdin.as_mut() {
            if let Err(error) = stdin.write_all(request.as_bytes()) {
                let _ = child.kill();
                let _ = child.wait();
                return Err(ComponentProcessFailure {
                    stage: "process_stdin_failed".to_string(),
                    reason: super::protocol::sanitize_process_diagnostic(&format!(
                        "component ipc write failed: {error}"
                    )),
                    result: None,
                    spawned_process: true,
                    exit_code: None,
                });
            }
            if let Err(error) = stdin.write_all(b"\n") {
                let _ = child.kill();
                let _ = child.wait();
                return Err(ComponentProcessFailure {
                    stage: "process_stdin_failed".to_string(),
                    reason: super::protocol::sanitize_process_diagnostic(&format!(
                        "component ipc write failed: {error}"
                    )),
                    result: None,
                    spawned_process: true,
                    exit_code: None,
                });
            }
        }
        drop(child.stdin.take());

        let output = wait_with_timeout(child, timeout_duration(envelope))?;

        let exit_code = output.status.code();
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ComponentProcessFailure {
                stage: "process_non_zero_exit".to_string(),
                reason: super::protocol::sanitize_process_diagnostic(&format!(
                    "component process exited with code {:?}: {}",
                    exit_code,
                    stderr.trim()
                )),
                result: None,
                spawned_process: true,
                exit_code,
            });
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let value = parse_component_process_result(&stdout, envelope, exit_code)?;

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
                fields.insert(
                    key.clone(),
                    super::protocol::json_value_to_public_string(value),
                );
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

    pub(super) fn process_failure(
        envelope: &KernelInvocationEnvelope,
        route: KernelRouteRuntimeOutput,
        error: ComponentProcessFailure,
    ) -> KernelInvocationRuntimeOutput {
        KernelInvocationRuntimeOutput {
            status: super::KernelInvocationStatus::Failed,
            route: Some(route.clone()),
            event: None,
            result: Some(Self::process_failure_result(envelope, &route, &error)),
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
}

impl KernelInvocationRuntime {
    fn process_failure_result(
        envelope: &KernelInvocationEnvelope,
        route: &KernelRouteRuntimeOutput,
        error: &ComponentProcessFailure,
    ) -> super::KernelInvocationResultEnvelope {
        if let Some(result) = error.result.clone() {
            return super::KernelInvocationResultEnvelope {
                invocation_id: envelope.invocation_id.clone(),
                trace_id: envelope.trace_context.trace_id.clone(),
                operation: envelope.operation.clone(),
                status: super::KernelInvocationStatus::Failed,
                route: Some(super::KernelInvocationResultRoute {
                    component_id: route.component_id.clone(),
                    app_id: route.app_id.clone(),
                    capability_id: route.capability_id.clone(),
                    contract_version: crate::format_contract(&route.contract_version),
                }),
                handler_kind: Some("local_process".to_string()),
                result_kind: result.result_kind,
                summary: result.summary,
                public_fields: result.public_fields,
                failure_stage: Some(error.stage.clone()),
                failure_reason: Some(error.reason.clone()),
                handler_executed: error.spawned_process,
                event_generated: false,
                ledger_appended: false,
            };
        }
        Self::failure_result(
            envelope,
            Some(route),
            &error.stage,
            &error.reason,
            error.spawned_process,
            false,
            Some("local_process".to_string()),
        )
    }
}

fn is_executable_file(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        return std::fs::metadata(path)
            .map(|metadata| metadata.permissions().mode() & 0o111 != 0)
            .unwrap_or(false);
    }
    #[cfg(not(unix))]
    {
        true
    }
}
