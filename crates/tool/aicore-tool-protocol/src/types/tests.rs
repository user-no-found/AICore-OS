use aicore_foundation::{InstanceId, Timestamp};

use super::*;

const FORBIDDEN_FIELDS: &[&str] = &[
    "raw_provider_request",
    "raw_provider_response",
    "raw_tool_input",
    "raw_tool_output",
    "raw_stdout",
    "raw_stderr",
    "raw_memory_content",
    "raw_prompt",
    "secret",
    "token",
    "api_key",
    "cookie",
    "credential",
    "authorization",
    "password",
];

fn safe_entry(status: ToolStatus) -> ToolRegistryEntry {
    ToolRegistryEntry {
        descriptor: ToolDescriptor {
            tool_id: ToolId::new("tool.git.status").unwrap(),
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
        approval_requirement: ToolApprovalRequirement::NotRequired,
        sandbox_policy: ToolSandboxPolicy::readonly(
            SandboxProfileId::new("readonly_workspace").unwrap(),
        ),
        lock_version: 1,
        registry_revision: 1,
        updated_at: Timestamp::from_unix_millis(1),
    }
}

#[test]
fn core_types_round_trip_through_serde() {
    let snapshot = ToolRegistrySnapshot {
        revision: 1,
        entries: vec![safe_entry(ToolStatus::Enabled)],
        created_at: Timestamp::from_unix_millis(2),
    };
    let encoded = serde_json::to_string(&snapshot).unwrap();
    assert!(encoded.contains("safe_read"));
    assert!(encoded.contains("enabled"));
    let decoded: ToolRegistrySnapshot = serde_json::from_str(&encoded).unwrap();
    assert_eq!(
        decoded.entries[0].permission_class,
        ToolPermissionClass::SafeRead
    );
}

#[test]
fn permission_class_values_are_contract_values() {
    let encoded = serde_json::to_string(&ToolPermissionClass::WorkspaceWrite).unwrap();
    assert_eq!(encoded, "\"workspace_write\"");
    let decoded: ToolPermissionClass = serde_json::from_str("\"command_exec\"").unwrap();
    assert_eq!(decoded, ToolPermissionClass::CommandExec);
}

#[test]
fn approval_scope_is_single_tool_call_only() {
    assert_eq!(
        ToolApprovalScope::from_contract_value("single_tool_call"),
        Some(ToolApprovalScope::SingleToolCall)
    );
    assert!(ToolApprovalScope::from_contract_value("session").is_none());
    assert!(ToolApprovalScope::from_contract_value("all_tools").is_none());
}

#[test]
fn forbidden_ids_are_fixed() {
    for id in [
        "event_query",
        "ledger_query",
        "self_evolution_query",
        "secret_read",
        "credential_export",
        "remote_deploy",
        "system_service_control",
        "destructive_git_auto_execution",
        "cross_instance_memory_search",
    ] {
        assert!(is_forbidden_tool_id(id));
    }
}

#[test]
fn visible_capability_filters_forbidden_tool() {
    let mut entry = safe_entry(ToolStatus::Enabled);
    entry.descriptor.tool_id = ToolId::new("event_query").unwrap();
    entry.permission_class = ToolPermissionClass::Forbidden;

    let snapshot = ToolRegistrySnapshot {
        revision: 1,
        entries: vec![entry],
        created_at: Timestamp::from_unix_millis(2),
    };
    let projection = VisibleCapabilitiesProjection::from_snapshot(
        InstanceId::new("workspace.demo").unwrap(),
        &snapshot,
        Timestamp::from_unix_millis(3),
    );
    assert!(projection.capabilities.is_empty());
}

#[test]
fn public_structures_do_not_expose_forbidden_fields() {
    let sample = serde_json::json!({
        "entry": safe_entry(ToolStatus::Enabled),
        "notice": ToolHotPlugNotice::new(
            ToolNoticeId::new("notice.tool.git.status.enabled.1").unwrap(),
            ToolId::new("tool.git.status").unwrap(),
            ToolHotPlugChangeKind::Enabled,
            Timestamp::from_unix_millis(4),
            "tool enabled",
            Some("工具已启用".to_string()),
        )
    });
    let encoded = serde_json::to_string(&sample).unwrap();
    for field in FORBIDDEN_FIELDS {
        assert!(!encoded.contains(field), "forbidden field leaked: {field}");
    }
}
