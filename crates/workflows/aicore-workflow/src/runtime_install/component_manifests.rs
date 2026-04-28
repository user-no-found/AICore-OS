use std::path::Path;

use aicore_kernel::{
    ComponentInvocationMode, ComponentTransport, InstalledCapability, InstalledComponentManifest,
};

use crate::layers::Workflow;

pub(super) fn app_manifest_for(
    workflow: Workflow,
    entrypoint: &Path,
) -> Option<InstalledComponentManifest> {
    let (component_id, capabilities) = match workflow {
        Workflow::AppAicore => (
            "aicore",
            vec![
                capability("runtime.status", "runtime.status"),
                capability("system.status", "system.status"),
            ],
        ),
        Workflow::AppCli => ("aicore-cli", vec![capability("config.path", "config.path")]),
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

pub(super) fn app_cli_process_manifests(entrypoint: &Path) -> Vec<InstalledComponentManifest> {
    vec![
        local_process_manifest(
            "aicore-component-smoke",
            "component.process.smoke",
            "__component-smoke-stdio",
            "diagnostic",
            entrypoint,
        ),
        local_process_manifest(
            "aicore-config-validate",
            "config.validate",
            "__component-config-validate-stdio",
            "user",
            entrypoint,
        ),
        local_process_manifest(
            "aicore-auth-list",
            "auth.list",
            "__component-auth-list-stdio",
            "user",
            entrypoint,
        ),
        local_process_manifest(
            "aicore-model-show",
            "model.show",
            "__component-model-show-stdio",
            "user",
            entrypoint,
        ),
        local_process_manifest(
            "aicore-service-list",
            "service.list",
            "__component-service-list-stdio",
            "user",
            entrypoint,
        ),
        local_process_manifest(
            "aicore-runtime-smoke",
            "runtime.smoke",
            "__component-runtime-smoke-stdio",
            "user",
            entrypoint,
        ),
        local_process_manifest(
            "aicore-instance-list",
            "instance.list",
            "__component-instance-list-stdio",
            "user",
            entrypoint,
        ),
        local_process_manifest(
            "aicore-cli-status",
            "cli.status",
            "__component-status-stdio",
            "user",
            entrypoint,
        ),
        local_process_manifest(
            "aicore-provider-smoke",
            "provider.smoke",
            "__component-provider-smoke-stdio",
            "user",
            entrypoint,
        ),
        local_process_manifest(
            "aicore-agent-smoke",
            "agent.smoke",
            "__component-agent-smoke-stdio",
            "user",
            entrypoint,
        ),
        local_process_manifest(
            "aicore-agent-session-smoke",
            "agent.session_smoke",
            "__component-agent-session-smoke-stdio",
            "user",
            entrypoint,
        ),
        local_process_manifest(
            "aicore-memory-status",
            "memory.status",
            "__component-memory-status-stdio",
            "user",
            entrypoint,
        ),
        local_process_manifest(
            "aicore-memory-search",
            "memory.search",
            "__component-memory-search-stdio",
            "user",
            entrypoint,
        ),
        local_process_manifest(
            "aicore-memory-proposals",
            "memory.proposals",
            "__component-memory-proposals-stdio",
            "user",
            entrypoint,
        ),
        local_process_manifest(
            "aicore-memory-audit",
            "memory.audit",
            "__component-memory-audit-stdio",
            "user",
            entrypoint,
        ),
        local_process_manifest(
            "aicore-memory-wiki",
            "memory.wiki",
            "__component-memory-wiki-stdio",
            "user",
            entrypoint,
        ),
        local_process_manifest(
            "aicore-memory-wiki-page",
            "memory.wiki_page",
            "__component-memory-wiki-page-stdio",
            "user",
            entrypoint,
        ),
        local_process_manifest(
            "aicore-memory-remember",
            "memory.remember",
            "__component-memory-remember-stdio",
            "user",
            entrypoint,
        ),
        local_process_manifest(
            "aicore-memory-accept",
            "memory.accept",
            "__component-memory-accept-stdio",
            "user",
            entrypoint,
        ),
        local_process_manifest(
            "aicore-memory-reject",
            "memory.reject",
            "__component-memory-reject-stdio",
            "user",
            entrypoint,
        ),
    ]
}

fn local_process_manifest(
    component_id: &str,
    operation: &str,
    arg: &str,
    visibility: &str,
    entrypoint: &Path,
) -> InstalledComponentManifest {
    InstalledComponentManifest {
        component_id: component_id.to_string(),
        app_id: "aicore-cli".to_string(),
        kind: "app".to_string(),
        entrypoint: entrypoint.display().to_string(),
        invocation_mode: ComponentInvocationMode::LocalProcess,
        transport: ComponentTransport::StdioJsonl,
        args: vec![arg.to_string()],
        working_dir: None,
        env_policy: Some("minimal".to_string()),
        contract_version: "kernel.app.v1".to_string(),
        capabilities: vec![InstalledCapability {
            id: operation.to_string(),
            operation: operation.to_string(),
            visibility: visibility.to_string(),
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
