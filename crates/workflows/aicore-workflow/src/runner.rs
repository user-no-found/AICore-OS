use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use aicore_foundation::AicoreLayout;
use aicore_terminal::Status;

use crate::cargo_runner::{CommandReport, run_cargo_capture};
use crate::layers::Workflow;
use crate::workflow_output::WorkflowOutput;

const TARGET_LIMIT_BYTES: u64 = 30 * 1024 * 1024 * 1024;

pub fn run(workflow: Workflow) -> Result<(), String> {
    let repo_root = find_repo_root()?;
    let mut output = WorkflowOutput::from_current(workflow.label_zh());
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
        "cargo fmt --check",
        &["fmt", "--check"],
    )?;
    run_cargo_for_workflow(output, repo_root, workflow, &target_dir, "test")?;
    run_cargo_for_workflow(output, repo_root, workflow, &target_dir, "build")?;
    output.step_started(&format!("install {}", workflow.label_zh()));
    install_layer(workflow, &target_dir)?;
    output.record_local_step(&format!("install {}", workflow.label_zh()), Status::Ok);
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
    let step_name = format!("cargo {subcommand} {}", workflow.label_zh());
    run_cargo_step(output, repo_root, Some(target_dir), &step_name, &arg_refs)
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
    step_name: &str,
    args: &[&str],
) -> Result<(), String> {
    output.step_started(step_name);
    let report = run_cargo_capture(repo_root, target_dir, args)?;
    let succeeded = report.succeeded();
    output.record_command_report(step_name, &report, !succeeded);
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

fn install_layer(workflow: Workflow, target_dir: &Path) -> Result<(), String> {
    if matches!(
        workflow,
        Workflow::AppAicore | Workflow::AppCli | Workflow::AppTui
    ) {
        install_app_binary(workflow, target_dir)?;
    }

    let manifest_path = install_manifest_for(target_dir);
    if let Some(parent) = manifest_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("创建安装目录 {} 失败: {error}", parent.display()))?;
    }

    let content = render_install_manifest(workflow, target_dir);
    fs::write(&manifest_path, content)
        .map_err(|error| format!("写入安装记录 {} 失败: {error}", manifest_path.display()))?;
    Ok(())
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

fn install_app_binary(workflow: Workflow, target_dir: &Path) -> Result<(), String> {
    let layout = AicoreLayout::from_system_home();
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

    Ok(())
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
    use std::path::{Path, PathBuf};

    use crate::layers::Workflow;

    use super::{
        cargo_args_for_workflow, install_bin_dir_for, install_manifest_for, installed_binary_path,
        target_dir_for,
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
}
