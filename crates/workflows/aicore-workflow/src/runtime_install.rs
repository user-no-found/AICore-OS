use std::fs;
use std::path::Path;

use aicore_foundation::AicoreLayout;
use aicore_kernel::{
    ComponentInvocationMode, ComponentTransport, InstalledCapability, InstalledComponentManifest,
};

use crate::layers::Workflow;

const RUNTIME_VERSION: &str = "0.1.0";
const FOUNDATION_CONTRACT_VERSION: &str = "foundation.runtime.v1";
const KERNEL_CONTRACT_VERSION: &str = "kernel.runtime.v1";

pub fn install_global_runtime_metadata(
    workflow: Workflow,
    layout: &AicoreLayout,
) -> Result<(), String> {
    ensure_global_runtime_dirs(layout)?;
    match workflow {
        Workflow::Foundation => install_foundation_metadata(layout),
        Workflow::Kernel => install_kernel_metadata(layout),
        Workflow::Core | Workflow::AppAicore | Workflow::AppCli | Workflow::AppTui => Ok(()),
    }
}

pub fn install_app_manifest(
    workflow: Workflow,
    layout: &AicoreLayout,
    entrypoint: &Path,
) -> Result<(), String> {
    ensure_global_runtime_dirs(layout)?;
    let Some(manifest) = app_manifest_for(workflow, entrypoint) else {
        return Ok(());
    };
    write_atomic(
        &layout
            .manifests_root
            .join(format!("{}.toml", manifest.component_id)),
        &manifest.to_toml(),
    )?;
    if workflow == Workflow::AppCli {
        let smoke_manifest = component_process_smoke_manifest(entrypoint);
        write_atomic(
            &layout
                .manifests_root
                .join(format!("{}.toml", smoke_manifest.component_id)),
            &smoke_manifest.to_toml(),
        )?;
    }
    Ok(())
}

fn ensure_global_runtime_dirs(layout: &AicoreLayout) -> Result<(), String> {
    for dir in [
        &layout.bin_root,
        &layout.runtime_root,
        &layout.runtime_foundation_root,
        &layout.runtime_kernel_root,
        &layout.share_root,
        &layout.manifests_root,
        &layout.contracts_root,
        &layout.schemas_root,
        &layout.kernel_state_root,
        &layout.cache_root,
        &layout.logs_root,
    ] {
        fs::create_dir_all(dir)
            .map_err(|error| format!("创建全局 runtime 目录 {} 失败: {error}", dir.display()))?;
    }
    Ok(())
}

fn install_foundation_metadata(layout: &AicoreLayout) -> Result<(), String> {
    let root = &layout.runtime_foundation_root;
    let binary_path = layout.bin_root.join("aicore-foundation");
    write_atomic(
        &root.join("install.toml"),
        &format!(
            "layer = \"foundation\"\nstatus = \"installed\"\nruntime_root = \"{}\"\nbin_root = \"{}\"\nruntime_binary_path = \"{}\"\nruntime_binary_installed = {}\nruntime_protocol = \"stdio_jsonl\"\ncontract_version = \"{FOUNDATION_CONTRACT_VERSION}\"\nhealth = \"installed\"\n",
            root.display(),
            layout.bin_root.display(),
            binary_path.display(),
            binary_path.exists()
        ),
    )?;
    write_atomic(
        &root.join("version.toml"),
        &format!(
            "runtime_version = \"{RUNTIME_VERSION}\"\ncontract_version = \"{FOUNDATION_CONTRACT_VERSION}\"\n"
        ),
    )?;
    write_atomic(
        &root.join("primitives.toml"),
        "ids = true\nerrors = true\npaths = true\ncancellation = true\nqueues = true\nleases = true\ntime = true\nredaction = true\n",
    )?;
    write_atomic(
        &root.join("terminal.toml"),
        "terminal_kit = \"aicore-terminal\"\nmode_rich = true\nmode_plain = true\nmode_json = true\nno_color = true\n",
    )?;
    write_atomic(
        &root.join("paths.toml"),
        &format!(
            "global_root = \"{}\"\nbin = \"{}\"\nruntime = \"{}\"\nshare_manifests = \"{}\"\nkernel_state = \"{}\"\ncache = \"{}\"\nlogs = \"{}\"\n",
            layout.state_root.display(),
            layout.bin_root.display(),
            layout.runtime_root.display(),
            layout.manifests_root.display(),
            layout.kernel_state_root.display(),
            layout.cache_root.display(),
            layout.logs_root.display()
        ),
    )
}

fn install_kernel_metadata(layout: &AicoreLayout) -> Result<(), String> {
    let root = &layout.runtime_kernel_root;
    let binary_path = layout.bin_root.join("aicore-kernel");
    write_atomic(
        &root.join("install.toml"),
        &format!(
            "layer = \"kernel\"\nstatus = \"installed\"\nruntime_root = \"{}\"\nregistry_root = \"{}\"\nstate_root = \"{}\"\nruntime_binary_path = \"{}\"\nruntime_binary_installed = {}\nruntime_protocol = \"stdio_jsonl\"\ncontract_version = \"{KERNEL_CONTRACT_VERSION}\"\nhealth = \"installed\"\n",
            root.display(),
            layout.manifests_root.display(),
            layout.kernel_state_root.display(),
            binary_path.display(),
            binary_path.exists()
        ),
    )?;
    write_atomic(
        &root.join("version.toml"),
        &format!(
            "runtime_version = \"{RUNTIME_VERSION}\"\ncontract_version = \"{KERNEL_CONTRACT_VERSION}\"\n"
        ),
    )?;
    write_atomic(
        &root.join("contracts.toml"),
        "contract_version = \"kernel.runtime.v1\"\ninvocation_envelope = \"declared\"\nroute_request = \"declared\"\nroute_decision = \"declared\"\nevent_envelope = \"declared\"\n",
    )?;
    write_atomic(
        &root.join("capabilities.toml"),
        "capability_count = 0\nsource = \"installed_manifests\"\n",
    )?;
    write_atomic(
        &root.join("registry.toml"),
        &format!(
            "source = \"installed_manifests\"\nmanifest_dir = \"{}\"\ncomponent_count = 0\n",
            layout.manifests_root.display()
        ),
    )?;
    write_atomic(
        &root.join("routing.toml"),
        "mode = \"metadata_only\"\nroute_decision_runtime = true\ndispatcher_enabled = false\n",
    )?;
    write_atomic(
        &root.join("scheduler.toml"),
        "multi_instance_parallel = true\nexecution_lanes = \"declared\"\nworker_pool = \"metadata_only\"\n",
    )
}

fn app_manifest_for(workflow: Workflow, entrypoint: &Path) -> Option<InstalledComponentManifest> {
    let (component_id, capabilities) = match workflow {
        Workflow::AppAicore => (
            "aicore",
            vec![
                capability("runtime.status", "runtime.status"),
                capability("system.status", "system.status"),
            ],
        ),
        Workflow::AppCli => (
            "aicore-cli",
            vec![
                capability("config.path", "config.path"),
                capability("config.validate", "config.validate"),
                capability("auth.list", "auth.list"),
                capability("model.show", "model.show"),
                capability("service.list", "service.list"),
                capability("memory.status", "memory.status"),
                capability("memory.search", "memory.search"),
                capability("memory.proposals", "memory.proposals"),
                capability("memory.audit", "memory.audit"),
                capability("memory.wiki", "memory.wiki"),
                capability("provider.smoke", "provider.smoke"),
                capability("agent.smoke", "agent.smoke"),
                capability("agent.session_smoke", "agent.session_smoke"),
                capability("runtime.smoke", "runtime.smoke"),
                capability("instance.list", "instance.list"),
            ],
        ),
        Workflow::AppTui => (
            "aicore-tui",
            vec![
                capability("tui.session", "tui.session"),
                capability("tui.route_smoke", "tui.route_smoke"),
            ],
        ),
        Workflow::Foundation | Workflow::Kernel | Workflow::Core => return None,
    };

    Some(InstalledComponentManifest {
        component_id: component_id.to_string(),
        app_id: component_id.to_string(),
        kind: "app".to_string(),
        entrypoint: entrypoint.display().to_string(),
        invocation_mode: ComponentInvocationMode::InProcess,
        transport: ComponentTransport::Unsupported,
        args: Vec::new(),
        working_dir: None,
        env_policy: None,
        contract_version: "kernel.app.v1".to_string(),
        capabilities,
    })
}

fn component_process_smoke_manifest(entrypoint: &Path) -> InstalledComponentManifest {
    InstalledComponentManifest {
        component_id: "aicore-component-smoke".to_string(),
        app_id: "aicore-cli".to_string(),
        kind: "app".to_string(),
        entrypoint: entrypoint.display().to_string(),
        invocation_mode: ComponentInvocationMode::LocalProcess,
        transport: ComponentTransport::StdioJsonl,
        args: vec!["__component-smoke-stdio".to_string()],
        working_dir: None,
        env_policy: Some("minimal".to_string()),
        contract_version: "kernel.app.v1".to_string(),
        capabilities: vec![InstalledCapability {
            id: "component.process.smoke".to_string(),
            operation: "component.process.smoke".to_string(),
            visibility: "diagnostic".to_string(),
        }],
    }
}

fn capability(id: &str, operation: &str) -> InstalledCapability {
    InstalledCapability {
        id: id.to_string(),
        operation: operation.to_string(),
        visibility: "user".to_string(),
    }
}

fn write_atomic(path: &Path, content: &str) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("无法确定 {} 的父目录", path.display()))?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("创建目录 {} 失败: {error}", parent.display()))?;
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| format!("无效文件名: {}", path.display()))?;
    let tmp_path = parent.join(format!(".{file_name}.tmp-{}", std::process::id()));
    fs::write(&tmp_path, content)
        .map_err(|error| format!("写入临时文件 {} 失败: {error}", tmp_path.display()))?;
    fs::rename(&tmp_path, path).map_err(|error| {
        format!(
            "替换 runtime metadata {} -> {} 失败: {error}",
            tmp_path.display(),
            path.display()
        )
    })
}
