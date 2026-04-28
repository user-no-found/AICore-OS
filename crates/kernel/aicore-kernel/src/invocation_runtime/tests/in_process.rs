use crate::{KernelEventPayload, KernelEventType, KernelHandlerRegistry, KernelInvocationStatus};

use super::helpers::{
    envelope, failing_handler, registry_with_manifest, runtime_status_runtime,
    runtime_with_handler, seed_runtime_status_layout, smoke_handler, structured_status_handler,
    temp_dir, write_manifest, write_manifest_with_contract,
};

#[test]
fn invocation_runtime_routes_before_handler_execution() {
    let registry = registry_with_manifest(&[("memory.search", "memory.search")]);
    let runtime = runtime_with_handler(
        &registry,
        KernelHandlerRegistry::new().with_handler("memory.search", smoke_handler),
    );

    let output = runtime.invoke(envelope("memory.search"));

    assert_eq!(output.status, KernelInvocationStatus::Completed);
    assert_eq!(
        output
            .route
            .as_ref()
            .expect("route should exist")
            .component_id,
        "aicore-cli"
    );
    assert!(output.route_decision_made);
    assert!(output.handler_executed);
}

#[test]
fn invocation_runtime_executes_registered_in_process_handler() {
    let registry = registry_with_manifest(&[("memory.search", "memory.search")]);
    let runtime = runtime_with_handler(
        &registry,
        KernelHandlerRegistry::new().with_handler("memory.search", smoke_handler),
    );

    let output = runtime.invoke(envelope("memory.search"));

    assert_eq!(output.status, KernelInvocationStatus::Completed);
    assert!(output.handler_executed);
    assert_eq!(output.handler_kind.as_deref(), Some("in_process"));
    assert_eq!(output.failure_reason, None);
}

#[test]
fn invocation_runtime_returns_event_envelope() {
    let registry = registry_with_manifest(&[("memory.search", "memory.search")]);
    let runtime = runtime_with_handler(
        &registry,
        KernelHandlerRegistry::new().with_handler("memory.search", smoke_handler),
    );

    let envelope = envelope("memory.search");
    let expected_invocation_id = envelope.invocation_id.clone();
    let output = runtime.invoke(envelope);
    let event = output.event.expect("event should be generated");

    assert_eq!(event.event_type, KernelEventType::InvocationCompleted);
    assert_eq!(event.app_id, "aicore-cli");
    assert_eq!(event.instance_id, "global-main");
    assert_eq!(event.invocation_id, expected_invocation_id);
    assert_eq!(event.trace_context.trace_id, "trace.default");
    assert_eq!(
        event.payload,
        KernelEventPayload::Summary("smoke handled memory.search".to_string())
    );
}

#[test]
fn kernel_invocation_runtime_status_returns_structured_result() {
    let registry = registry_with_manifest(&[("runtime.status", "runtime.status")]);
    let runtime = runtime_with_handler(
        &registry,
        KernelHandlerRegistry::new().with_handler("runtime.status", structured_status_handler),
    );

    let output = runtime.invoke(envelope("runtime.status"));
    let result = output.result.expect("result envelope should exist");

    assert_eq!(result.result_kind.as_deref(), Some("runtime.status"));
    assert_eq!(
        result.public_fields.get("foundation_installed"),
        Some(&"yes".to_string())
    );
    assert_eq!(
        result.public_fields.get("manifest_count"),
        Some(&"3".to_string())
    );
}

#[test]
fn runtime_status_handler_builds_structured_result_from_supplied_layout() {
    let home = temp_dir("runtime-status-handler-home");
    seed_runtime_status_layout(&home, 2);
    let runtime = runtime_status_runtime(&home);

    let output = runtime.invoke(envelope("runtime.status"));
    let result = output.result.expect("runtime status result envelope");

    assert_eq!(result.result_kind.as_deref(), Some("runtime.status"));
    assert_eq!(
        result.public_fields.get("global_root"),
        Some(&home.join(".aicore").display().to_string())
    );
    assert_eq!(
        result.public_fields.get("foundation_installed"),
        Some(&"yes".to_string())
    );
    assert_eq!(
        result.public_fields.get("kernel_installed"),
        Some(&"yes".to_string())
    );
    assert_eq!(
        result.public_fields.get("manifest_count"),
        Some(&"1".to_string())
    );
    assert_eq!(
        result.public_fields.get("capability_count"),
        Some(&"2".to_string())
    );
}

#[test]
fn runtime_status_handler_uses_supplied_layout_not_home() {
    let home = temp_dir("runtime-status-handler-layout");
    seed_runtime_status_layout(&home, 1);
    let runtime = runtime_status_runtime(&home);

    let output = runtime.invoke(envelope("runtime.status"));

    let result = output.result.expect("runtime status result envelope");
    assert_eq!(
        result.public_fields.get("global_root"),
        Some(&home.join(".aicore").display().to_string())
    );
    assert_ne!(
        result.public_fields.get("global_root"),
        Some(
            &aicore_foundation::AicoreLayout::from_system_home()
                .state_root
                .display()
                .to_string()
        )
    );
}

#[test]
fn kernel_invocation_output_contains_result_envelope() {
    let registry = registry_with_manifest(&[("runtime.status", "runtime.status")]);
    let runtime = runtime_with_handler(
        &registry,
        KernelHandlerRegistry::new().with_handler("runtime.status", structured_status_handler),
    );

    let output = runtime.invoke(envelope("runtime.status"));
    let result = output.result.expect("result envelope should exist");

    assert_eq!(result.operation, "runtime.status");
    assert_eq!(result.status, KernelInvocationStatus::Completed);
    assert_eq!(result.handler_kind.as_deref(), Some("in_process"));
    assert!(result.handler_executed);
    assert!(result.event_generated);
}

#[test]
fn kernel_invocation_result_preserves_invocation_id() {
    let registry = registry_with_manifest(&[("runtime.status", "runtime.status")]);
    let runtime = runtime_with_handler(
        &registry,
        KernelHandlerRegistry::new().with_handler("runtime.status", structured_status_handler),
    );
    let envelope = envelope("runtime.status");
    let expected_invocation_id = envelope.invocation_id.clone();

    let output = runtime.invoke(envelope);

    assert_eq!(
        output.result.expect("result envelope").invocation_id,
        expected_invocation_id
    );
}

#[test]
fn kernel_invocation_result_preserves_route_metadata() {
    let registry = registry_with_manifest(&[("runtime.status", "runtime.status")]);
    let runtime = runtime_with_handler(
        &registry,
        KernelHandlerRegistry::new().with_handler("runtime.status", structured_status_handler),
    );

    let output = runtime.invoke(envelope("runtime.status"));
    let result = output.result.expect("result envelope");
    let route = result.route.expect("route metadata");

    assert_eq!(route.component_id, "aicore-cli");
    assert_eq!(route.app_id, "aicore-cli");
    assert_eq!(route.capability_id, "runtime.status");
    assert_eq!(route.contract_version, "kernel.app.v1");
}

#[test]
fn kernel_invocation_result_summary_is_derived_from_structured_result() {
    let registry = registry_with_manifest(&[("runtime.status", "runtime.status")]);
    let runtime = runtime_with_handler(
        &registry,
        KernelHandlerRegistry::new().with_handler("runtime.status", structured_status_handler),
    );

    let output = runtime.invoke(envelope("runtime.status"));
    let result = output.result.expect("result envelope");

    assert_eq!(
        result.summary,
        "foundation_installed=yes | kernel_installed=yes | manifest_count=3"
    );
    assert_eq!(
        result.public_fields.get("kernel_installed"),
        Some(&"yes".to_string())
    );
}

#[test]
fn invocation_runtime_missing_operation_does_not_execute_handler() {
    let registry = registry_with_manifest(&[("memory.search", "memory.search")]);
    let runtime = runtime_with_handler(
        &registry,
        KernelHandlerRegistry::new().with_handler("unknown.operation", smoke_handler),
    );

    let output = runtime.invoke(envelope("unknown.operation"));

    assert_eq!(output.status, KernelInvocationStatus::Failed);
    assert!(!output.route_decision_made);
    assert!(!output.handler_executed);
    assert!(
        output
            .failure_reason
            .as_deref()
            .expect("failure reason")
            .contains("missing capability")
    );
}

#[test]
fn invocation_runtime_missing_handler_returns_structured_failure() {
    let registry = registry_with_manifest(&[("memory.search", "memory.search")]);
    let runtime = runtime_with_handler(&registry, KernelHandlerRegistry::new());

    let output = runtime.invoke(envelope("memory.search"));

    assert_eq!(output.status, KernelInvocationStatus::Failed);
    assert!(output.route_decision_made);
    assert!(!output.handler_executed);
    assert_eq!(output.failure_stage.as_deref(), Some("handler_lookup"));
    assert!(
        output
            .failure_reason
            .as_deref()
            .expect("failure reason")
            .contains("missing handler")
    );
}

#[test]
fn invocation_runtime_ambiguous_route_does_not_execute_handler() {
    let root = temp_dir("ambiguous-route");
    write_manifest(
        &root,
        "aicore-cli.toml",
        "aicore-cli",
        &[("memory.search", "memory.search")],
    );
    write_manifest(
        &root,
        "aicore-memory.toml",
        "aicore-memory",
        &[("memory.search", "memory.search")],
    );
    let runtime = runtime_with_handler(
        &root,
        KernelHandlerRegistry::new().with_handler("memory.search", smoke_handler),
    );

    let output = runtime.invoke(envelope("memory.search"));

    assert_eq!(output.status, KernelInvocationStatus::Failed);
    assert!(!output.handler_executed);
    assert!(
        output
            .failure_reason
            .as_deref()
            .expect("failure reason")
            .contains("ambiguous route")
    );
}

#[test]
fn invocation_runtime_contract_mismatch_does_not_execute_handler() {
    let root = temp_dir("contract-mismatch");
    write_manifest_with_contract(
        &root,
        "aicore-cli.toml",
        "aicore-cli",
        "kernel.app.v2",
        &[("memory.search", "memory.search")],
    );
    let runtime = runtime_with_handler(
        &root,
        KernelHandlerRegistry::new().with_handler("memory.search", smoke_handler),
    );

    let output = runtime.invoke(envelope("memory.search"));

    assert_eq!(output.status, KernelInvocationStatus::Failed);
    assert!(!output.handler_executed);
    assert!(
        output
            .failure_reason
            .as_deref()
            .expect("failure reason")
            .contains("contract version mismatch")
    );
}

#[test]
fn invocation_runtime_handler_failure_returns_structured_failure() {
    let registry = registry_with_manifest(&[("memory.search", "memory.search")]);
    let runtime = runtime_with_handler(
        &registry,
        KernelHandlerRegistry::new().with_handler("memory.search", failing_handler),
    );

    let output = runtime.invoke(envelope("memory.search"));

    assert_eq!(output.status, KernelInvocationStatus::Failed);
    assert!(output.route_decision_made);
    assert!(output.handler_executed);
    assert_eq!(output.failure_stage.as_deref(), Some("handler_execute"));
    assert!(
        output
            .failure_reason
            .as_deref()
            .expect("failure reason")
            .contains("smoke handler failed")
    );
}

#[test]
fn invocation_runtime_output_marks_handler_executed() {
    let registry = registry_with_manifest(&[("memory.search", "memory.search")]);
    let runtime = runtime_with_handler(
        &registry,
        KernelHandlerRegistry::new().with_handler("memory.search", smoke_handler),
    );

    let output = runtime.invoke(envelope("memory.search"));

    assert!(output.handler_executed);
    assert!(output.event_generated);
    assert!(!output.spawned_process);
    assert!(!output.called_real_component);
}

#[test]
fn in_process_runtime_status_still_works() {
    let registry = registry_with_manifest(&[("runtime.status", "runtime.status")]);
    let runtime = runtime_with_handler(
        &registry,
        KernelHandlerRegistry::new().with_handler("runtime.status", structured_status_handler),
    );

    let output = runtime.invoke(envelope("runtime.status"));

    assert_eq!(output.status, KernelInvocationStatus::Completed);
    assert_eq!(output.handler_kind.as_deref(), Some("in_process"));
    assert!(!output.spawned_process);
    assert_eq!(
        output.result.expect("result").result_kind.as_deref(),
        Some("runtime.status")
    );
}

#[test]
fn invocation_event_uses_same_invocation_id_as_envelope() {
    let registry = registry_with_manifest(&[("memory.search", "memory.search")]);
    let runtime = runtime_with_handler(
        &registry,
        KernelHandlerRegistry::new().with_handler("memory.search", smoke_handler),
    );
    let envelope = envelope("memory.search");
    let expected_invocation_id = envelope.invocation_id.clone();

    let output = runtime.invoke(envelope);

    assert_eq!(
        output
            .event
            .expect("event should be generated")
            .invocation_id,
        expected_invocation_id
    );
}
