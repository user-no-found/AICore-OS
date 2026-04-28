use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::layers::Workflow;
use crate::shell_integration::{
    MANAGED_BLOCK_END, MANAGED_BLOCK_START, MANAGED_PATH_LINE, ShellPathBootstrapEnv,
};

use super::core::{cargo_args_for_workflow, target_dir_for};
use super::install::{
    install_bin_dir_for, install_layer_with_shell_env, install_manifest_for,
    install_visibility_warnings, installed_binary_path,
};

#[test]
fn foundation_workflow_uses_foundation_target_dir() {
    let root = Path::new("/repo");
    assert_eq!(
        target_dir_for(root, Workflow::Foundation),
        root.join("target/layers/foundation")
    );
}

#[test]
fn kernel_workflow_uses_kernel_target_dir() {
    let root = Path::new("/repo");
    assert_eq!(
        target_dir_for(root, Workflow::Kernel),
        root.join("target/layers/kernel")
    );
}

#[test]
fn app_aicore_workflow_uses_app_target_dir() {
    let root = Path::new("/repo");
    assert_eq!(
        target_dir_for(root, Workflow::AppAicore),
        root.join("target/apps/aicore")
    );
}

#[test]
fn app_cli_workflow_uses_app_target_dir() {
    let root = Path::new("/repo");
    assert_eq!(
        target_dir_for(root, Workflow::AppCli),
        root.join("target/apps/aicore-cli")
    );
}

#[test]
fn app_tui_workflow_uses_app_target_dir() {
    let root = Path::new("/repo");
    assert_eq!(
        target_dir_for(root, Workflow::AppTui),
        root.join("target/apps/aicore-tui")
    );
}

#[test]
fn foundation_install_manifest_path_is_under_install_dir() {
    let target_dir = PathBuf::from("/repo/target/layers/foundation");
    assert_eq!(
        install_manifest_for(&target_dir),
        PathBuf::from("/repo/target/layers/foundation/install/install.toml")
    );
}

#[test]
fn app_workflow_installs_binary_into_aicore_bin() {
    let home_root = Path::new("/home/demo");
    assert_eq!(
        install_bin_dir_for(home_root),
        PathBuf::from("/home/demo/.aicore/bin")
    );
    assert_eq!(
        installed_binary_path(home_root, Workflow::AppCli),
        PathBuf::from("/home/demo/.aicore/bin/aicore-cli")
    );
}

#[test]
fn workflow_install_warns_when_aicore_bin_not_in_path() {
    let home_root = Path::new("/home/demo");
    let warnings = install_visibility_warnings(home_root, "/usr/bin:/bin", |path| {
        matches!(
            path.to_str(),
            Some("/home/demo/.aicore/bin/aicore-cli")
                | Some("/home/demo/.aicore/bin/aicore")
                | Some("/home/demo/.aicore/bin/aicore-tui")
        )
    });

    let message = warnings
        .iter()
        .map(|warning| warning.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(message.contains("~/.aicore/bin 当前不在 PATH"));
    assert!(message.contains("/home/demo/.aicore/bin/aicore-cli"));
    assert!(message.contains("请先运行 cargo foundation"));
}

#[test]
fn foundation_workflow_runs_shell_path_bootstrap() {
    let home_root = temp_home("foundation-bootstrap");
    let target_dir = fake_app_target("foundation-target", "aicore-foundation");
    let outcome =
        install_layer_with_shell_env(Workflow::Foundation, &target_dir, &bash_env(&home_root))
            .expect("foundation install should succeed");
    let bashrc = fs::read_to_string(home_root.join(".bashrc")).expect("read bashrc");

    assert!(outcome.shell_bootstrap.is_some());
    assert!(bashrc.contains(MANAGED_BLOCK_START));
    assert!(bashrc.contains(MANAGED_PATH_LINE));
    assert!(bashrc.contains(MANAGED_BLOCK_END));
}

#[test]
fn foundation_install_writes_global_runtime_metadata() {
    let home_root = temp_home("foundation-runtime");
    let target_dir = fake_app_target("foundation-runtime-target", "aicore-foundation");
    install_layer_with_shell_env(Workflow::Foundation, &target_dir, &bash_env(&home_root))
        .expect("foundation install should succeed");

    for file in [
        "install.toml",
        "version.toml",
        "primitives.toml",
        "terminal.toml",
        "paths.toml",
    ] {
        assert!(
            home_root
                .join(".aicore/runtime/foundation")
                .join(file)
                .exists(),
            "{file} should be installed under global foundation runtime"
        );
    }
}

#[test]
fn kernel_install_writes_global_runtime_metadata() {
    let home_root = temp_home("kernel-runtime");
    let target_dir = fake_app_target("kernel-runtime-target", "aicore-kernel");
    install_layer_with_shell_env(Workflow::Kernel, &target_dir, &bash_env(&home_root))
        .expect("kernel install should succeed");

    for file in [
        "install.toml",
        "version.toml",
        "contracts.toml",
        "capabilities.toml",
        "registry.toml",
        "routing.toml",
        "scheduler.toml",
    ] {
        assert!(
            home_root.join(".aicore/runtime/kernel").join(file).exists(),
            "{file} should be installed under global kernel runtime"
        );
    }
}

#[test]
fn foundation_runtime_binary_is_installed_by_cargo_foundation() {
    let home_root = temp_home("foundation-runtime-binary");
    let target_dir = fake_app_target("foundation-runtime-binary-target", "aicore-foundation");
    install_layer_with_shell_env(Workflow::Foundation, &target_dir, &bash_env(&home_root))
        .expect("foundation install should succeed");

    assert!(home_root.join(".aicore/bin/aicore-foundation").exists());
}

#[test]
fn kernel_runtime_binary_is_installed_by_cargo_kernel() {
    let home_root = temp_home("kernel-runtime-binary");
    let target_dir = fake_app_target("kernel-runtime-binary-target", "aicore-kernel");
    install_layer_with_shell_env(Workflow::Kernel, &target_dir, &bash_env(&home_root))
        .expect("kernel install should succeed");

    assert!(home_root.join(".aicore/bin/aicore-kernel").exists());
}

#[test]
fn runtime_install_metadata_records_foundation_binary() {
    let home_root = temp_home("foundation-runtime-binary-metadata");
    let target_dir = fake_app_target(
        "foundation-runtime-binary-metadata-target",
        "aicore-foundation",
    );
    install_layer_with_shell_env(Workflow::Foundation, &target_dir, &bash_env(&home_root))
        .expect("foundation install should succeed");

    let metadata = fs::read_to_string(
        home_root
            .join(".aicore/runtime/foundation")
            .join("install.toml"),
    )
    .expect("foundation install metadata should exist");
    assert!(metadata.contains("runtime_binary_path = \""));
    assert!(metadata.contains("aicore-foundation"));
    assert!(metadata.contains("runtime_binary_installed = true"));
    assert!(metadata.contains("runtime_protocol = \"stdio_jsonl\""));
}

#[test]
fn runtime_install_metadata_records_kernel_binary() {
    let home_root = temp_home("kernel-runtime-binary-metadata");
    let target_dir = fake_app_target("kernel-runtime-binary-metadata-target", "aicore-kernel");
    install_layer_with_shell_env(Workflow::Kernel, &target_dir, &bash_env(&home_root))
        .expect("kernel install should succeed");

    let metadata = fs::read_to_string(
        home_root
            .join(".aicore/runtime/kernel")
            .join("install.toml"),
    )
    .expect("kernel install metadata should exist");
    assert!(metadata.contains("runtime_binary_path = \""));
    assert!(metadata.contains("aicore-kernel"));
    assert!(metadata.contains("runtime_binary_installed = true"));
    assert!(metadata.contains("runtime_protocol = \"stdio_jsonl\""));
}

#[test]
fn global_runtime_layout_creates_expected_directories() {
    let home_root = temp_home("global-runtime-dirs");
    let target_dir = fake_app_target("global-runtime-dirs-target", "aicore-kernel");
    install_layer_with_shell_env(Workflow::Kernel, &target_dir, &bash_env(&home_root))
        .expect("kernel install should succeed");

    assert!(home_root.join(".aicore/share/manifests").is_dir());
    assert!(home_root.join(".aicore/state/kernel").is_dir());
}

#[test]
fn global_runtime_metadata_uses_atomic_write() {
    let home_root = temp_home("global-runtime-atomic");
    let target_dir = fake_app_target("global-runtime-atomic-target", "aicore-foundation");
    install_layer_with_shell_env(Workflow::Foundation, &target_dir, &bash_env(&home_root))
        .expect("foundation install should succeed");

    let runtime_dir = home_root.join(".aicore/runtime/foundation");
    let temp_files = fs::read_dir(&runtime_dir)
        .expect("runtime dir should exist")
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().contains(".tmp"))
        .collect::<Vec<_>>();

    assert!(temp_files.is_empty(), "atomic temp files should not remain");
    assert!(runtime_dir.join("install.toml").exists());
}

#[test]
fn app_install_warning_points_to_shell_reload_when_path_not_active() {
    let home_root = temp_home("app-reload-warning");
    fs::write(
        home_root.join(".bashrc"),
        format!("{MANAGED_BLOCK_START}\n{MANAGED_PATH_LINE}\n{MANAGED_BLOCK_END}\n"),
    )
    .expect("write bashrc");
    let warnings = install_visibility_warnings(
        &home_root,
        "/usr/bin:/bin",
        |path| matches!(path.to_str(), Some(value) if value.ends_with("/.aicore/bin/aicore")),
    );
    let message = warnings
        .iter()
        .map(|warning| warning.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    assert!(message.contains("底层 shell bootstrap 已提供永久配置"));
    assert!(message.contains("当前 shell 可能尚未 reload"));
    assert!(message.contains("source ~/.bashrc && hash -r"));
}

#[test]
fn app_install_warning_points_to_foundation_when_managed_block_missing() {
    let home_root = temp_home("app-foundation-warning");
    let warnings = install_visibility_warnings(
        &home_root,
        "/usr/bin:/bin",
        |path| matches!(path.to_str(), Some(value) if value.ends_with("/.aicore/bin/aicore")),
    );
    let message = warnings
        .iter()
        .map(|warning| warning.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    assert!(message.contains("请先运行 cargo foundation"));
}

#[test]
fn app_aicore_install_writes_global_manifest() {
    let home_root = temp_home("app-aicore-manifest");
    let target_dir = fake_app_target("app-aicore-target", "aicore");
    install_layer_with_shell_env(Workflow::AppAicore, &target_dir, &bash_env(&home_root))
        .expect("app-aicore install should succeed");
    let manifest = fs::read_to_string(home_root.join(".aicore/share/manifests/aicore.toml"))
        .expect("aicore manifest should exist");

    assert!(manifest.contains("component_id = \"aicore\""));
    assert!(manifest.contains("app_id = \"aicore\""));
    assert!(manifest.contains("entrypoint = \""));
    assert!(manifest.contains("[[capabilities]]"));
    assert!(manifest.contains("operation = \"runtime.status\""));
}

#[test]
fn app_cli_install_writes_global_manifest_with_capabilities() {
    let home_root = temp_home("app-cli-manifest");
    let target_dir = fake_app_target("app-cli-target", "aicore-cli");
    install_layer_with_shell_env(Workflow::AppCli, &target_dir, &bash_env(&home_root))
        .expect("app-cli install should succeed");
    let manifest = fs::read_to_string(home_root.join(".aicore/share/manifests/aicore-cli.toml"))
        .expect("aicore-cli manifest should exist");

    assert!(manifest.contains("component_id = \"aicore-cli\""));
    assert!(manifest.contains("app_id = \"aicore-cli\""));
    assert!(manifest.contains("kind = \"app\""));
    assert!(manifest.contains("contract_version = \"kernel.app.v1\""));
    assert!(manifest.contains("operation = \"memory.status\""));
    assert!(manifest.contains("operation = \"memory.search\""));
    assert!(manifest.contains("operation = \"provider.smoke\""));
}

#[test]
fn app_cli_install_writes_process_smoke_manifest() {
    let home_root = temp_home("app-cli-process-smoke-manifest");
    let target_dir = fake_app_target("app-cli-process-smoke-target", "aicore-cli");
    install_layer_with_shell_env(Workflow::AppCli, &target_dir, &bash_env(&home_root))
        .expect("app-cli install should succeed");
    let manifest =
        fs::read_to_string(home_root.join(".aicore/share/manifests/aicore-component-smoke.toml"))
            .expect("process smoke manifest should exist");

    assert!(manifest.contains("component_id = \"aicore-component-smoke\""));
    assert!(manifest.contains("app_id = \"aicore-cli\""));
    assert!(manifest.contains("invocation_mode = \"local_process\""));
    assert!(manifest.contains("transport = \"stdio_jsonl\""));
    assert!(manifest.contains("args = [\"__component-smoke-stdio\"]"));
    assert!(manifest.contains("operation = \"component.process.smoke\""));
}

#[test]
fn app_cli_install_writes_config_validate_process_manifest() {
    let home_root = temp_home("app-cli-config-validate-process-manifest");
    let target_dir = fake_app_target("app-cli-config-validate-target", "aicore-cli");
    install_layer_with_shell_env(Workflow::AppCli, &target_dir, &bash_env(&home_root))
        .expect("app-cli install should succeed");
    let manifest =
        fs::read_to_string(home_root.join(".aicore/share/manifests/aicore-config-validate.toml"))
            .expect("config validate manifest should exist");
    let cli_manifest =
        fs::read_to_string(home_root.join(".aicore/share/manifests/aicore-cli.toml"))
            .expect("aicore-cli manifest should exist");

    assert!(manifest.contains("component_id = \"aicore-config-validate\""));
    assert!(manifest.contains("app_id = \"aicore-cli\""));
    assert!(manifest.contains("invocation_mode = \"local_process\""));
    assert!(manifest.contains("transport = \"stdio_jsonl\""));
    assert!(manifest.contains("args = [\"__component-config-validate-stdio\"]"));
    assert!(manifest.contains("operation = \"config.validate\""));
    assert!(!cli_manifest.contains("operation = \"config.validate\""));
}

#[test]
fn app_cli_install_writes_auth_model_service_process_manifests() {
    let home_root = temp_home("app-cli-auth-model-service-process-manifest");
    let target_dir = fake_app_target("app-cli-auth-model-service-target", "aicore-cli");
    install_layer_with_shell_env(Workflow::AppCli, &target_dir, &bash_env(&home_root))
        .expect("app-cli install should succeed");
    let cli_manifest =
        fs::read_to_string(home_root.join(".aicore/share/manifests/aicore-cli.toml"))
            .expect("aicore-cli manifest should exist");

    for (file_name, component_id, operation, arg) in [
        (
            "aicore-auth-list.toml",
            "aicore-auth-list",
            "auth.list",
            "__component-auth-list-stdio",
        ),
        (
            "aicore-model-show.toml",
            "aicore-model-show",
            "model.show",
            "__component-model-show-stdio",
        ),
        (
            "aicore-service-list.toml",
            "aicore-service-list",
            "service.list",
            "__component-service-list-stdio",
        ),
    ] {
        let manifest =
            fs::read_to_string(home_root.join(".aicore/share/manifests").join(file_name))
                .unwrap_or_else(|_| panic!("{file_name} should exist"));

        assert!(manifest.contains(&format!("component_id = \"{component_id}\"")));
        assert!(manifest.contains("app_id = \"aicore-cli\""));
        assert!(manifest.contains("invocation_mode = \"local_process\""));
        assert!(manifest.contains("transport = \"stdio_jsonl\""));
        assert!(manifest.contains(&format!("args = [\"{arg}\"]")));
        assert!(manifest.contains(&format!("operation = \"{operation}\"")));
        assert!(!cli_manifest.contains(&format!("operation = \"{operation}\"")));
    }
}

#[test]
fn app_tui_install_writes_global_manifest() {
    let home_root = temp_home("app-tui-manifest");
    let target_dir = fake_app_target("app-tui-target", "aicore-tui");
    install_layer_with_shell_env(Workflow::AppTui, &target_dir, &bash_env(&home_root))
        .expect("app-tui install should succeed");
    let manifest = fs::read_to_string(home_root.join(".aicore/share/manifests/aicore-tui.toml"))
        .expect("aicore-tui manifest should exist");

    assert!(manifest.contains("component_id = \"aicore-tui\""));
    assert!(manifest.contains("operation = \"tui.session\""));
}

#[test]
fn workflow_install_warns_when_command_is_shadowed_by_local_bin() {
    let home_root = Path::new("/home/demo");
    let warnings = install_visibility_warnings(
        home_root,
        "/home/demo/.local/bin:/usr/bin:/home/demo/.aicore/bin",
        |path| {
            matches!(
                path.to_str(),
                Some("/home/demo/.local/bin/aicore")
                    | Some("/home/demo/.aicore/bin/aicore")
                    | Some("/home/demo/.aicore/bin/aicore-cli")
                    | Some("/home/demo/.aicore/bin/aicore-tui")
            )
        },
    );

    let message = warnings
        .iter()
        .map(|warning| warning.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(message.contains("检测到命令 shadowing"));
    assert!(message.contains("当前 shell 的 `aicore` 指向 `/home/demo/.local/bin/aicore`"));
    assert!(message.contains("新安装的 AICore OS 位于 `/home/demo/.aicore/bin/aicore`"));
    assert!(message.contains("请将 `$HOME/.aicore/bin` 放到 PATH 前面"));
}

#[test]
fn workflow_install_reports_installed_binary_paths() {
    let home_root = Path::new("/home/demo");
    let warnings = install_visibility_warnings(home_root, "/usr/bin:/bin", |path| {
        matches!(
            path.to_str(),
            Some("/home/demo/.aicore/bin/aicore-cli") | Some("/home/demo/.aicore/bin/aicore")
        )
    });

    let message = warnings
        .iter()
        .map(|warning| warning.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(message.contains("/home/demo/.aicore/bin/aicore"));
    assert!(message.contains("/home/demo/.aicore/bin/aicore-cli"));
}

#[test]
fn workflow_install_does_not_delete_existing_local_bin_binary() {
    let home_root = Path::new("/home/demo");
    let warnings = install_visibility_warnings(
        home_root,
        "/home/demo/.local/bin:/home/demo/.aicore/bin",
        |path| {
            matches!(
                path.to_str(),
                Some("/home/demo/.local/bin/aicore") | Some("/home/demo/.aicore/bin/aicore")
            )
        },
    );

    let message = warnings
        .iter()
        .map(|warning| warning.message.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(message.contains("/home/demo/.local/bin/aicore"));
    assert!(!message.contains("删除"));
    assert!(!message.contains("覆盖"));
}

#[test]
fn provider_workflow_does_not_require_live_sdk_by_default() {
    let args = cargo_args_for_workflow(Workflow::AppCli, "test");

    assert!(args.contains(&"--offline".to_string()));
    assert!(!args.iter().any(|arg| arg.contains("OPENAI_API_KEY")));
    assert!(!args.iter().any(|arg| arg.contains("ANTHROPIC_API_KEY")));
}

#[test]
fn formal_provider_doc_exists() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir
        .ancestors()
        .nth(3)
        .expect("workflow crate should live under crates/workflows");
    let doc = repo_root
        .join("docs")
        .join("architecture")
        .join("AICore-OS-Provider请求应用规范.md");

    assert!(doc.exists());
}

#[test]
fn formal_terminal_doc_exists() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir
        .ancestors()
        .nth(3)
        .expect("workflow crate should live under crates/workflows");
    let doc = repo_root
        .join("docs")
        .join("architecture")
        .join("AICore-OS-终端输出规范.md");

    assert!(doc.exists());
}

#[test]
fn formal_docs_do_not_include_stage_journal_terms() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir
        .ancestors()
        .nth(3)
        .expect("workflow crate should live under crates/workflows");
    let docs = [
        repo_root
            .join("docs")
            .join("architecture")
            .join("AICore-OS-内核协议规范.md"),
        repo_root
            .join("docs")
            .join("architecture")
            .join("AICore-OS-运行时安装布局规范.md"),
    ];
    let forbidden_terms = [
        "提交",
        "验证命令",
        "本轮",
        "修正原因",
        "之前不合适",
        "checklist",
        "下一步",
    ];

    for doc in docs {
        let content = std::fs::read_to_string(&doc).expect("formal doc should be readable");
        for term in forbidden_terms {
            assert!(
                !content.contains(term),
                "{} should not contain journal term {term}",
                doc.display()
            );
        }
    }
}

#[test]
fn cargo_workflow_aliases_use_quiet_run() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir
        .ancestors()
        .nth(3)
        .expect("workflow crate should live under crates/workflows");
    let config = std::fs::read_to_string(repo_root.join(".cargo/config.toml"))
        .expect("cargo config should be readable");

    for alias in [
        "foundation",
        "kernel",
        "core",
        "app-aicore",
        "app-cli",
        "app-tui",
    ] {
        assert!(
            config.contains(&format!(
                "{alias} = \"run --quiet -p aicore-workflow -- {alias}\""
            )),
            "{alias} alias should use cargo run --quiet"
        );
    }
}

fn bash_env(home: &Path) -> ShellPathBootstrapEnv {
    ShellPathBootstrapEnv {
        home: Some(home.to_path_buf()),
        shell: Some("/bin/bash".to_string()),
        path: "/usr/bin:/bin".to_string(),
        ci: false,
    }
}

fn temp_home(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "aicore-runner-{name}-{}-{unique}",
        std::process::id()
    ));
    fs::create_dir_all(&path).expect("create temp dir");
    path
}

fn fake_app_target(name: &str, binary: &str) -> PathBuf {
    let path = temp_home(name);
    let debug = path.join("debug");
    fs::create_dir_all(&debug).expect("create debug dir");
    fs::write(debug.join(binary), "fake binary").expect("write fake binary");
    path
}
