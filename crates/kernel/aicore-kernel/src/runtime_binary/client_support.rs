use std::path::Path;

use aicore_foundation::AicoreLayout;

use crate::KernelInvocationEnvelope;

use super::error::{
    KernelRuntimeBinaryError, KernelRuntimeBinaryErrorKind, KernelRuntimeBinaryInvocation,
    runtime_binary_failure_payload, sanitize_runtime_binary_diagnostic,
};
use super::public_json::add_runtime_binary_contract_metadata;
use super::request::KernelRuntimeBinaryRequest;
use super::response::KernelRuntimeBinaryResponse;

pub(super) fn response_payload(
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

pub(super) fn failure_invocation(
    request: KernelRuntimeBinaryRequest,
    envelope: &KernelInvocationEnvelope,
    layout: &AicoreLayout,
    foundation_binary_path: &Path,
    kernel_binary_path: &Path,
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
            layout,
            foundation_binary_path,
            kernel_binary_path,
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
