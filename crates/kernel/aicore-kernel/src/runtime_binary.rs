use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use aicore_foundation::AicoreLayout;

use crate::{
    KernelInvocationEnvelope, KernelInvocationRuntimeOutput, KernelInvocationStatus, KernelPayload,
};

pub const FOUNDATION_RUNTIME_BINARY_NAME: &str = "aicore-foundation";
pub const KERNEL_RUNTIME_BINARY_NAME: &str = "aicore-kernel";
pub const RUNTIME_BINARY_PROTOCOL: &str = "stdio_jsonl";
pub const RUNTIME_BINARY_PROTOCOL_VERSION: &str = "aicore.kernel.runtime_binary.stdio_jsonl.v1";
pub const RUNTIME_BINARY_REQUEST_SCHEMA_VERSION: &str = "aicore.kernel.runtime_binary.request.v1";
pub const RUNTIME_BINARY_RESPONSE_SCHEMA_VERSION: &str = "aicore.kernel.runtime_binary.response.v1";
pub const RUNTIME_BINARY_CONTRACT_VERSION: &str = "kernel.runtime.v1";

#[derive(Debug, Clone)]
pub struct KernelRuntimeBinaryClient {
    layout: AicoreLayout,
    foundation_binary_path: PathBuf,
    kernel_binary_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelRuntimeBinaryRequest {
    pub schema_version: String,
    pub request_id: String,
    pub protocol: String,
    pub protocol_version: String,
    pub contract_version: String,
    pub invocation_id: String,
    pub trace_id: String,
    pub instance_id: String,
    pub capability: String,
    pub operation: String,
    pub payload_summary: String,
    pub ledger_path: String,
}

impl KernelRuntimeBinaryRequest {
    pub fn from_envelope(envelope: &KernelInvocationEnvelope, layout: &AicoreLayout) -> Self {
        Self {
            schema_version: RUNTIME_BINARY_REQUEST_SCHEMA_VERSION.to_string(),
            request_id: format!("request.{}", envelope.invocation_id),
            protocol: RUNTIME_BINARY_PROTOCOL.to_string(),
            protocol_version: RUNTIME_BINARY_PROTOCOL_VERSION.to_string(),
            contract_version: RUNTIME_BINARY_CONTRACT_VERSION.to_string(),
            invocation_id: envelope.invocation_id.clone(),
            trace_id: envelope.trace_context.trace_id.clone(),
            instance_id: envelope.instance_id.clone(),
            capability: envelope.capability.clone(),
            operation: envelope.operation.clone(),
            payload_summary: envelope.payload.summary(),
            ledger_path: layout
                .kernel_state_root
                .join("invocation-ledger.jsonl")
                .display()
                .to_string(),
        }
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "schema_version": self.schema_version,
            "request_id": self.request_id,
            "protocol": self.protocol,
            "protocol_version": self.protocol_version,
            "contract_version": self.contract_version,
            "invocation_id": self.invocation_id,
            "trace_id": self.trace_id,
            "instance_id": self.instance_id,
            "capability": self.capability,
            "operation": self.operation,
            "payload": self.payload_summary,
            "ledger_path": self.ledger_path,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelRuntimeBinaryResponse {
    pub protocol: String,
    pub protocol_version: String,
    pub contract_version: String,
    pub payload: serde_json::Value,
    pub exit_code: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KernelRuntimeBinaryErrorKind {
    FoundationBinaryMissing,
    FoundationBinaryNotExecutable,
    KernelBinaryMissing,
    KernelBinaryNotExecutable,
    ProcessSpawnFailed,
    StdinWriteFailed,
    StdoutReadFailed,
    NonZeroExit,
    InvalidJsonlOutput,
    ProtocolVersionMismatch,
    ContractVersionMismatch,
    KernelInvocationFailed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelRuntimeBinaryError {
    pub kind: KernelRuntimeBinaryErrorKind,
    pub stage: String,
    pub message: String,
    pub exit_code: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelRuntimeBinaryInvocation {
    pub request: KernelRuntimeBinaryRequest,
    pub response: Option<KernelRuntimeBinaryResponse>,
    pub payload: serde_json::Value,
    pub exit_success: bool,
    pub error: Option<KernelRuntimeBinaryError>,
}

impl KernelRuntimeBinaryClient {
    pub fn new(layout: AicoreLayout) -> Self {
        let foundation_binary_path = layout.bin_root.join(FOUNDATION_RUNTIME_BINARY_NAME);
        let kernel_binary_path = layout.bin_root.join(KERNEL_RUNTIME_BINARY_NAME);
        Self {
            layout,
            foundation_binary_path,
            kernel_binary_path,
        }
    }

    pub fn with_binary_paths(
        layout: AicoreLayout,
        foundation_binary_path: PathBuf,
        kernel_binary_path: PathBuf,
    ) -> Self {
        Self {
            layout,
            foundation_binary_path,
            kernel_binary_path,
        }
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
        let request = KernelRuntimeBinaryRequest::from_envelope(&envelope, &self.layout);
        let foundation_binary = self.foundation_binary_path.clone();
        let kernel_binary = self.kernel_binary_path.clone();

        if !foundation_binary.exists() {
            return self.failure_invocation(
                request,
                &envelope,
                KernelRuntimeBinaryErrorKind::FoundationBinaryMissing,
                "foundation_runtime_binary_missing",
                &format!(
                    "Foundation runtime binary missing: {}",
                    foundation_binary.display()
                ),
                false,
                None,
            );
        }

        if !is_executable_file(&foundation_binary) {
            return self.failure_invocation(
                request,
                &envelope,
                KernelRuntimeBinaryErrorKind::FoundationBinaryNotExecutable,
                "foundation_runtime_binary_not_executable",
                &format!(
                    "Foundation runtime binary is not executable: {}",
                    foundation_binary.display()
                ),
                false,
                None,
            );
        }

        if !kernel_binary.exists() {
            return self.failure_invocation(
                request,
                &envelope,
                KernelRuntimeBinaryErrorKind::KernelBinaryMissing,
                "kernel_runtime_binary_missing",
                &format!("Kernel runtime binary missing: {}", kernel_binary.display()),
                false,
                None,
            );
        }

        if !is_executable_file(&kernel_binary) {
            return self.failure_invocation(
                request,
                &envelope,
                KernelRuntimeBinaryErrorKind::KernelBinaryNotExecutable,
                "kernel_runtime_binary_not_executable",
                &format!(
                    "Kernel runtime binary is not executable: {}",
                    kernel_binary.display()
                ),
                false,
                None,
            );
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
                return self.failure_invocation(
                    request,
                    &envelope,
                    KernelRuntimeBinaryErrorKind::ProcessSpawnFailed,
                    "kernel_runtime_process_spawn",
                    &format!("Kernel runtime binary spawn failed: {error}"),
                    false,
                    None,
                );
            }
        };

        if let Some(stdin) = child.stdin.as_mut() {
            if let Err(error) = writeln!(stdin, "{}", request.to_json()) {
                let _ = child.kill();
                return self.failure_invocation(
                    request,
                    &envelope,
                    KernelRuntimeBinaryErrorKind::StdinWriteFailed,
                    "kernel_runtime_ipc_write",
                    &format!("Kernel runtime binary stdin write failed: {error}"),
                    true,
                    None,
                );
            }
        }
        drop(child.stdin.take());

        let output = match child.wait_with_output() {
            Ok(output) => output,
            Err(error) => {
                return self.failure_invocation(
                    request,
                    &envelope,
                    KernelRuntimeBinaryErrorKind::StdoutReadFailed,
                    "kernel_runtime_ipc_read",
                    &format!("Kernel runtime binary output read failed: {error}"),
                    true,
                    None,
                );
            }
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        match parse_kernel_invocation_result_response(&stdout, output.status.code()) {
            Ok(mut response) => {
                let mut payload = response.payload.clone();
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
                add_runtime_binary_contract_metadata(&mut payload);
                response.payload = payload.clone();
                let invocation_failed =
                    payload.get("status").and_then(|value| value.as_str()) == Some("failed");
                return KernelRuntimeBinaryInvocation {
                    request,
                    response: Some(response),
                    payload,
                    exit_success: output.status.success(),
                    error: invocation_failed.then(|| KernelRuntimeBinaryError {
                        kind: KernelRuntimeBinaryErrorKind::KernelInvocationFailed,
                        stage: "kernel_invocation_failed".to_string(),
                        message: "Kernel runtime returned invocation failure".to_string(),
                        exit_code: output.status.code(),
                    }),
                };
            }
            Err(KernelRuntimeBinaryErrorKind::ProtocolVersionMismatch) => {
                return self.failure_invocation(
                    request,
                    &envelope,
                    KernelRuntimeBinaryErrorKind::ProtocolVersionMismatch,
                    "kernel_runtime_protocol_version_mismatch",
                    "Kernel runtime binary protocol version mismatch",
                    true,
                    output.status.code(),
                );
            }
            Err(KernelRuntimeBinaryErrorKind::ContractVersionMismatch) => {
                return self.failure_invocation(
                    request,
                    &envelope,
                    KernelRuntimeBinaryErrorKind::ContractVersionMismatch,
                    "kernel_runtime_contract_version_mismatch",
                    "Kernel runtime binary contract version mismatch",
                    true,
                    output.status.code(),
                );
            }
            Err(_) => {}
        }

        let stderr = String::from_utf8_lossy(&output.stderr);
        if !output.status.success() {
            return self.failure_invocation(
                request,
                &envelope,
                KernelRuntimeBinaryErrorKind::NonZeroExit,
                "kernel_runtime_non_zero_exit",
                &format!(
                    "Kernel runtime binary exited with {:?}; stderr={}",
                    output.status.code(),
                    sanitize_runtime_binary_diagnostic(&stderr)
                ),
                true,
                output.status.code(),
            );
        }

        self.failure_invocation(
            request,
            &envelope,
            KernelRuntimeBinaryErrorKind::InvalidJsonlOutput,
            "kernel_runtime_invalid_jsonl_output",
            &format!(
                "Kernel runtime binary returned no valid invocation result; stderr={}",
                sanitize_runtime_binary_diagnostic(&stderr)
            ),
            true,
            output.status.code(),
        )
    }

    fn failure_invocation(
        &self,
        request: KernelRuntimeBinaryRequest,
        envelope: &KernelInvocationEnvelope,
        kind: KernelRuntimeBinaryErrorKind,
        stage: &str,
        reason: &str,
        spawned_process: bool,
        process_exit_code: Option<i32>,
    ) -> KernelRuntimeBinaryInvocation {
        KernelRuntimeBinaryInvocation {
            response: None,
            payload: runtime_binary_failure_payload(
                &envelope,
                stage,
                reason,
                &self.layout,
                &self.foundation_binary_path,
                &self.kernel_binary_path,
                false,
                spawned_process,
                process_exit_code,
            ),
            exit_success: false,
            request,
            error: Some(KernelRuntimeBinaryError {
                kind,
                stage: stage.to_string(),
                message: sanitize_runtime_binary_diagnostic(reason),
                exit_code: process_exit_code,
            }),
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
    let mut fields = result
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
    if via_runtime_binary {
        add_runtime_binary_contract_metadata_to_fields(&mut fields);
    }

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

fn parse_kernel_invocation_result_response(
    stdout: &str,
    exit_code: Option<i32>,
) -> Result<KernelRuntimeBinaryResponse, KernelRuntimeBinaryErrorKind> {
    stdout
        .lines()
        .find_map(|line| {
            let value: serde_json::Value = serde_json::from_str(line).ok()?;
            if value.get("event").and_then(|event| event.as_str())
                == Some("kernel.invocation.result")
            {
                let protocol = value
                    .get("protocol")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default();
                let protocol_version = value
                    .get("protocol_version")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default();
                let contract_version = value
                    .get("contract_version")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default();
                if protocol != RUNTIME_BINARY_PROTOCOL
                    || protocol_version != RUNTIME_BINARY_PROTOCOL_VERSION
                {
                    return Some(Err(KernelRuntimeBinaryErrorKind::ProtocolVersionMismatch));
                }
                if contract_version != RUNTIME_BINARY_CONTRACT_VERSION {
                    return Some(Err(KernelRuntimeBinaryErrorKind::ContractVersionMismatch));
                }
                return Some(Ok(KernelRuntimeBinaryResponse {
                    protocol: protocol.to_string(),
                    protocol_version: protocol_version.to_string(),
                    contract_version: contract_version.to_string(),
                    payload: value
                        .get("payload")
                        .cloned()
                        .unwrap_or(serde_json::Value::Null),
                    exit_code,
                }));
            }
            None
        })
        .unwrap_or(Err(KernelRuntimeBinaryErrorKind::InvalidJsonlOutput))
}

fn add_runtime_binary_contract_metadata(payload: &mut serde_json::Value) {
    if let Some(result_fields) = payload
        .get_mut("result")
        .and_then(|result| result.get_mut("fields"))
        .and_then(|fields| fields.as_object_mut())
    {
        insert_runtime_binary_contract_metadata(result_fields);
    }
}

fn add_runtime_binary_contract_metadata_to_fields(fields: &mut serde_json::Value) {
    if let Some(fields) = fields.as_object_mut() {
        insert_runtime_binary_contract_metadata(fields);
    }
}

fn insert_runtime_binary_contract_metadata(
    fields: &mut serde_json::Map<String, serde_json::Value>,
) {
    fields.insert(
        "protocol".to_string(),
        serde_json::Value::String(RUNTIME_BINARY_PROTOCOL.to_string()),
    );
    fields.insert(
        "protocol_version".to_string(),
        serde_json::Value::String(RUNTIME_BINARY_PROTOCOL_VERSION.to_string()),
    );
    fields.insert(
        "runtime_binary_contract_version".to_string(),
        serde_json::Value::String(RUNTIME_BINARY_CONTRACT_VERSION.to_string()),
    );
    fields.insert(
        "binary_health".to_string(),
        serde_json::Value::String("ok".to_string()),
    );
}

fn runtime_binary_failure_payload(
    envelope: &KernelInvocationEnvelope,
    stage: &str,
    reason: &str,
    layout: &AicoreLayout,
    foundation_binary: &std::path::Path,
    kernel_binary: &std::path::Path,
    in_process_fallback: bool,
    spawned_process: bool,
    process_exit_code: Option<i32>,
) -> serde_json::Value {
    let foundation_health = binary_health(foundation_binary);
    let kernel_health = binary_health(kernel_binary);
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
            "foundation_health": foundation_health,
            "kernel_path": kernel_binary.display().to_string(),
            "kernel_installed": kernel_binary.exists(),
            "kernel_health": kernel_health,
            "protocol": RUNTIME_BINARY_PROTOCOL,
            "protocol_version": RUNTIME_BINARY_PROTOCOL_VERSION,
            "contract_version": RUNTIME_BINARY_CONTRACT_VERSION,
            "in_process_fallback": in_process_fallback,
        }
    })
}

fn is_executable_file(path: &std::path::Path) -> bool {
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

fn binary_health(path: &std::path::Path) -> &'static str {
    if !path.exists() {
        "missing"
    } else if is_executable_file(path) {
        "ok"
    } else {
        "not_executable"
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_layout(name: &str) -> AicoreLayout {
        let root = std::env::temp_dir().join(format!(
            "aicore-kernel-runtime-binary-{name}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be monotonic")
                .as_nanos()
        ));
        if root.exists() {
            std::fs::remove_dir_all(&root).expect("temp root should be removable");
        }
        std::fs::create_dir_all(&root).expect("temp root should be creatable");
        AicoreLayout::new(root.join(".aicore"))
    }

    fn seed_foundation_binary(layout: &AicoreLayout) {
        seed_executable(
            &layout.bin_root.join(FOUNDATION_RUNTIME_BINARY_NAME),
            "#!/bin/sh\necho foundation-ok\n",
        );
    }

    fn seed_kernel_binary(layout: &AicoreLayout, script: &str) {
        seed_executable(&layout.bin_root.join(KERNEL_RUNTIME_BINARY_NAME), script);
    }

    fn seed_non_executable_kernel_binary(layout: &AicoreLayout) {
        let path = layout.bin_root.join(KERNEL_RUNTIME_BINARY_NAME);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("kernel binary parent should be creatable");
        }
        std::fs::write(&path, "#!/bin/sh\necho should-not-run\n")
            .expect("kernel binary should be writable");
        #[cfg(unix)]
        {
            let mut permissions = std::fs::metadata(&path)
                .expect("metadata should exist")
                .permissions();
            permissions.set_mode(0o644);
            std::fs::set_permissions(&path, permissions).expect("permissions should be settable");
        }
    }

    fn seed_executable(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("binary parent should be creatable");
        }
        std::fs::write(path, content).expect("binary should be writable");
        #[cfg(unix)]
        {
            let mut permissions = std::fs::metadata(path)
                .expect("binary metadata should exist")
                .permissions();
            permissions.set_mode(0o755);
            std::fs::set_permissions(path, permissions)
                .expect("binary permissions should be settable");
        }
    }

    fn fixture_success_script() -> &'static str {
        r#"#!/bin/sh
request=$(cat)
case "$request" in
  *super-secret-token*) echo "raw payload leaked" >&2 ;;
esac
printf '%s\n' '{"event":"kernel.invocation.result","schema_version":"aicore.kernel.runtime_binary.response.v1","protocol":"stdio_jsonl","protocol_version":"aicore.kernel.runtime_binary.stdio_jsonl.v1","contract_version":"kernel.runtime.v1","payload":{"invocation_id":"invoke.fixture","trace_id":"trace.default","operation":"runtime.status","status":"completed","route":{"component_id":"aicore","app_id":"aicore","capability_id":"runtime.status","contract_version":"kernel.app.v1"},"handler":{"kind":"kernel_runtime_binary","invocation_mode":"local_process","transport":"stdio_jsonl","process_exit_code":0,"executed":true,"event_generated":true,"spawned_process":true,"called_real_component":false,"first_party_in_process_adapter":false},"ledger":{"appended":true,"path":"fixture-ledger","records":5},"result":{"kind":"runtime.status","summary":"ok","fields":{"kernel_invocation_path":"binary","foundation_runtime_binary":"installed","kernel_runtime_binary":"installed","protocol":"stdio_jsonl","protocol_version":"aicore.kernel.runtime_binary.stdio_jsonl.v1","contract_version":"kernel.runtime.v1","binary_health":"ok"}},"failure":{"stage":null,"reason":null}}}'
"#
    }

    #[test]
    fn kernel_runtime_binary_client_reports_missing_binary() {
        let layout = temp_layout("missing-binary");
        seed_foundation_binary(&layout);

        let invocation = KernelRuntimeBinaryClient::new(layout).invoke_readonly("runtime.status");

        assert!(!invocation.exit_success);
        assert_eq!(
            invocation.error.as_ref().map(|error| &error.kind),
            Some(&KernelRuntimeBinaryErrorKind::KernelBinaryMissing)
        );
        assert_eq!(
            invocation.payload["failure"]["stage"],
            "kernel_runtime_binary_missing"
        );
        assert_eq!(
            invocation.payload["runtime_binary"]["in_process_fallback"],
            false
        );
    }

    #[test]
    fn kernel_runtime_binary_client_reports_non_executable_binary() {
        let layout = temp_layout("non-executable");
        seed_foundation_binary(&layout);
        seed_non_executable_kernel_binary(&layout);

        let invocation = KernelRuntimeBinaryClient::new(layout).invoke_readonly("runtime.status");

        assert!(!invocation.exit_success);
        assert_eq!(
            invocation.error.as_ref().map(|error| &error.kind),
            Some(&KernelRuntimeBinaryErrorKind::KernelBinaryNotExecutable)
        );
        assert_eq!(
            invocation.payload["failure"]["stage"],
            "kernel_runtime_binary_not_executable"
        );
    }

    #[test]
    fn kernel_runtime_binary_client_reports_spawn_failure() {
        let layout = temp_layout("spawn-failure");
        seed_foundation_binary(&layout);
        seed_kernel_binary(
            &layout,
            "#!/path/to/aicore/missing/interpreter\necho should-not-run\n",
        );

        let invocation = KernelRuntimeBinaryClient::new(layout).invoke_readonly("runtime.status");

        assert!(!invocation.exit_success);
        assert_eq!(
            invocation.error.as_ref().map(|error| &error.kind),
            Some(&KernelRuntimeBinaryErrorKind::ProcessSpawnFailed)
        );
        assert_eq!(
            invocation.payload["failure"]["stage"],
            "kernel_runtime_process_spawn"
        );
    }

    #[test]
    fn kernel_runtime_binary_client_reports_non_zero_exit() {
        let layout = temp_layout("non-zero-exit");
        seed_foundation_binary(&layout);
        seed_kernel_binary(&layout, "#!/bin/sh\necho broken >&2\nexit 7\n");

        let invocation = KernelRuntimeBinaryClient::new(layout).invoke_readonly("runtime.status");

        assert!(!invocation.exit_success);
        assert_eq!(
            invocation.error.as_ref().map(|error| &error.kind),
            Some(&KernelRuntimeBinaryErrorKind::NonZeroExit)
        );
        assert_eq!(
            invocation.payload["failure"]["stage"],
            "kernel_runtime_non_zero_exit"
        );
        assert_eq!(
            invocation.payload["handler"]["process_exit_code"],
            serde_json::Value::from(7)
        );
    }

    #[test]
    fn kernel_runtime_binary_client_reports_invalid_jsonl_output() {
        let layout = temp_layout("invalid-jsonl-output");
        seed_foundation_binary(&layout);
        seed_kernel_binary(&layout, "#!/bin/sh\necho 'not json'\n");

        let invocation = KernelRuntimeBinaryClient::new(layout).invoke_readonly("runtime.status");

        assert!(!invocation.exit_success);
        assert_eq!(
            invocation.error.as_ref().map(|error| &error.kind),
            Some(&KernelRuntimeBinaryErrorKind::InvalidJsonlOutput)
        );
        assert_eq!(
            invocation.payload["failure"]["stage"],
            "kernel_runtime_invalid_jsonl_output"
        );
    }

    #[test]
    fn kernel_runtime_binary_client_reports_protocol_version_mismatch() {
        let layout = temp_layout("protocol-version-mismatch");
        seed_foundation_binary(&layout);
        seed_kernel_binary(
            &layout,
            r#"#!/bin/sh
cat >/dev/null
cat <<'JSON'
{"event":"kernel.invocation.result","schema_version":"aicore.kernel.runtime_binary.response.v1","protocol":"stdio_jsonl","protocol_version":"wrong.version","contract_version":"kernel.runtime.v1","payload":{"status":"completed"}}
JSON
"#,
        );

        let invocation = KernelRuntimeBinaryClient::new(layout).invoke_readonly("runtime.status");

        assert!(!invocation.exit_success);
        assert_eq!(
            invocation.error.as_ref().map(|error| &error.kind),
            Some(&KernelRuntimeBinaryErrorKind::ProtocolVersionMismatch)
        );
        assert_eq!(
            invocation.payload["failure"]["stage"],
            "kernel_runtime_protocol_version_mismatch"
        );
    }

    #[test]
    fn kernel_runtime_binary_client_does_not_fallback_to_in_process() {
        let layout = temp_layout("no-fallback");
        seed_foundation_binary(&layout);

        let invocation = KernelRuntimeBinaryClient::new(layout).invoke_readonly("runtime.status");

        assert!(!invocation.exit_success);
        assert_eq!(
            invocation.payload["runtime_binary"]["in_process_fallback"],
            false
        );
        assert_ne!(invocation.payload["status"], "completed");
    }

    #[test]
    fn invocation_ledger_does_not_record_raw_runtime_protocol_payload() {
        let layout = temp_layout("raw-protocol-payload");
        seed_foundation_binary(&layout);
        seed_kernel_binary(&layout, fixture_success_script());

        let envelope = KernelInvocationEnvelope::new(
            "global-main",
            "runtime.status",
            "runtime.status",
            KernelPayload::Text("super-secret-token".to_string()),
        );
        let invocation = KernelRuntimeBinaryClient::new(layout.clone()).invoke_envelope(envelope);

        assert!(invocation.exit_success);
        let ledger_path = layout.kernel_state_root.join("invocation-ledger.jsonl");
        let ledger = std::fs::read_to_string(ledger_path).unwrap_or_default();
        assert!(!ledger.contains("super-secret-token"));
        assert!(!ledger.contains("raw payload leaked"));
    }

    #[test]
    fn runtime_binary_failure_does_not_expose_secret_like_output() {
        let layout = temp_layout("redact-stderr");
        seed_foundation_binary(&layout);
        seed_kernel_binary(
            &layout,
            "#!/bin/sh\necho 'token=super-secret-token api_key=abc123' >&2\nexit 7\n",
        );

        let invocation = KernelRuntimeBinaryClient::new(layout).invoke_readonly("runtime.status");
        let failure = invocation.payload["failure"]["reason"]
            .as_str()
            .expect("failure reason should be a string");

        assert!(!failure.contains("super-secret-token"));
        assert!(!failure.contains("abc123"));
        assert!(failure.contains("[redacted:failure_reason]"));
    }

    #[allow(dead_code)]
    fn _assert_paths_are_send_sync(_: PathBuf) {}
}
