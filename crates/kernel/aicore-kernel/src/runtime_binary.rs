use std::io::Write;
use std::process::{Command, Stdio};

use aicore_foundation::AicoreLayout;

use crate::{
    KernelInvocationEnvelope, KernelInvocationRuntimeOutput, KernelInvocationStatus, KernelPayload,
};

pub const FOUNDATION_RUNTIME_BINARY_NAME: &str = "aicore-foundation";
pub const KERNEL_RUNTIME_BINARY_NAME: &str = "aicore-kernel";
pub const RUNTIME_BINARY_PROTOCOL: &str = "stdio_jsonl";

#[derive(Debug, Clone)]
pub struct KernelRuntimeBinaryClient {
    layout: AicoreLayout,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelRuntimeBinaryInvocation {
    pub payload: serde_json::Value,
    pub exit_success: bool,
}

impl KernelRuntimeBinaryClient {
    pub fn new(layout: AicoreLayout) -> Self {
        Self { layout }
    }

    pub fn invoke_readonly(&self, operation: &str) -> KernelRuntimeBinaryInvocation {
        let envelope = KernelInvocationEnvelope::new(
            "global-main",
            operation,
            operation,
            KernelPayload::Empty,
        );
        self.invoke_envelope(envelope)
    }

    pub fn invoke_envelope(
        &self,
        envelope: KernelInvocationEnvelope,
    ) -> KernelRuntimeBinaryInvocation {
        let foundation_binary = self.layout.bin_root.join(FOUNDATION_RUNTIME_BINARY_NAME);
        let kernel_binary = self.layout.bin_root.join(KERNEL_RUNTIME_BINARY_NAME);
        let ledger_path = self
            .layout
            .kernel_state_root
            .join("invocation-ledger.jsonl");

        if !foundation_binary.exists() {
            return KernelRuntimeBinaryInvocation {
                payload: runtime_binary_failure_payload(
                    &envelope,
                    "foundation_runtime_binary_missing",
                    &format!(
                        "Foundation runtime binary missing: {}",
                        foundation_binary.display()
                    ),
                    &self.layout,
                    false,
                    None,
                ),
                exit_success: false,
            };
        }

        if !kernel_binary.exists() {
            return KernelRuntimeBinaryInvocation {
                payload: runtime_binary_failure_payload(
                    &envelope,
                    "kernel_runtime_binary_missing",
                    &format!("Kernel runtime binary missing: {}", kernel_binary.display()),
                    &self.layout,
                    false,
                    None,
                ),
                exit_success: false,
            };
        }

        let mut child = match Command::new(&kernel_binary)
            .arg("--invoke-stdio-jsonl")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(child) => child,
            Err(error) => {
                return KernelRuntimeBinaryInvocation {
                    payload: runtime_binary_failure_payload(
                        &envelope,
                        "kernel_runtime_process_spawn",
                        &format!("Kernel runtime binary spawn failed: {error}"),
                        &self.layout,
                        false,
                        None,
                    ),
                    exit_success: false,
                };
            }
        };

        if let Some(stdin) = child.stdin.as_mut() {
            let request = serde_json::json!({
                "schema_version": "aicore.kernel.runtime_binary.invocation.v1",
                "invocation_id": envelope.invocation_id,
                "trace_id": envelope.trace_context.trace_id,
                "instance_id": envelope.instance_id,
                "capability": envelope.capability,
                "operation": envelope.operation,
                "payload": envelope.payload.summary(),
                "ledger_path": ledger_path.display().to_string(),
            });
            if let Err(error) = writeln!(stdin, "{request}") {
                let _ = child.kill();
                return KernelRuntimeBinaryInvocation {
                    payload: runtime_binary_failure_payload(
                        &envelope,
                        "kernel_runtime_ipc_write",
                        &format!("Kernel runtime binary stdin write failed: {error}"),
                        &self.layout,
                        true,
                        None,
                    ),
                    exit_success: false,
                };
            }
        }
        drop(child.stdin.take());

        let output = match child.wait_with_output() {
            Ok(output) => output,
            Err(error) => {
                return KernelRuntimeBinaryInvocation {
                    payload: runtime_binary_failure_payload(
                        &envelope,
                        "kernel_runtime_ipc_read",
                        &format!("Kernel runtime binary output read failed: {error}"),
                        &self.layout,
                        true,
                        None,
                    ),
                    exit_success: false,
                };
            }
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        if let Some(mut payload) = parse_kernel_invocation_result_payload(&stdout) {
            if let Some(handler) = payload
                .get_mut("handler")
                .and_then(|value| value.as_object_mut())
            {
                handler.insert(
                    "process_exit_code".to_string(),
                    output
                        .status
                        .code()
                        .map(serde_json::Value::from)
                        .unwrap_or(serde_json::Value::Null),
                );
                handler.insert("spawned_process".to_string(), serde_json::Value::Bool(true));
                handler.insert(
                    "first_party_in_process_adapter".to_string(),
                    serde_json::Value::Bool(false),
                );
            }
            return KernelRuntimeBinaryInvocation {
                payload,
                exit_success: output.status.success(),
            };
        }

        let stderr = String::from_utf8_lossy(&output.stderr);
        KernelRuntimeBinaryInvocation {
            payload: runtime_binary_failure_payload(
                &envelope,
                "kernel_runtime_protocol",
                &format!(
                    "Kernel runtime binary returned no valid invocation result; exit={:?}; stderr={}",
                    output.status.code(),
                    sanitize_runtime_binary_diagnostic(&stderr)
                ),
                &self.layout,
                true,
                output.status.code(),
            ),
            exit_success: false,
        }
    }
}

pub fn kernel_invocation_result_public_json(
    output: &KernelInvocationRuntimeOutput,
) -> serde_json::Value {
    let result = output.result.as_ref();
    let route = result
        .and_then(|result| result.route.as_ref())
        .map(|route| {
            serde_json::json!({
                "component_id": route.component_id,
                "app_id": route.app_id,
                "capability_id": route.capability_id,
                "contract_version": route.contract_version,
            })
        })
        .or_else(|| {
            output.route.as_ref().map(|route| {
                serde_json::json!({
                    "component_id": route.component_id,
                    "app_id": route.app_id,
                    "capability_id": route.capability_id,
                    "contract_version": crate::format_contract(&route.contract_version),
                })
            })
        })
        .unwrap_or(serde_json::Value::Null);
    let fields = result
        .map(|result| serde_json::json!(result.public_fields))
        .unwrap_or_else(|| serde_json::json!({}));
    let via_runtime_binary = result
        .and_then(|result| result.public_fields.get("kernel_invocation_path"))
        .is_some_and(|value| value == "binary");
    let handler_kind = if via_runtime_binary {
        Some("kernel_runtime_binary")
    } else {
        output.handler_kind.as_deref()
    };
    let invocation_mode = if via_runtime_binary {
        Some("local_process")
    } else {
        output
            .route
            .as_ref()
            .map(|route| route.invocation_mode.as_str())
    };
    let transport = if via_runtime_binary {
        Some(RUNTIME_BINARY_PROTOCOL)
    } else {
        output.transport.as_deref()
    };
    let spawned_process = output.spawned_process || via_runtime_binary;

    serde_json::json!({
        "invocation_id": result
            .map(|result| result.invocation_id.as_str())
            .or_else(|| output.event.as_ref().map(|event| event.invocation_id.as_str())),
        "trace_id": result
            .map(|result| result.trace_id.as_str())
            .or_else(|| output.event.as_ref().map(|event| event.trace_context.trace_id.as_str())),
        "operation": result
            .map(|result| result.operation.as_str())
            .or_else(|| output.route.as_ref().map(|route| route.operation.as_str())),
        "status": match output.status {
            KernelInvocationStatus::Completed => "completed",
            KernelInvocationStatus::Failed => "failed",
        },
        "route": route,
        "handler": {
            "kind": handler_kind,
            "invocation_mode": invocation_mode,
            "transport": transport,
            "process_exit_code": output.process_exit_code,
            "executed": output.handler_executed,
            "event_generated": output.event_generated,
            "spawned_process": spawned_process,
            "called_real_component": output.called_real_component,
            "first_party_in_process_adapter": !via_runtime_binary
                && output.handler_kind.as_deref() == Some("in_process")
                && result.and_then(|result| result.result_kind.as_deref()) == Some("runtime.status"),
        },
        "ledger": {
            "appended": output.ledger_appended,
            "path": output.ledger_path.as_deref(),
            "records": output.ledger_record_count,
        },
        "result": {
            "kind": result.and_then(|result| result.result_kind.as_deref()),
            "summary": result.map(|result| result.summary.as_str()),
            "fields": fields,
        },
        "failure": {
            "stage": output.failure_stage.as_deref(),
            "reason": output.failure_reason.as_deref(),
        }
    })
}

fn parse_kernel_invocation_result_payload(stdout: &str) -> Option<serde_json::Value> {
    stdout.lines().find_map(|line| {
        let value: serde_json::Value = serde_json::from_str(line).ok()?;
        if value.get("event").and_then(|event| event.as_str()) == Some("kernel.invocation.result") {
            return value.get("payload").cloned();
        }
        None
    })
}

fn runtime_binary_failure_payload(
    envelope: &KernelInvocationEnvelope,
    stage: &str,
    reason: &str,
    layout: &AicoreLayout,
    spawned_process: bool,
    process_exit_code: Option<i32>,
) -> serde_json::Value {
    let foundation_binary = layout.bin_root.join(FOUNDATION_RUNTIME_BINARY_NAME);
    let kernel_binary = layout.bin_root.join(KERNEL_RUNTIME_BINARY_NAME);
    serde_json::json!({
        "invocation_id": envelope.invocation_id,
        "trace_id": envelope.trace_context.trace_id,
        "operation": envelope.operation,
        "status": "failed",
        "route": serde_json::Value::Null,
        "handler": {
            "kind": "kernel_runtime_binary",
            "invocation_mode": "local_process",
            "transport": RUNTIME_BINARY_PROTOCOL,
            "process_exit_code": process_exit_code,
            "executed": false,
            "event_generated": false,
            "spawned_process": spawned_process,
            "called_real_component": false,
            "first_party_in_process_adapter": false,
        },
        "ledger": {
            "appended": false,
            "path": layout.kernel_state_root.join("invocation-ledger.jsonl").display().to_string(),
            "records": 0,
        },
        "result": {
            "kind": serde_json::Value::Null,
            "summary": serde_json::Value::Null,
            "fields": {},
        },
        "failure": {
            "stage": stage,
            "reason": sanitize_runtime_binary_diagnostic(reason),
        },
        "runtime_binary": {
            "foundation_path": foundation_binary.display().to_string(),
            "foundation_installed": foundation_binary.exists(),
            "kernel_path": kernel_binary.display().to_string(),
            "kernel_installed": kernel_binary.exists(),
            "protocol": RUNTIME_BINARY_PROTOCOL,
            "in_process_fallback": false,
        }
    })
}

fn sanitize_runtime_binary_diagnostic(value: &str) -> String {
    let without_control = value
        .chars()
        .filter(|character| !character.is_control() || matches!(character, '\n' | '\t'))
        .collect::<String>();
    let mut summary = crate::redact_failure_reason(&without_control).replace('\n', " ");
    if summary.chars().count() > 240 {
        summary = summary.chars().take(240).collect::<String>();
        summary.push_str("...");
    }
    summary
}
