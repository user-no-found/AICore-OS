use std::fs;

use crate::layers::Workflow;

use super::{bash_env, fake_app_target, install_layer_with_shell_env, temp_home};

#[test]
fn app_cli_install_writes_memory_read_process_manifests() {
    let home_root = temp_home("app-cli-memory-read-process-manifest");
    let target_dir = fake_app_target("app-cli-memory-read-target", "aicore-cli");
    install_layer_with_shell_env(Workflow::AppCli, &target_dir, &bash_env(&home_root))
        .expect("app-cli install should succeed");
    let cli_manifest =
        fs::read_to_string(home_root.join(".aicore/share/manifests/aicore-cli.toml"))
            .expect("aicore-cli manifest should exist");

    for manifest_spec in memory_read_manifest_specs() {
        let manifest = fs::read_to_string(
            home_root
                .join(".aicore/share/manifests")
                .join(manifest_spec.file_name),
        )
        .unwrap_or_else(|_| panic!("{} should exist", manifest_spec.file_name));

        assert!(manifest.contains(&format!(
            "component_id = \"{}\"",
            manifest_spec.component_id
        )));
        assert!(manifest.contains("app_id = \"aicore-cli\""));
        assert!(manifest.contains("invocation_mode = \"local_process\""));
        assert!(manifest.contains("transport = \"stdio_jsonl\""));
        assert!(manifest.contains(&format!("args = [\"{}\"]", manifest_spec.arg)));
        assert!(manifest.contains(&format!("operation = \"{}\"", manifest_spec.operation)));
        assert!(!cli_manifest.contains(&format!("operation = \"{}\"", manifest_spec.operation)));
    }
}

struct MemoryReadManifestSpec {
    file_name: &'static str,
    component_id: &'static str,
    operation: &'static str,
    arg: &'static str,
}

fn memory_read_manifest_specs() -> [MemoryReadManifestSpec; 6] {
    [
        MemoryReadManifestSpec {
            file_name: "aicore-memory-status.toml",
            component_id: "aicore-memory-status",
            operation: "memory.status",
            arg: "__component-memory-status-stdio",
        },
        MemoryReadManifestSpec {
            file_name: "aicore-memory-search.toml",
            component_id: "aicore-memory-search",
            operation: "memory.search",
            arg: "__component-memory-search-stdio",
        },
        MemoryReadManifestSpec {
            file_name: "aicore-memory-proposals.toml",
            component_id: "aicore-memory-proposals",
            operation: "memory.proposals",
            arg: "__component-memory-proposals-stdio",
        },
        MemoryReadManifestSpec {
            file_name: "aicore-memory-audit.toml",
            component_id: "aicore-memory-audit",
            operation: "memory.audit",
            arg: "__component-memory-audit-stdio",
        },
        MemoryReadManifestSpec {
            file_name: "aicore-memory-wiki.toml",
            component_id: "aicore-memory-wiki",
            operation: "memory.wiki",
            arg: "__component-memory-wiki-stdio",
        },
        MemoryReadManifestSpec {
            file_name: "aicore-memory-wiki-page.toml",
            component_id: "aicore-memory-wiki-page",
            operation: "memory.wiki_page",
            arg: "__component-memory-wiki-page-stdio",
        },
    ]
}
