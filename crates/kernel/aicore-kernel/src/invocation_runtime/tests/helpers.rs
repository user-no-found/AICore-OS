use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use aicore_foundation::AicoreLayout;

use crate::{
    InstalledManifestRegistry, KernelHandlerError, KernelHandlerRegistry, KernelHandlerResult,
    KernelInvocationEnvelope, KernelInvocationRuntime, KernelPayload,
    runtime_status_handler_for_layout,
};

pub(super) fn smoke_handler(
    envelope: &KernelInvocationEnvelope,
    _route: &crate::KernelRouteRuntimeOutput,
) -> Result<KernelHandlerResult, KernelHandlerError> {
    Ok(KernelHandlerResult::summary(format!(
        "smoke handled {}",
        envelope.operation
    )))
}

pub(super) fn structured_status_handler(
    _envelope: &KernelInvocationEnvelope,
    _route: &crate::KernelRouteRuntimeOutput,
) -> Result<KernelHandlerResult, KernelHandlerError> {
    let mut fields = BTreeMap::new();
    fields.insert("foundation_installed".to_string(), "yes".to_string());
    fields.insert("kernel_installed".to_string(), "yes".to_string());
    fields.insert("manifest_count".to_string(), "3".to_string());

    Ok(KernelHandlerResult::structured(
        "runtime.status",
        fields,
        "foundation_installed=yes | kernel_installed=yes | manifest_count=3",
    ))
}

pub(super) fn failing_handler(
    _envelope: &KernelInvocationEnvelope,
    _route: &crate::KernelRouteRuntimeOutput,
) -> Result<KernelHandlerResult, KernelHandlerError> {
    Err(KernelHandlerError::new("smoke handler failed"))
}

pub(super) fn secret_failing_handler(
    _envelope: &KernelInvocationEnvelope,
    _route: &crate::KernelRouteRuntimeOutput,
) -> Result<KernelHandlerResult, KernelHandlerError> {
    Err(KernelHandlerError::new(
        "failed with sk-live-secret-value secret://auth.openai.main token=abc123",
    ))
}

pub(super) fn structured_secret_result_handler(
    _envelope: &KernelInvocationEnvelope,
    _route: &crate::KernelRouteRuntimeOutput,
) -> Result<KernelHandlerResult, KernelHandlerError> {
    let mut fields = BTreeMap::new();
    fields.insert(
        "unsafe_detail".to_string(),
        "structured-secret-field-value secret_ref".to_string(),
    );

    Ok(KernelHandlerResult::structured(
        "runtime.status",
        fields,
        "raw result payload structured-secret-field-value",
    ))
}

pub(super) fn envelope(operation: &str) -> KernelInvocationEnvelope {
    KernelInvocationEnvelope::new("global-main", operation, operation, KernelPayload::Empty)
}

pub(super) fn registry_with_manifest(capabilities: &[(&str, &str)]) -> PathBuf {
    let root = temp_dir("invocation-registry");
    write_manifest(&root, "aicore-cli.toml", "aicore-cli", capabilities);
    root
}

pub(super) fn write_manifest(
    root: &PathBuf,
    file_name: &str,
    app_id: &str,
    capabilities: &[(&str, &str)],
) {
    write_manifest_with_contract(root, file_name, app_id, "kernel.app.v1", capabilities);
}

pub(super) fn write_manifest_with_contract(
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

pub(super) fn write_process_manifest(
    root: &PathBuf,
    entrypoint: &str,
    transport: &str,
    args: &[&str],
) {
    let args_toml = args
        .iter()
        .map(|arg| format!("\"{}\"", arg.replace('"', "\\\"")))
        .collect::<Vec<_>>()
        .join(", ");
    let content = format!(
        "component_id = \"aicore-component-smoke\"\napp_id = \"aicore-cli\"\nkind = \"app\"\nentrypoint = \"{}\"\ninvocation_mode = \"local_process\"\ntransport = \"{transport}\"\nargs = [{args_toml}]\ncontract_version = \"kernel.app.v1\"\n\n[[capabilities]]\nid = \"component.process.smoke\"\noperation = \"component.process.smoke\"\nvisibility = \"diagnostic\"\n",
        entrypoint.replace('"', "\\\"")
    );
    fs::write(root.join("aicore-component-smoke.toml"), content).expect("write manifest");
}

pub(super) fn process_fixture_script(root: &PathBuf, name: &str, body: &str) -> String {
    let path = root.join(name);
    fs::write(&path, format!("#!/bin/sh\n{body}")).expect("write process fixture");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(&path).expect("script metadata").permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&path, permissions).expect("script should be executable");
    }
    path.display().to_string()
}

pub(super) fn temp_dir(name: &str) -> PathBuf {
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

pub(super) fn seed_runtime_status_layout(home: &PathBuf, capability_count: usize) {
    let foundation = home.join(".aicore/runtime/foundation");
    let kernel = home.join(".aicore/runtime/kernel");
    let manifests = home.join(".aicore/share/manifests");
    fs::create_dir_all(&foundation).expect("foundation metadata dir");
    fs::create_dir_all(&kernel).expect("kernel metadata dir");
    fs::create_dir_all(&manifests).expect("manifest dir");
    fs::write(foundation.join("install.toml"), "status = \"installed\"\n")
        .expect("foundation install metadata");
    fs::write(kernel.join("install.toml"), "status = \"installed\"\n")
        .expect("kernel install metadata");
    fs::write(
        manifests.join("aicore.toml"),
        manifest_with_capability_count(capability_count),
    )
    .expect("manifest metadata");
}

pub(super) fn manifest_with_capability_count(count: usize) -> String {
    let mut content = r#"
component_id = "aicore"
app_id = "aicore"
kind = "app"
entrypoint = "/tmp/aicore"
contract_version = "kernel.app.v1"
"#
    .to_string();
    for index in 0..count {
        content.push_str(&format!(
            "\n[[capabilities]]\nid = \"runtime.status.{index}\"\noperation = \"runtime.status.{index}\"\nvisibility = \"user\"\n"
        ));
    }
    content
}

pub(super) fn runtime_with_handler(
    registry_root: &PathBuf,
    handlers: KernelHandlerRegistry,
) -> KernelInvocationRuntime {
    KernelInvocationRuntime::new(
        InstalledManifestRegistry::load_from_dir(registry_root).expect("registry"),
        handlers,
    )
}

pub(super) fn runtime_status_runtime(layout_home: &PathBuf) -> KernelInvocationRuntime {
    let layout = AicoreLayout::new(layout_home);
    let registry = registry_with_manifest(&[("runtime.status", "runtime.status")]);
    KernelInvocationRuntime::new(
        InstalledManifestRegistry::load_from_dir(&registry).expect("registry"),
        KernelHandlerRegistry::new()
            .with_handler("runtime.status", runtime_status_handler_for_layout(layout)),
    )
}
