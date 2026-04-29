use aicore_memory::{MemoryPermanence, MemoryType, RememberInput};

use crate::config_store::{global_main_memory_scope, real_memory_kernel};
use crate::errors::memory_error;

pub(crate) fn build_memory_remember_write_report(
    content: &str,
) -> Result<(String, serde_json::Value), String> {
    if content.trim().is_empty() {
        return Err("memory.remember content 不能为空".to_string());
    }
    let mut kernel = real_memory_kernel()?;
    let memory_id = kernel
        .remember_user_explicit(RememberInput {
            memory_type: MemoryType::Core,
            permanence: MemoryPermanence::Standard,
            scope: global_main_memory_scope(),
            content: content.to_string(),
            localized_summary: content.to_string(),
            state_key: None,
            current_state: None,
        })
        .map_err(memory_error)?;

    Ok((
        "memory.remember write applied".to_string(),
        serde_json::json!({
            "operation": "memory.remember",
            "write_applied": "true",
            "audit_closed": "true",
            "write_outcome": "applied",
            "idempotency": "not_guaranteed",
            "memory_id": memory_id,
            "memory_type": "core",
            "source": "user_explicit",
            "permanence": "standard",
            "content_present": "true",
            "content_length": content.chars().count().to_string(),
            "kernel_invocation_path": "binary"
        }),
    ))
}

pub(crate) fn build_memory_accept_write_report(
    proposal_id: &str,
) -> Result<(String, serde_json::Value), String> {
    if proposal_id.trim().is_empty() {
        return Err("memory.accept proposal_id 不能为空".to_string());
    }
    let mut kernel = real_memory_kernel()?;
    let memory_id = kernel
        .accept_proposal(proposal_id, "user", Some("cli accept"))
        .map_err(memory_error)?;

    Ok((
        "memory.accept write applied".to_string(),
        serde_json::json!({
            "operation": "memory.accept",
            "write_applied": "true",
            "audit_closed": "true",
            "write_outcome": "applied",
            "idempotency": "not_guaranteed",
            "proposal_id": proposal_id,
            "memory_id": memory_id,
            "status": "accepted",
            "kernel_invocation_path": "binary"
        }),
    ))
}

pub(crate) fn build_memory_reject_write_report(
    proposal_id: &str,
) -> Result<(String, serde_json::Value), String> {
    if proposal_id.trim().is_empty() {
        return Err("memory.reject proposal_id 不能为空".to_string());
    }
    let mut kernel = real_memory_kernel()?;
    kernel
        .reject_proposal(proposal_id, "user", Some("cli reject"))
        .map_err(memory_error)?;

    Ok((
        "memory.reject write applied".to_string(),
        serde_json::json!({
            "operation": "memory.reject",
            "write_applied": "true",
            "audit_closed": "true",
            "write_outcome": "applied",
            "idempotency": "not_guaranteed",
            "proposal_id": proposal_id,
            "status": "rejected",
            "kernel_invocation_path": "binary"
        }),
    ))
}

pub(crate) fn memory_write_failure_fields(
    operation: &str,
    proposal_id: Option<String>,
    content_length: Option<usize>,
) -> serde_json::Value {
    let mut fields = serde_json::json!({
        "operation": operation,
        "write_applied": "false",
        "audit_closed": "true",
        "write_outcome": "failed",
        "idempotency": "not_guaranteed",
        "kernel_invocation_path": "binary"
    });
    let object = fields
        .as_object_mut()
        .expect("memory write failure fields should be an object");
    if let Some(proposal_id) = proposal_id.filter(|value| !value.trim().is_empty()) {
        object.insert(
            "proposal_id".to_string(),
            serde_json::Value::String(proposal_id),
        );
    }
    if let Some(content_length) = content_length {
        object.insert(
            "content_present".to_string(),
            serde_json::Value::String((content_length > 0).to_string()),
        );
        object.insert(
            "content_length".to_string(),
            serde_json::Value::String(content_length.to_string()),
        );
    }
    fields
}
