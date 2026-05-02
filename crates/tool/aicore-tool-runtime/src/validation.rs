use aicore_tool_protocol::*;

use crate::InMemoryToolRegistry;

pub fn validate_tool_call(
    registry: &InMemoryToolRegistry,
    request: &ToolCallValidationRequest,
) -> ToolCallValidationOutcome {
    let tool_call_id = request.call.tool_call_id.clone();
    let tool_id = &request.call.tool_id;

    if is_forbidden_tool_id(tool_id.as_str()) {
        return ToolCallValidationOutcome::failed(
            tool_call_id,
            ToolValidationFailureCode::ForbiddenTool,
        );
    }

    let entry = match registry.get_tool(tool_id) {
        Some(entry) => entry,
        None => {
            return ToolCallValidationOutcome::failed(
                tool_call_id,
                ToolValidationFailureCode::ToolNotFound,
            );
        }
    };

    match entry.status {
        ToolStatus::Enabled => {}
        ToolStatus::Disabled => {
            return ToolCallValidationOutcome::failed(
                tool_call_id,
                ToolValidationFailureCode::ToolDisabled,
            );
        }
        ToolStatus::Removed => {
            return ToolCallValidationOutcome::failed(
                tool_call_id,
                ToolValidationFailureCode::ToolRemoved,
            );
        }
        ToolStatus::Broken | ToolStatus::Updating | ToolStatus::Installed => {
            return ToolCallValidationOutcome::failed(
                tool_call_id,
                ToolValidationFailureCode::ToolBroken,
            );
        }
    }

    if entry.permission_class == ToolPermissionClass::Forbidden
        || entry.approval_requirement == ToolApprovalRequirement::Forbidden
    {
        return ToolCallValidationOutcome::failed(
            tool_call_id,
            ToolValidationFailureCode::ForbiddenTool,
        );
    }

    if request.call.schema_hash != entry.schema.schema_hash {
        return ToolCallValidationOutcome::failed(
            tool_call_id,
            ToolValidationFailureCode::SchemaHashMismatch,
        );
    }

    if request.call.args_digest != request.computed_args_digest {
        return ToolCallValidationOutcome::failed(
            tool_call_id,
            ToolValidationFailureCode::ArgsDigestMismatch,
        );
    }

    if request.call.lock_version != entry.lock_version {
        return ToolCallValidationOutcome::failed(
            tool_call_id,
            ToolValidationFailureCode::LockVersionMismatch,
        );
    }

    if request.call.sandbox_profile_id != entry.sandbox_policy.sandbox_profile_id {
        return ToolCallValidationOutcome::failed(
            tool_call_id,
            ToolValidationFailureCode::SandboxDenied,
        );
    }

    if matches!(
        entry.permission_class,
        ToolPermissionClass::Privileged | ToolPermissionClass::Destructive
    ) {
        return ToolCallValidationOutcome::failed(
            tool_call_id,
            ToolValidationFailureCode::SandboxDenied,
        );
    }

    match request.sandbox_decision.kind {
        ToolSandboxDecisionKind::Allow => {}
        ToolSandboxDecisionKind::RequiresApproval => {
            if request.approval_binding.is_none() {
                return ToolCallValidationOutcome::approval_required(tool_call_id);
            }
        }
        ToolSandboxDecisionKind::Deny => {
            return ToolCallValidationOutcome::failed(
                tool_call_id,
                ToolValidationFailureCode::SandboxDenied,
            );
        }
        ToolSandboxDecisionKind::Forbidden => {
            return ToolCallValidationOutcome::failed(
                tool_call_id,
                ToolValidationFailureCode::ForbiddenTool,
            );
        }
    }

    if entry.approval_requirement == ToolApprovalRequirement::Required
        && request.approval_binding.is_none()
    {
        return ToolCallValidationOutcome::approval_required(tool_call_id);
    }

    if let Some(binding) = &request.approval_binding {
        if binding.approval_scope != ToolApprovalScope::SingleToolCall {
            return ToolCallValidationOutcome::failed(
                tool_call_id,
                ToolValidationFailureCode::ApprovalScopeInvalid,
            );
        }
        let binding_matches = binding.approved
            && binding.tool_call_id == request.call.tool_call_id
            && binding.tool_id == request.call.tool_id
            && binding.schema_hash == request.call.schema_hash
            && binding.args_digest == request.call.args_digest
            && binding.sandbox_profile_id == request.call.sandbox_profile_id
            && binding.lock_version == request.call.lock_version
            && binding.target_instance_id == request.call.instance_id
            && binding.turn_id == request.call.turn_id;
        if !binding_matches {
            return ToolCallValidationOutcome::failed(
                tool_call_id,
                ToolValidationFailureCode::ApprovalBindingMismatch,
            );
        }
    }

    if request.stop_requested {
        return ToolCallValidationOutcome::failed(
            tool_call_id,
            ToolValidationFailureCode::TurnStopped,
        );
    }

    if !request.turn_is_active {
        return ToolCallValidationOutcome::failed(
            tool_call_id,
            ToolValidationFailureCode::TurnStale,
        );
    }

    ToolCallValidationOutcome {
        tool_call_id: tool_call_id.clone(),
        status: ToolCallStatus::Approved,
        failure_code: None,
        execution_summary: Some(ToolExecutionSummary {
            tool_call_id,
            status: ToolCallStatus::ExecutionSkipped,
            summary_en: "Validation passed; no real tool execution performed.".to_string(),
            summary_zh: Some("校验通过；未执行真实工具。".to_string()),
            did_execute_external_operation: false,
            redaction_applied: true,
            output_truncated: false,
            sensitive_content_omitted: true,
        }),
    }
}

pub fn validate_live_registry_status(
    registry: &InMemoryToolRegistry,
    tool_id: &ToolId,
) -> Option<ToolValidationFailureCode> {
    let entry = registry.get_tool(tool_id)?;
    match entry.status {
        ToolStatus::Enabled => None,
        ToolStatus::Disabled => Some(ToolValidationFailureCode::ToolDisabled),
        ToolStatus::Removed => Some(ToolValidationFailureCode::ToolRemoved),
        ToolStatus::Broken | ToolStatus::Updating | ToolStatus::Installed => {
            Some(ToolValidationFailureCode::ToolBroken)
        }
    }
}

pub fn validate_schema_hash(call: &ToolCallEnvelope, entry: &ToolRegistryEntry) -> bool {
    call.schema_hash == entry.schema.schema_hash
}

pub fn validate_args_digest(request: &ToolCallValidationRequest) -> bool {
    request.call.args_digest == request.computed_args_digest
}

pub fn validate_lock_version(call: &ToolCallEnvelope, entry: &ToolRegistryEntry) -> bool {
    call.lock_version == entry.lock_version
}

pub fn validate_approval_binding(request: &ToolCallValidationRequest) -> bool {
    let Some(binding) = &request.approval_binding else {
        return false;
    };
    binding.approved
        && binding.approval_scope == ToolApprovalScope::SingleToolCall
        && binding.tool_call_id == request.call.tool_call_id
        && binding.tool_id == request.call.tool_id
        && binding.schema_hash == request.call.schema_hash
        && binding.args_digest == request.call.args_digest
        && binding.sandbox_profile_id == request.call.sandbox_profile_id
        && binding.lock_version == request.call.lock_version
        && binding.target_instance_id == request.call.instance_id
        && binding.turn_id == request.call.turn_id
}

pub fn validate_sandbox_decision(request: &ToolCallValidationRequest) -> bool {
    request.sandbox_decision.kind == ToolSandboxDecisionKind::Allow
        && request.sandbox_decision.sandbox_profile_id == request.call.sandbox_profile_id
}

pub fn validate_stop_state(
    request: &ToolCallValidationRequest,
) -> Option<ToolValidationFailureCode> {
    if request.stop_requested {
        Some(ToolValidationFailureCode::TurnStopped)
    } else if !request.turn_is_active {
        Some(ToolValidationFailureCode::TurnStale)
    } else {
        None
    }
}
