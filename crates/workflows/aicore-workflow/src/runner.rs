use std::ffi::OsString;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::Instant;

use aicore_foundation::AicoreLayout;
use aicore_terminal::{Status, WarningDiagnostic};

use crate::cargo_runner::{CommandReport, run_cargo_capture};
use crate::layers::Workflow;
use crate::runtime_install::{install_app_manifest, install_global_runtime_metadata};
use crate::shell_integration::{
    ShellPathBootstrapEnv, ShellPathBootstrapResult, ShellPathBootstrapStatus,
    bootstrap_shell_path, has_managed_path_block,
};
use crate::workflow_output::WorkflowOutput;

const TARGET_LIMIT_BYTES: u64 = 30 * 1024 * 1024 * 1024;

pub fn run(workflow: Workflow) -> Result<(), String> {
    let repo_root = find_repo_root()?;
    let mut output =
        WorkflowOutput::from_current(workflow.id(), &repo_root, workflow.target_label());
    output.start();

    let result = match workflow {
        Workflow::Core => {
            run_single(&repo_root, Workflow::Foundation, &mut output)?;
            run_single(&repo_root, Workflow::Kernel, &mut output)?;
            Ok(())
        }
        Workflow::Foundation
        | Workflow::Kernel
        | Workflow::AppAicore
        | Workflow::AppCli
        | Workflow::AppTui => run_single(&repo_root, workflow, &mut output),
    };

    let final_status = match result {
        Ok(()) if output.warning_count() > 0 => Status::Warn,
        Ok(()) => Status::Ok,
        Err(_) => Status::Failed,
    };
    let finish_result = output.finish(final_status);
    match (result, finish_result) {
        (Err(error), _) => Err(error),
        (Ok(()), Err(error)) => Err(error),
        (Ok(()), Ok(())) => Ok(()),
    }
}

fn run_single(
    repo_root: &Path,
    workflow: Workflow,
    output: &mut WorkflowOutput,
) -> Result<(), String> {
    let target_dir = target_dir_for(repo_root, workflow);
    cleanup_target_if_needed(&target_dir, output)?;
    run_cargo_step(
        output,
        repo_root,
        None,
        workflow.id(),
        "fmt",
        "cargo fmt --check",
        &["fmt", "--check"],
    )?;
    run_cargo_for_workflow(output, repo_root, workflow, &target_dir, "test")?;
    run_cargo_for_workflow(output, repo_root, workflow, &target_dir, "build")?;
    output.step_started(&format!("{} / install", workflow.id()));
    let install_started_at = Instant::now();
    let install_outcome = install_layer(workflow, &target_dir)?;
    if let Some(shell_bootstrap) = &install_outcome.shell_bootstrap {
        output.record_shell_path_bootstrap(shell_bootstrap);
    }
    let install_warning_count = install_outcome.warnings.len();
    for warning in install_outcome.warnings {
        output.record_warning(warning);
    }
    if install_warning_count > 0 {
        output.record_local_step_with_warning_count(
            workflow.id(),
            "install",
            "install",
            Status::Warn,
            install_started_at.elapsed(),
            install_warning_count,
        );
    } else {
        output.record_local_step(
            workflow.id(),
            "install",
            "install",
            Status::Ok,
            install_started_at.elapsed(),
        );
    }
    Ok(())
}

fn run_cargo_for_workflow(
    output: &mut WorkflowOutput,
    repo_root: &Path,
    workflow: Workflow,
    target_dir: &Path,
    subcommand: &str,
) -> Result<(), String> {
    let args = cargo_args_for_workflow(workflow, subcommand);
    let arg_refs = args.iter().map(String::as_str).collect::<Vec<_>>();
    let command = format!("cargo {}", arg_refs.join(" "));
    run_cargo_step(
        output,
        repo_root,
        Some(target_dir),
        workflow.id(),
        subcommand,
        &command,
        &arg_refs,
    )
}

fn cargo_args_for_workflow(workflow: Workflow, subcommand: &str) -> Vec<String> {
    let mut args = vec![subcommand.to_string()];
    for crate_name in workflow.crates() {
        args.push("-p".to_string());
        args.push((*crate_name).to_string());
    }
    args.push("--offline".to_string());
    args
}

fn run_cargo_step(
    output: &mut WorkflowOutput,
    repo_root: &Path,
    target_dir: Option<&Path>,
    layer: &str,
    step: &str,
    command: &str,
    args: &[&str],
) -> Result<(), String> {
    output.step_started(&format!("{layer} / {step}"));
    let report = run_cargo_capture(repo_root, target_dir, args)?;
    let succeeded = report.succeeded();
    output.record_command_report(layer, step, command, &report, !succeeded);
    if succeeded {
        Ok(())
    } else {
        Err(render_cargo_failure(&report))
    }
}

fn render_cargo_failure(report: &CommandReport) -> String {
    format!("{} 执行失败。", report.command)
}

fn cleanup_target_if_needed(target_dir: &Path, output: &WorkflowOutput) -> Result<(), String> {
    if !target_dir.exists() {
        return Ok(());
    }

    let size = dir_size(target_dir)?;
    if size > TARGET_LIMIT_BYTES {
        output.message(&format!(
            "{} 超过 30GiB，正在清理后重新编译。",
            target_dir.display()
        ));
        fs::remove_dir_all(target_dir)
            .map_err(|error| format!("删除 {} 失败: {error}", target_dir.display()))?;
    }

    Ok(())
}

fn target_dir_for(repo_root: &Path, workflow: Workflow) -> PathBuf {
    match workflow {
        Workflow::Foundation => repo_root.join("target/layers/foundation"),
        Workflow::Kernel => repo_root.join("target/layers/kernel"),
        Workflow::Core => unreachable!("core should run foundation + kernel separately"),
        Workflow::AppAicore => repo_root.join("target/apps/aicore"),
        Workflow::AppCli => repo_root.join("target/apps/aicore-cli"),
        Workflow::AppTui => repo_root.join("target/apps/aicore-tui"),
    }
}

fn dir_size(path: &Path) -> Result<u64, String> {
    let mut total = 0u64;
    let entries = fs::read_dir(path).map_err(|error| format!("读取目录失败: {error}"))?;
    for entry in entries {
        let entry = entry.map_err(|error| format!("读取目录项失败: {error}"))?;
        let metadata = entry
            .metadata()
            .map_err(|error| format!("读取元数据失败: {error}"))?;
        if metadata.is_dir() {
            total += dir_size(&entry.path())?;
        } else {
            total += metadata.len();
        }
    }
    Ok(total)
}

fn find_repo_root() -> Result<PathBuf, String> {
    let mut current =
        std::env::current_dir().map_err(|error| format!("读取当前目录失败: {error}"))?;
    loop {
        if current.join("Cargo.toml").exists() && current.join("crates").exists() {
            return Ok(current);
        }
        if !current.pop() {
            return Err("未找到仓库根目录。".to_string());
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct InstallOutcome {
    warnings: Vec<WarningDiagnostic>,
    shell_bootstrap: Option<ShellPathBootstrapResult>,
}

fn install_layer(workflow: Workflow, target_dir: &Path) -> Result<InstallOutcome, String> {
    install_layer_with_shell_env(workflow, target_dir, &ShellPathBootstrapEnv::current())
}

fn install_layer_with_shell_env(
    workflow: Workflow,
    target_dir: &Path,
    shell_env: &ShellPathBootstrapEnv,
) -> Result<InstallOutcome, String> {
    let mut warnings = Vec::new();
    let mut shell_bootstrap = None;
    if matches!(
        workflow,
        Workflow::AppAicore | Workflow::AppCli | Workflow::AppTui
    ) {
        let layout = layout_from_shell_env(shell_env)?;
        warnings.extend(install_app_binary(
            workflow,
            target_dir,
            &layout,
            &shell_env.path,
        )?);
    } else if matches!(workflow, Workflow::Foundation | Workflow::Kernel) {
        let layout = layout_from_shell_env(shell_env)?;
        install_global_runtime_metadata(workflow, &layout)?;
        if workflow == Workflow::Foundation {
            let result = bootstrap_shell_path(shell_env);
            if let Some(warning) = shell_bootstrap_warning(&result) {
                warnings.push(warning);
            }
            shell_bootstrap = Some(result);
        }
    }

    let manifest_path = install_manifest_for(target_dir);
    if let Some(parent) = manifest_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("创建安装目录 {} 失败: {error}", parent.display()))?;
    }

    let content = render_install_manifest(workflow, target_dir);
    fs::write(&manifest_path, content)
        .map_err(|error| format!("写入安装记录 {} 失败: {error}", manifest_path.display()))?;
    Ok(InstallOutcome {
        warnings,
        shell_bootstrap,
    })
}

fn layout_from_shell_env(shell_env: &ShellPathBootstrapEnv) -> Result<AicoreLayout, String> {
    shell_env
        .home
        .clone()
        .map(AicoreLayout::new)
        .ok_or_else(|| "HOME 不可用，无法安装全局 runtime metadata。".to_string())
}

fn shell_bootstrap_warning(result: &ShellPathBootstrapResult) -> Option<WarningDiagnostic> {
    match result.status {
        ShellPathBootstrapStatus::Failed | ShellPathBootstrapStatus::UnsupportedShell => {
            Some(WarningDiagnostic::new(
                "install",
                &format!(
                    "Shell PATH bootstrap 未完成。\n状态：{}\n说明：{}",
                    result.status.label(),
                    result.message.as_deref().unwrap_or("无附加说明")
                ),
            ))
        }
        ShellPathBootstrapStatus::AlreadyConfigured
        | ShellPathBootstrapStatus::Appended
        | ShellPathBootstrapStatus::Updated
        | ShellPathBootstrapStatus::SkippedCi => None,
    }
}

fn install_manifest_for(target_dir: &Path) -> PathBuf {
    target_dir.join("install/install.toml")
}

fn install_bin_dir_for(home_root: &Path) -> PathBuf {
    home_root.join(".aicore/bin")
}

fn installed_binary_path(home_root: &Path, workflow: Workflow) -> PathBuf {
    install_bin_dir_for(home_root).join(binary_name_for(workflow))
}

fn built_binary_path(target_dir: &Path, workflow: Workflow) -> PathBuf {
    target_dir.join("debug").join(binary_name_for(workflow))
}

fn binary_name_for(workflow: Workflow) -> &'static str {
    match workflow {
        Workflow::AppAicore => "aicore",
        Workflow::AppCli => "aicore-cli",
        Workflow::AppTui => "aicore-tui",
        Workflow::Foundation | Workflow::Kernel | Workflow::Core => {
            unreachable!("non-app workflows do not install binaries")
        }
    }
}

const INSTALLED_COMMANDS: [&str; 3] = ["aicore", "aicore-cli", "aicore-tui"];

fn install_app_binary(
    workflow: Workflow,
    target_dir: &Path,
    layout: &AicoreLayout,
    path_env: &str,
) -> Result<Vec<WarningDiagnostic>, String> {
    let install_dir = install_bin_dir_for(&layout.home_root);
    fs::create_dir_all(&install_dir)
        .map_err(|error| format!("创建应用安装目录 {} 失败: {error}", install_dir.display()))?;

    let source_path = built_binary_path(target_dir, workflow);
    if !source_path.exists() {
        return Err(format!("未找到待安装二进制: {}", source_path.display()));
    }

    let target_path = installed_binary_path(&layout.home_root, workflow);
    fs::copy(&source_path, &target_path).map_err(|error| {
        format!(
            "复制二进制 {} -> {} 失败: {error}",
            source_path.display(),
            target_path.display()
        )
    })?;

    #[cfg(unix)]
    {
        let mut permissions = fs::metadata(&target_path)
            .map_err(|error| format!("读取安装后二进制权限失败: {error}"))?
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&target_path, permissions)
            .map_err(|error| format!("设置二进制可执行权限失败: {error}"))?;
    }

    install_app_manifest(workflow, layout, &target_path)?;

    Ok(install_visibility_warnings(
        &layout.home_root,
        path_env,
        Path::exists,
    ))
}

fn install_visibility_warnings(
    home_root: &Path,
    path_env: &str,
    exists: impl Fn(&Path) -> bool,
) -> Vec<WarningDiagnostic> {
    let install_dir = install_bin_dir_for(home_root);
    let installed = INSTALLED_COMMANDS
        .iter()
        .map(|command| (*command, install_dir.join(command)))
        .filter(|(_, path)| exists(path))
        .collect::<Vec<_>>();
    let mut warnings = Vec::new();

    if !path_contains_dir(path_env, &install_dir) {
        let installed_paths = if installed.is_empty() {
            format!("- {}", install_dir.display())
        } else {
            installed
                .iter()
                .map(|(_, path)| format!("- {}", path.display()))
                .collect::<Vec<_>>()
                .join("\n")
        };
        warnings.push(WarningDiagnostic::new(
            "install",
            &format!(
                "~/.aicore/bin 当前不在 PATH。\n当前安装的二进制路径：\n{installed_paths}\n{}\n重新加载命令：source ~/.bashrc && hash -r",
                if has_managed_path_block(home_root) {
                    "底层 shell bootstrap 已提供永久配置；当前 shell 可能尚未 reload。"
                } else {
                    "请先运行 cargo foundation 或 foundation shell bootstrap。"
                }
            ),
        ));
    }

    for (command, installed_path) in installed {
        if let Some(resolved_path) = resolve_command_in_path(command, path_env, &exists) {
            if resolved_path != installed_path {
                warnings.push(WarningDiagnostic::new(
                    "install",
                    &format!(
                        "检测到命令 shadowing：\n当前 shell 的 `{command}` 指向 `{}`。\n新安装的 AICore OS 位于 `{}`。\n请将 `$HOME/.aicore/bin` 放到 PATH 前面，或清理旧的 `{}`。",
                        resolved_path.display(),
                        installed_path.display(),
                        resolved_path.display()
                    ),
                ));
            }
        }
    }

    warnings
}

fn path_contains_dir(path_env: &str, expected_dir: &Path) -> bool {
    path_entries(path_env)
        .iter()
        .any(|entry| entry == expected_dir)
}

fn resolve_command_in_path(
    command: &str,
    path_env: &str,
    exists: impl Fn(&Path) -> bool,
) -> Option<PathBuf> {
    path_entries(path_env)
        .into_iter()
        .map(|entry| entry.join(command))
        .find(|candidate| exists(candidate))
}

fn path_entries(path_env: &str) -> Vec<PathBuf> {
    std::env::split_paths(&OsString::from(path_env)).collect()
}

fn render_install_manifest(workflow: Workflow, target_dir: &Path) -> String {
    let target_dir_escaped = target_dir.display().to_string();
    let packages = workflow
        .crates()
        .iter()
        .map(|pkg| format!("\"{pkg}\""))
        .collect::<Vec<_>>()
        .join(", ");

    format!(
        "layer = \"{}\"\nstatus = \"installed\"\ntarget_dir = \"{}\"\npackages = [{}]\n",
        match workflow {
            Workflow::Foundation => "foundation",
            Workflow::Kernel => "kernel",
            Workflow::Core => unreachable!("core should not render install manifest"),
            Workflow::AppAicore => "app-aicore",
            Workflow::AppCli => "app-cli",
            Workflow::AppTui => "app-tui",
        },
        target_dir_escaped,
        packages
    )
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::layers::Workflow;
    use crate::shell_integration::{
        MANAGED_BLOCK_END, MANAGED_BLOCK_START, MANAGED_PATH_LINE, ShellPathBootstrapEnv,
    };

    use super::{
        cargo_args_for_workflow, install_bin_dir_for, install_layer_with_shell_env,
        install_manifest_for, install_visibility_warnings, installed_binary_path, target_dir_for,
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
        let target_dir = temp_home("foundation-target");
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
        let target_dir = temp_home("foundation-runtime-target");
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
        let target_dir = temp_home("kernel-runtime-target");
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
    fn global_runtime_layout_creates_expected_directories() {
        let home_root = temp_home("global-runtime-dirs");
        let target_dir = temp_home("global-runtime-dirs-target");
        install_layer_with_shell_env(Workflow::Kernel, &target_dir, &bash_env(&home_root))
            .expect("kernel install should succeed");

        assert!(home_root.join(".aicore/share/manifests").is_dir());
        assert!(home_root.join(".aicore/state/kernel").is_dir());
    }

    #[test]
    fn global_runtime_metadata_uses_atomic_write() {
        let home_root = temp_home("global-runtime-atomic");
        let target_dir = temp_home("global-runtime-atomic-target");
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
        let warnings = install_visibility_warnings(&home_root, "/usr/bin:/bin", |path| {
            matches!(
                path.to_str(),
                Some(value) if value.ends_with("/.aicore/bin/aicore")
            )
        });
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
        let warnings = install_visibility_warnings(&home_root, "/usr/bin:/bin", |path| {
            matches!(
                path.to_str(),
                Some(value) if value.ends_with("/.aicore/bin/aicore")
            )
        });
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
        let manifest =
            fs::read_to_string(home_root.join(".aicore/share/manifests/aicore-cli.toml"))
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
    fn app_tui_install_writes_global_manifest() {
        let home_root = temp_home("app-tui-manifest");
        let target_dir = fake_app_target("app-tui-target", "aicore-tui");
        install_layer_with_shell_env(Workflow::AppTui, &target_dir, &bash_env(&home_root))
            .expect("app-tui install should succeed");
        let manifest =
            fs::read_to_string(home_root.join(".aicore/share/manifests/aicore-tui.toml"))
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
}
