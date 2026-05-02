use aicore_foundation::{InstanceId, SessionId, Timestamp, TurnId};
use aicore_tool_protocol::*;
use aicore_tool_runtime::*;

fn now(value: u128) -> Timestamp {
    Timestamp::from_unix_millis(value)
}

fn tool_id() -> ToolId {
    ToolId::new("tool.git.status").unwrap()
}

fn sandbox_id() -> SandboxProfileId {
    SandboxProfileId::new("readonly_workspace").unwrap()
}

fn safe_entry(status: ToolStatus, approval: ToolApprovalRequirement) -> ToolRegistryEntry {
    ToolRegistryEntry {
        descriptor: ToolDescriptor {
            tool_id: tool_id(),
            module_id: ToolModuleId::new("module.git").unwrap(),
            version: ToolVersion::new("v1").unwrap(),
            name: "git_status".to_string(),
            description_en: "Summarize current git status.".to_string(),
            description_zh: Some("查看 Git 状态".to_string()),
        },
        schema: ToolSchemaDescriptor {
            input_schema_summary: "{}".to_string(),
            output_schema_summary: "summary".to_string(),
            schema_hash: ToolSchemaHash::new("schema.git.status.v1").unwrap(),
        },
        status,
        permission_class: ToolPermissionClass::SafeRead,
        approval_requirement: approval,
        sandbox_policy: ToolSandboxPolicy::readonly(sandbox_id()),
        lock_version: 1,
        registry_revision: 0,
        updated_at: now(1),
    }
}

fn registry_with(entry: ToolRegistryEntry) -> InMemoryToolRegistry {
    let mut registry = InMemoryToolRegistry::new();
    registry.register_tool(entry, now(2)).unwrap();
    registry
}

fn call() -> ToolCallEnvelope {
    ToolCallEnvelope {
        instance_id: InstanceId::new("workspace.demo").unwrap(),
        session_id: Some(SessionId::new("session.1").unwrap()),
        turn_id: TurnId::new("turn.1").unwrap(),
        tool_call_id: ToolCallId::new("toolcall.1").unwrap(),
        tool_id: tool_id(),
        schema_hash: ToolSchemaHash::new("schema.git.status.v1").unwrap(),
        args_digest: ToolArgsDigest::new("args.empty.v1").unwrap(),
        registry_revision: 1,
        lock_version: 1,
        sandbox_profile_id: sandbox_id(),
        proposed_at: now(3),
    }
}

fn validation_request() -> ToolCallValidationRequest {
    ToolCallValidationRequest {
        call: call(),
        computed_args_digest: ToolArgsDigest::new("args.empty.v1").unwrap(),
        approval_binding: None,
        sandbox_decision: ToolSandboxDecision::allow(sandbox_id()),
        turn_is_active: true,
        stop_requested: false,
    }
}

fn approval_for(call: &ToolCallEnvelope) -> ToolApprovalBinding {
    ToolApprovalBinding {
        approval_id: "approval.1".to_string(),
        approval_scope: ToolApprovalScope::SingleToolCall,
        tool_call_id: call.tool_call_id.clone(),
        tool_id: call.tool_id.clone(),
        schema_hash: call.schema_hash.clone(),
        args_digest: call.args_digest.clone(),
        sandbox_profile_id: call.sandbox_profile_id.clone(),
        lock_version: call.lock_version,
        target_instance_id: call.instance_id.clone(),
        turn_id: call.turn_id.clone(),
        approved: true,
    }
}

#[test]
fn register_enabled_safe_read_tool_and_project_visible_capabilities() {
    let mut registry = InMemoryToolRegistry::new();
    let notice = registry
        .register_tool(
            safe_entry(ToolStatus::Enabled, ToolApprovalRequirement::NotRequired),
            now(10),
        )
        .unwrap();
    assert_eq!(registry.revision(), 1);
    assert_eq!(notice.remaining_turns, 3);

    let projection =
        registry.project_visible_capabilities(InstanceId::new("workspace.demo").unwrap(), now(11));
    assert_eq!(projection.registry_revision, 1);
    assert_eq!(projection.capabilities.len(), 1);
    assert_eq!(projection.capabilities[0].tool_id, tool_id());
}

#[test]
fn disabled_removed_and_broken_tools_are_not_visible() {
    for status in [
        ToolStatus::Disabled,
        ToolStatus::Removed,
        ToolStatus::Broken,
    ] {
        let registry = registry_with(safe_entry(status, ToolApprovalRequirement::NotRequired));
        let projection = registry
            .project_visible_capabilities(InstanceId::new("workspace.demo").unwrap(), now(20));
        assert!(projection.capabilities.is_empty());
    }
}

#[test]
fn forbidden_tool_register_fails_closed() {
    let mut entry = safe_entry(ToolStatus::Enabled, ToolApprovalRequirement::NotRequired);
    entry.descriptor.tool_id = ToolId::new("event_query").unwrap();
    entry.permission_class = ToolPermissionClass::Forbidden;

    let mut registry = InMemoryToolRegistry::new();
    assert_eq!(
        registry.register_tool(entry, now(30)).unwrap_err(),
        ToolRuntimeError::ForbiddenTool("event_query".to_string())
    );
}

#[test]
fn snapshot_revision_increments_for_hot_plug_changes() {
    let mut registry = registry_with(safe_entry(
        ToolStatus::Enabled,
        ToolApprovalRequirement::NotRequired,
    ));
    assert_eq!(registry.snapshot(now(40)).revision, 1);
    registry.disable_tool(&tool_id(), now(41)).unwrap();
    assert_eq!(registry.snapshot(now(42)).revision, 2);
    registry.enable_tool(&tool_id(), now(43)).unwrap();
    assert_eq!(registry.snapshot(now(44)).revision, 3);
}

#[test]
fn valid_safe_read_call_passes_validation_without_real_execution() {
    let registry = registry_with(safe_entry(
        ToolStatus::Enabled,
        ToolApprovalRequirement::NotRequired,
    ));
    let outcome = validate_tool_call(&registry, &validation_request());
    assert_eq!(outcome.status, ToolCallStatus::Approved);
    let summary = outcome.execution_summary.unwrap();
    assert!(!summary.did_execute_external_operation);
    assert_eq!(summary.status, ToolCallStatus::ExecutionSkipped);
}

#[test]
fn validation_rejects_disabled_removed_broken_and_forbidden_tools() {
    for (status, code) in [
        (
            ToolStatus::Disabled,
            ToolValidationFailureCode::ToolDisabled,
        ),
        (ToolStatus::Removed, ToolValidationFailureCode::ToolRemoved),
        (ToolStatus::Broken, ToolValidationFailureCode::ToolBroken),
    ] {
        let registry = registry_with(safe_entry(status, ToolApprovalRequirement::NotRequired));
        let outcome = validate_tool_call(&registry, &validation_request());
        assert_eq!(outcome.failure_code, Some(code));
    }

    let registry = InMemoryToolRegistry::new();
    let mut request = validation_request();
    request.call.tool_id = ToolId::new("ledger_query").unwrap();
    let outcome = validate_tool_call(&registry, &request);
    assert_eq!(
        outcome.failure_code,
        Some(ToolValidationFailureCode::ForbiddenTool)
    );
}

#[test]
fn validation_checks_schema_args_and_lock_version() {
    let registry = registry_with(safe_entry(
        ToolStatus::Enabled,
        ToolApprovalRequirement::NotRequired,
    ));

    let mut request = validation_request();
    request.call.schema_hash = ToolSchemaHash::new("schema.old").unwrap();
    assert_eq!(
        validate_tool_call(&registry, &request).failure_code,
        Some(ToolValidationFailureCode::SchemaHashMismatch)
    );

    let mut request = validation_request();
    request.computed_args_digest = ToolArgsDigest::new("args.changed").unwrap();
    assert_eq!(
        validate_tool_call(&registry, &request).failure_code,
        Some(ToolValidationFailureCode::ArgsDigestMismatch)
    );

    let mut request = validation_request();
    request.call.lock_version = 9;
    assert_eq!(
        validate_tool_call(&registry, &request).failure_code,
        Some(ToolValidationFailureCode::LockVersionMismatch)
    );
}

#[test]
fn approval_required_and_binding_mismatch_are_enforced() {
    let registry = registry_with(safe_entry(
        ToolStatus::Enabled,
        ToolApprovalRequirement::Required,
    ));
    let mut request = validation_request();
    assert_eq!(
        validate_tool_call(&registry, &request).status,
        ToolCallStatus::ApprovalRequired
    );

    let mut binding = approval_for(&request.call);
    binding.tool_id = ToolId::new("tool.other").unwrap();
    request.approval_binding = Some(binding);
    assert_eq!(
        validate_tool_call(&registry, &request).failure_code,
        Some(ToolValidationFailureCode::ApprovalBindingMismatch)
    );

    let mut request = validation_request();
    request.approval_binding = Some(approval_for(&request.call));
    assert_eq!(
        validate_tool_call(&registry, &request).status,
        ToolCallStatus::Approved
    );
}

#[test]
fn sandbox_denied_wins_even_with_approval() {
    let registry = registry_with(safe_entry(
        ToolStatus::Enabled,
        ToolApprovalRequirement::Required,
    ));
    let mut request = validation_request();
    request.approval_binding = Some(approval_for(&request.call));
    request.sandbox_decision = ToolSandboxDecision::deny(sandbox_id(), "policy_denied");

    assert_eq!(
        validate_tool_call(&registry, &request).failure_code,
        Some(ToolValidationFailureCode::SandboxDenied)
    );
}

#[test]
fn stopped_or_stale_turn_fails_validation() {
    let registry = registry_with(safe_entry(
        ToolStatus::Enabled,
        ToolApprovalRequirement::NotRequired,
    ));

    let mut request = validation_request();
    request.stop_requested = true;
    assert_eq!(
        validate_tool_call(&registry, &request).failure_code,
        Some(ToolValidationFailureCode::TurnStopped)
    );

    let mut request = validation_request();
    request.turn_is_active = false;
    assert_eq!(
        validate_tool_call(&registry, &request).failure_code,
        Some(ToolValidationFailureCode::TurnStale)
    );
}

#[test]
fn hot_plug_notice_advances_and_does_not_change_registry_or_authorize_tool() {
    let mut registry = registry_with(safe_entry(
        ToolStatus::Enabled,
        ToolApprovalRequirement::NotRequired,
    ));
    let before = registry.snapshot(now(60)).revision;
    let mut notice = registry.disable_tool(&tool_id(), now(61)).unwrap();
    assert_eq!(notice.remaining_turns, 3);
    assert!(notice.message_en.contains("not available"));
    notice.advance_one_turn();
    notice.advance_one_turn();
    notice.advance_one_turn();
    assert!(notice.is_expired());
    assert_eq!(registry.snapshot(now(62)).revision, before + 1);

    let outcome = validate_tool_call(&registry, &validation_request());
    assert_eq!(
        outcome.failure_code,
        Some(ToolValidationFailureCode::ToolDisabled)
    );
}

#[test]
fn source_has_no_execution_or_query_entrypoints() {
    let manifest = std::fs::read_to_string("Cargo.toml").unwrap();
    assert!(!manifest.contains("reqwest"));
    assert!(!manifest.contains("hyper"));
    assert!(!manifest.contains("tokio"));
    assert!(!manifest.contains("rusqlite"));

    let files = ["src/lib.rs", "src/registry.rs", "src/validation.rs"];
    let joined = files
        .iter()
        .map(|file| std::fs::read_to_string(file).unwrap())
        .collect::<Vec<_>>()
        .join("\n");
    for forbidden in [
        "shell_exec",
        "read_file(",
        "write_file(",
        "browser",
        "mcp_call",
        "event_query_tool",
        "ledger_query_tool",
        "session_ledger",
        "provider_sdk",
        "agent_runtime",
    ] {
        assert!(
            !joined.contains(forbidden),
            "unexpected non-goal symbol: {forbidden}"
        );
    }
}
