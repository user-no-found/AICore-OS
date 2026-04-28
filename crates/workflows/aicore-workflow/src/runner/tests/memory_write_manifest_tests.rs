use std::fs;

use crate::layers::Workflow;

use super::{bash_env, fake_app_target, install_layer_with_shell_env, temp_home};

#[test]
fn app_cli_install_writes_memory_write_process_manifests() {
    let home_root = temp_home("app-cli-memory-write-process-manifest");
    let target_dir = fake_app_target("app-cli-memory-write-target", "aicore-cli");
    install_layer_with_shell_env(Workflow::AppCli, &target_dir, &bash_env(&home_root))
        .expect("app-cli install should succeed");
    let cli_manifest =
        fs::read_to_string(home_root.join(".aicore/share/manifests/aicore-cli.toml"))
            .expect("aicore-cli manifest should exist");

    for manifest_spec in memory_write_manifest_specs() {
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

struct MemoryWriteManifestSpec {
    file_name: &'static str,
    component_id: &'static str,
    operation: &'static str,
    arg: &'static str,
}

fn memory_write_manifest_specs() -> [MemoryWriteManifestSpec; 3] {
    [
        MemoryWriteManifestSpec {
            file_name: "aicore-memory-remember.toml",
            component_id: "aicore-memory-remember",
            operation: "memory.remember",
            arg: "__component-memory-remember-stdio",
        },
        MemoryWriteManifestSpec {
            file_name: "aicore-memory-accept.toml",
            component_id: "aicore-memory-accept",
            operation: "memory.accept",
            arg: "__component-memory-accept-stdio",
        },
        MemoryWriteManifestSpec {
            file_name: "aicore-memory-reject.toml",
            component_id: "aicore-memory-reject",
            operation: "memory.reject",
            arg: "__component-memory-reject-stdio",
        },
    ]
}
