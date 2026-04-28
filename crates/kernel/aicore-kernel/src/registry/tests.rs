use aicore_foundation::{AicoreLayout, InstanceId};

use super::{
    AppManifest, AppRegistry, CapabilityDescriptor, KernelRoutePlanner,
    default_capability_registry, default_instance_registry, workspace_instance,
};
use crate::{ContractVersion, InstanceKind, KernelErrorCode, KernelRouteRequest};

#[test]
fn app_registry_rejects_duplicate_app_id() {
    let mut registry = AppRegistry::new();
    registry
        .register(AppManifest::new("app.cli", "cli"))
        .expect("first app should register");

    let error = registry
        .register(AppManifest::new("app.cli", "cli"))
        .expect_err("duplicate app id should fail");

    assert_eq!(error.code, KernelErrorCode::Conflict);
}

#[test]
fn instance_registry_contains_global_main() {
    let registry = default_instance_registry();

    assert_eq!(registry.list()[0].id.as_str(), "global-main");
    assert_eq!(registry.list()[0].kind, InstanceKind::GlobalMain);
}

#[test]
fn workspace_instance_cannot_impersonate_global_main() {
    let layout = AicoreLayout::new("/home/demo");
    let error = workspace_instance("global-main", "/workspace/demo", &layout)
        .expect_err("workspace cannot use global-main");

    assert_eq!(
        error.to_string(),
        "invalid state: workspace instance cannot use global-main id"
    );
}

#[test]
fn capability_registry_finds_app_for_operation() {
    let mut registry = super::CapabilityRegistry::new();
    registry.register(
        "app.memory",
        CapabilityDescriptor::new("memory.search").with_operation("search"),
        ContractVersion::new("kernel.memory", 1, 0),
    );

    let entry = registry
        .find("memory.search", "search")
        .expect("capability should resolve");

    assert_eq!(entry.app_id, "app.memory");
}

#[test]
fn route_planner_routes_provider_chat_to_provider_app() {
    let planner = KernelRoutePlanner::new(default_capability_registry());
    let decision = planner
        .plan(KernelRouteRequest::new(
            "global-main",
            "provider.chat",
            "complete",
        ))
        .expect("provider chat should route");

    assert_eq!(decision.target.app_id, "app.provider");
}

#[test]
fn route_planner_routes_memory_search_to_memory_app() {
    let planner = KernelRoutePlanner::new(default_capability_registry());
    let decision = planner
        .plan(KernelRouteRequest::new(
            "global-main",
            "memory.search",
            "search",
        ))
        .expect("memory search should route");

    assert_eq!(decision.target.app_id, "app.memory");
}

#[test]
fn route_planner_routes_tool_shell_to_tools_app() {
    let planner = KernelRoutePlanner::new(default_capability_registry());
    let decision = planner
        .plan(KernelRouteRequest::new(
            "global-main",
            "tool.shell",
            "execute",
        ))
        .expect("tool shell should route");

    assert_eq!(decision.target.app_id, "app.tools");
}

#[test]
fn route_planner_rejects_missing_capability() {
    let planner = KernelRoutePlanner::new(default_capability_registry());
    let error = planner
        .plan(KernelRouteRequest::new("global-main", "missing.cap", "run"))
        .expect_err("missing capability should fail");

    assert_eq!(error.code, KernelErrorCode::MissingCapability);
}

#[test]
fn route_planner_rejects_contract_version_mismatch() {
    let planner = KernelRoutePlanner::new(default_capability_registry());
    let mut request = KernelRouteRequest::new("global-main", "provider.chat", "complete");
    request.requested_contract = Some(ContractVersion::new("kernel.provider", 2, 0));

    let error = planner
        .plan(request)
        .expect_err("version mismatch should fail");

    assert_eq!(error.code, KernelErrorCode::VersionMismatch);
}

#[test]
fn route_decision_includes_trace_and_audit_context() {
    let planner = KernelRoutePlanner::new(default_capability_registry());
    let decision = planner
        .plan(KernelRouteRequest::new(
            "global-main",
            "provider.chat",
            "complete",
        ))
        .expect("provider chat should route");

    assert_eq!(decision.request.trace_context.trace_id, "trace.route");
    assert_eq!(decision.request.audit_context.actor, "system");
}

#[test]
fn workspace_instance_registration_rejects_global_kind_impersonation() {
    let mut registry = default_instance_registry();
    let layout = AicoreLayout::new("/home/demo");
    let mut record =
        workspace_instance("inst-a", "/workspace/a", &layout).expect("workspace should build");
    record.id = InstanceId::new("inst-global").expect("id should be safe");
    record.kind = InstanceKind::GlobalMain;

    let error = registry
        .register(record)
        .expect_err("workspace cannot claim global kind");

    assert_eq!(
        error.to_string(),
        "invalid state: only global-main can use InstanceKind::GlobalMain"
    );
}
