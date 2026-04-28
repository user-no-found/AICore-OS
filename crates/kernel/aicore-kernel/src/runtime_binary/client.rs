use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use aicore_foundation::AicoreLayout;

use crate::{KernelInvocationEnvelope, KernelPayload};

use super::error::{
    KernelRuntimeBinaryError, KernelRuntimeBinaryErrorKind, KernelRuntimeBinaryInvocation,
    runtime_binary_failure_payload, sanitize_runtime_binary_diagnostic,
};
use super::health::is_executable_file;
use super::protocol::{FOUNDATION_RUNTIME_BINARY_NAME, KERNEL_RUNTIME_BINARY_NAME};
use super::public_json::add_runtime_binary_contract_metadata;
use super::request::KernelRuntimeBinaryRequest;
use super::response::{KernelRuntimeBinaryResponse, parse_kernel_invocation_result_response};

#[derive(Debug, Clone)]
pub struct KernelRuntimeBinaryClient {
    layout: AicoreLayout,
    foundation_binary_path: PathBuf,
    kernel_binary_path: PathBuf,
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
                let payload = Self::response_payload(&mut response, output.status.code());
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

    fn response_payload(
        response: &mut KernelRuntimeBinaryResponse,
        process_exit_code: Option<i32>,
    ) -> serde_json::Value {
        let mut payload = response.payload.clone();
        if let Some(handler) = payload
            .get_mut("handler")
            .and_then(|value| value.as_object_mut())
        {
            handler.insert(
                "process_exit_code".to_string(),
                process_exit_code
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
        payload
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
                envelope,
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
