use aicore_foundation::Timestamp;
use aicore_mcp_adapter::*;
use aicore_tool_protocol::{
    SandboxProfileId, ToolApprovalRequirement, ToolId, ToolPermissionClass, ToolSchemaHash,
};

fn now(value: u128) -> Timestamp {
    Timestamp::from_unix_millis(value)
}

fn server_id() -> McpServerId {
    McpServerId::new("mcp.filesystem.demo").unwrap()
}

fn server(trust_level: McpTrustLevel) -> McpServerDescriptor {
    McpServerDescriptor {
        server_id: server_id(),
        display_name: "Filesystem Demo".to_string(),
        status: McpServerStatus::Discovered,
        trust_level,
        discovery_id: McpDiscoveryId::new("discovery.1").unwrap(),
        summary_en: "Candidate server descriptor.".to_string(),
        configured_by_user: false,
        created_at: now(1),
    }
}

fn candidate(status: McpToolCandidateStatus) -> McpToolCandidate {
    McpToolCandidate {
        server_id: server_id(),
        tool_name: McpToolName::new("read_file_summary").unwrap(),
        status,
        summary_en: "Summarize a file through an adapter boundary.".to_string(),
        input_schema_summary: "path ref".to_string(),
        output_schema_summary: "summary only".to_string(),
        prompt_visible: false,
        executable: false,
        discovered_at: now(2),
    }
}

#[test]
fn server_descriptor_round_trips_json() {
    let descriptor = server(McpTrustLevel::UserConfirmed);
    let json = serde_json::to_string(&descriptor).unwrap();
    let decoded: McpServerDescriptor = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded.server_id, server_id());
    assert_eq!(decoded.status, McpServerStatus::Discovered);
}

#[test]
fn discovered_candidate_is_not_executable_or_prompt_visible() {
    let candidate = McpToolCandidate::discovered(
        server_id(),
        McpToolName::new("search_docs").unwrap(),
        "Search docs candidate".to_string(),
        now(2),
    );
    assert!(!candidate.executable);
    assert!(!candidate.prompt_visible);
}

#[test]
fn mapping_expresses_tool_module_contract_without_runtime_registration() {
    let mapping = McpToolToAicoreToolMapping::new_candidate(
        McpMappingId::new("mapping.1").unwrap(),
        server(McpTrustLevel::UserConfirmed),
        candidate(McpToolCandidateStatus::Discovered),
        ToolId::new("tool.mcp.filesystem.read_file_summary").unwrap(),
        ToolSchemaHash::new("schema.mcp.read_file_summary.v1").unwrap(),
        SandboxProfileId::new("mcp.summary_only").unwrap(),
        now(3),
    )
    .unwrap();
    assert_eq!(mapping.status, McpMappingStatus::RegisteredToolModule);
    assert!(!mapping.registered_in_runtime);
    assert_eq!(mapping.permission_class, ToolPermissionClass::SafeRead);
    assert_eq!(
        mapping.approval_requirement,
        ToolApprovalRequirement::Required
    );
}

#[test]
fn unknown_mcp_tool_fails_closed() {
    let outcome = validate_mcp_tool_mapping(None, &server(McpTrustLevel::UserConfirmed));
    assert_eq!(outcome, McpMappingValidationOutcome::RejectedUnknownTool);
}

#[test]
fn untrusted_server_cannot_map_enabled_tool() {
    let outcome = validate_mcp_tool_mapping(
        Some(&candidate(McpToolCandidateStatus::Discovered)),
        &server(McpTrustLevel::Untrusted),
    );
    assert_eq!(
        outcome,
        McpMappingValidationOutcome::RejectedUntrustedServer
    );
}

#[test]
fn result_boundary_is_summary_or_ref_only() {
    let boundary = McpResultSummaryBoundary {
        mapping_id: McpMappingId::new("mapping.1").unwrap(),
        redaction_mode: McpRedactionMode::SummaryOnly,
        summary_en: "Operation returned a safe summary.".to_string(),
        artifact_ref: None,
        redaction_applied: true,
        payload_omitted: true,
    };
    assert!(boundary.payload_omitted);
    assert_eq!(boundary.redaction_mode, McpRedactionMode::SummaryOnly);
}

#[test]
fn permission_hook_requires_registry_approval_and_sandbox() {
    let hook = McpPermissionHook {
        mapping_id: McpMappingId::new("mapping.1").unwrap(),
        requires_tool_registry_registration: true,
        requires_approval: true,
        requires_sandbox: true,
        summary_en: "MCP-backed tools still pass through registry, approval, and sandbox."
            .to_string(),
    };
    assert!(hook.requires_tool_registry_registration);
    assert!(hook.requires_approval);
    assert!(hook.requires_sandbox);
}

#[test]
fn no_raw_mcp_payload_fields_are_serialized() {
    let report = McpDiscoveryReport {
        discovery_id: McpDiscoveryId::new("discovery.1").unwrap(),
        servers: vec![server(McpTrustLevel::UserConfirmed)],
        tool_candidates: vec![candidate(McpToolCandidateStatus::Discovered)],
        created_at: now(4),
    };
    let json = serde_json::to_string(&report).unwrap();
    for word in [
        "raw_provider_request",
        "raw_provider_response",
        "raw_tool_input",
        "raw_tool_output",
        "raw_stdout",
        "raw_stderr",
        "raw_memory_content",
        "raw_prompt",
        "raw_mcp_payload",
        "secret",
        "token",
        "api_key",
        "cookie",
        "credential",
        "authorization",
        "password",
    ] {
        assert!(!json.contains(word), "forbidden field leaked: {word}");
    }
}

#[test]
fn non_goals_have_no_runtime_entry_symbols() {
    let exported = exported_contract_symbols();
    for word in [
        "execute",
        "spawn_process",
        "start_server",
        "http_client",
        "query",
        "event_query",
        "session_ledger_write",
    ] {
        assert!(
            !exported.iter().any(|symbol| symbol.contains(word)),
            "unexpected symbol: {word}"
        );
    }
}
