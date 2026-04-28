use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{ContractVersion, KernelRouteRuntime, KernelRouteRuntimeInput};

use super::InstalledManifestRegistry;

#[test]
fn installed_manifest_loader_reads_component_manifest() {
    let root = temp_dir("manifest-loader");
    fs::write(
        root.join("aicore-cli.toml"),
        r#"
component_id = "aicore-cli"
app_id = "aicore-cli"
kind = "app"
entrypoint = "/home/demo/.aicore/bin/aicore-cli"
contract_version = "kernel.app.v1"

[[capabilities]]
id = "memory.status"
operation = "memory.status"
visibility = "user"

[[capabilities]]
id = "memory.search"
operation = "memory.search"
visibility = "user"
"#,
    )
    .expect("write manifest");

    let registry =
        InstalledManifestRegistry::load_from_dir(&root).expect("manifest registry should load");

    assert_eq!(registry.manifest_count(), 1);
    assert_eq!(registry.capability_count(), 2);
    assert_eq!(registry.manifests()[0].component_id, "aicore-cli");
    assert_eq!(
        registry.manifests()[0].capabilities[0].operation,
        "memory.status"
    );
}

#[test]
fn installed_manifest_registry_builds_capability_registry() {
    let root = temp_dir("capability-registry");
    fs::write(
        root.join("aicore.toml"),
        r#"
component_id = "aicore"
app_id = "aicore"
kind = "app"
entrypoint = "/home/demo/.aicore/bin/aicore"
contract_version = "kernel.app.v1"

[[capabilities]]
id = "runtime.status"
operation = "runtime.status"
visibility = "user"
"#,
    )
    .expect("write manifest");

    let registry =
        InstalledManifestRegistry::load_from_dir(&root).expect("manifest registry should load");
    let capability_registry = registry.to_capability_registry();
    let entry = capability_registry
        .find("runtime.status", "runtime.status")
        .expect("runtime.status should be registered");

    assert_eq!(entry.app_id, "aicore");
    assert_eq!(entry.contract_version.contract_id, "kernel.app");
    assert_eq!(entry.contract_version.major, 1);
}

#[test]
fn component_manifest_supports_invocation_mode_and_transport() {
    let root = temp_dir("manifest-process-metadata");
    fs::write(
        root.join("aicore-component-smoke.toml"),
        r#"
component_id = "aicore-component-smoke"
app_id = "aicore-cli"
kind = "app"
entrypoint = "/home/demo/.aicore/bin/aicore-cli"
invocation_mode = "local_process"
transport = "stdio_jsonl"
args = ["__component-smoke-stdio"]
working_dir = "/home/demo"
env_policy = "minimal"
contract_version = "kernel.app.v1"

[[capabilities]]
id = "component.process.smoke"
operation = "component.process.smoke"
visibility = "diagnostic"
"#,
    )
    .expect("write process manifest");

    let registry =
        InstalledManifestRegistry::load_from_dir(&root).expect("manifest registry should load");
    let manifest = &registry.manifests()[0];
    let candidate = registry
        .operation_candidates("component.process.smoke")
        .pop()
        .expect("process smoke route candidate");

    assert_eq!(manifest.invocation_mode.as_str(), "local_process");
    assert_eq!(manifest.transport.as_str(), "stdio_jsonl");
    assert_eq!(manifest.args, vec!["__component-smoke-stdio"]);
    assert_eq!(manifest.working_dir.as_deref(), Some("/home/demo"));
    assert_eq!(manifest.env_policy.as_deref(), Some("minimal"));
    assert_eq!(candidate.invocation_mode.as_str(), "local_process");
    assert_eq!(candidate.transport.as_str(), "stdio_jsonl");
    assert_eq!(candidate.args, vec!["__component-smoke-stdio"]);
}

#[test]
fn component_manifest_defaults_to_in_process_for_existing_manifests() {
    let root = temp_dir("manifest-process-defaults");
    write_manifest(
        &root,
        "aicore-cli.toml",
        "aicore-cli",
        "kernel.app.v1",
        &[("memory.search", "memory.search")],
    );

    let registry =
        InstalledManifestRegistry::load_from_dir(&root).expect("manifest registry should load");
    let manifest = &registry.manifests()[0];
    let candidate = registry
        .operation_candidates("memory.search")
        .pop()
        .expect("memory.search route candidate");

    assert_eq!(manifest.invocation_mode.as_str(), "in_process");
    assert_eq!(manifest.transport.as_str(), "unsupported");
    assert!(manifest.args.is_empty());
    assert_eq!(candidate.invocation_mode.as_str(), "in_process");
    assert_eq!(candidate.transport.as_str(), "unsupported");
}

#[test]
fn installed_manifest_registry_routes_memory_search() {
    let root = temp_dir("route-memory-search");
    write_manifest(
        &root,
        "aicore-cli.toml",
        "aicore-cli",
        "kernel.app.v1",
        &[("memory.search", "memory.search")],
    );
    let registry = InstalledManifestRegistry::load_from_dir(&root).expect("registry");
    let runtime = KernelRouteRuntime::from_registry(registry);

    let output = runtime
        .route(KernelRouteRuntimeInput::new("memory.search"))
        .expect("memory.search should route");

    assert_eq!(output.component_id, "aicore-cli");
    assert_eq!(output.app_id, "aicore-cli");
    assert_eq!(output.capability_id, "memory.search");
    assert_eq!(output.decision.request.operation, "memory.search");
    assert_eq!(output.decision.target.app_id, "aicore-cli");
    assert_eq!(
        output.decision.target.contract_version.contract_id,
        "kernel.app"
    );
    assert_eq!(output.invocation_mode.as_str(), "in_process");
    assert_eq!(output.transport.as_str(), "unsupported");
}

#[test]
fn route_decision_exposes_component_process_metadata() {
    let root = temp_dir("route-process-metadata");
    fs::write(
        root.join("aicore-component-smoke.toml"),
        r#"
component_id = "aicore-component-smoke"
app_id = "aicore-cli"
kind = "app"
entrypoint = "/home/demo/.aicore/bin/aicore-cli"
invocation_mode = "local_process"
transport = "stdio_jsonl"
args = ["__component-smoke-stdio"]
contract_version = "kernel.app.v1"

[[capabilities]]
id = "component.process.smoke"
operation = "component.process.smoke"
visibility = "diagnostic"
"#,
    )
    .expect("write process manifest");
    let registry = InstalledManifestRegistry::load_from_dir(&root).expect("registry");
    let runtime = KernelRouteRuntime::from_registry(registry);

    let output = runtime
        .route(KernelRouteRuntimeInput::new("component.process.smoke"))
        .expect("component process smoke should route");

    assert_eq!(output.component_id, "aicore-component-smoke");
    assert_eq!(output.invocation_mode.as_str(), "local_process");
    assert_eq!(output.transport.as_str(), "stdio_jsonl");
    assert_eq!(output.args, vec!["__component-smoke-stdio"]);
    assert!(!output.handler_executed);
}

#[test]
fn installed_manifest_registry_routes_provider_smoke() {
    let root = temp_dir("route-provider-smoke");
    write_manifest(
        &root,
        "aicore-cli.toml",
        "aicore-cli",
        "kernel.app.v1",
        &[("provider.smoke", "provider.smoke")],
    );
    let registry = InstalledManifestRegistry::load_from_dir(&root).expect("registry");
    let runtime = KernelRouteRuntime::from_registry(registry);

    let output = runtime
        .route(KernelRouteRuntimeInput::new("provider.smoke"))
        .expect("provider.smoke should route");

    assert_eq!(output.component_id, "aicore-cli");
    assert_eq!(output.capability_id, "provider.smoke");
    assert!(!output.handler_executed);
}

#[test]
fn installed_manifest_registry_rejects_missing_operation() {
    let root = temp_dir("route-missing-operation");
    write_manifest(
        &root,
        "aicore-cli.toml",
        "aicore-cli",
        "kernel.app.v1",
        &[("memory.search", "memory.search")],
    );
    let registry = InstalledManifestRegistry::load_from_dir(&root).expect("registry");
    let runtime = KernelRouteRuntime::from_registry(registry);

    let error = runtime
        .route(KernelRouteRuntimeInput::new("unknown.operation"))
        .expect_err("unknown operation should fail");

    assert!(error.to_string().contains("missing capability"));
    assert!(error.to_string().contains("unknown.operation"));
}

#[test]
fn installed_manifest_registry_missing_manifest_dir_returns_no_route() {
    let root = temp_dir("route-missing-dir").join("missing");
    let registry = InstalledManifestRegistry::load_from_dir(&root).expect("registry");
    let runtime = KernelRouteRuntime::from_registry(registry);

    let error = runtime
        .route(KernelRouteRuntimeInput::new("memory.search"))
        .expect_err("missing manifest dir should have no route");

    assert!(error.to_string().contains("missing capability"));
}

#[test]
fn route_decision_rejects_contract_version_mismatch() {
    let root = temp_dir("route-contract-mismatch");
    write_manifest(
        &root,
        "aicore-cli.toml",
        "aicore-cli",
        "kernel.app.v2",
        &[("memory.search", "memory.search")],
    );
    let registry = InstalledManifestRegistry::load_from_dir(&root).expect("registry");
    let runtime = KernelRouteRuntime::from_registry(registry);

    let error = runtime
        .route(KernelRouteRuntimeInput::new("memory.search"))
        .expect_err("contract mismatch should fail");

    assert!(error.to_string().contains("contract version mismatch"));
}

#[test]
fn route_decision_rejects_requested_contract_version_mismatch() {
    let root = temp_dir("route-requested-contract-mismatch");
    write_manifest(
        &root,
        "aicore-cli.toml",
        "aicore-cli",
        "kernel.app.v1",
        &[("memory.search", "memory.search")],
    );
    let registry = InstalledManifestRegistry::load_from_dir(&root).expect("registry");
    let runtime = KernelRouteRuntime::from_registry(registry);

    let error = runtime
        .route(
            KernelRouteRuntimeInput::new("memory.search")
                .with_requested_contract(ContractVersion::new("kernel.app", 2, 0)),
        )
        .expect_err("requested contract mismatch should fail");

    assert!(error.to_string().contains("contract version mismatch"));
}

#[test]
fn route_decision_rejects_ambiguous_duplicate_capability() {
    let root = temp_dir("route-duplicate-capability");
    write_manifest(
        &root,
        "aicore-cli.toml",
        "aicore-cli",
        "kernel.app.v1",
        &[("memory.search", "memory.search")],
    );
    write_manifest(
        &root,
        "aicore-memory.toml",
        "aicore-memory",
        "kernel.app.v1",
        &[("memory.search", "memory.search")],
    );
    let registry = InstalledManifestRegistry::load_from_dir(&root).expect("registry");
    let runtime = KernelRouteRuntime::from_registry(registry);

    let error = runtime
        .route(KernelRouteRuntimeInput::new("memory.search"))
        .expect_err("duplicate operation should be ambiguous");

    assert!(error.to_string().contains("ambiguous route"));
    assert!(error.to_string().contains("aicore-cli"));
    assert!(error.to_string().contains("aicore-memory"));
}

fn write_manifest(
    root: &PathBuf,
    file_name: &str,
    app_id: &str,
    contract_version: &str,
    capabilities: &[(&str, &str)],
) {
    let mut content = format!(
        "component_id = \"{app_id}\"\napp_id = \"{app_id}\"\nkind = \"app\"\nentrypoint = \"/home/demo/.aicore/bin/{app_id}\"\ncontract_version = \"{contract_version}\"\n"
    );
    for (id, operation) in capabilities {
        content.push_str(&format!(
            "\n[[capabilities]]\nid = \"{id}\"\noperation = \"{operation}\"\nvisibility = \"user\"\n"
        ));
    }
    fs::write(root.join(file_name), content).expect("write manifest");
}

fn temp_dir(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "aicore-kernel-{name}-{}-{unique}",
        std::process::id()
    ));
    fs::create_dir_all(&path).expect("create temp dir");
    path
}
