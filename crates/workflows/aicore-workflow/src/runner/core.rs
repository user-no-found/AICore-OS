use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use aicore_terminal::Status;

use crate::cargo_runner::{CommandReport, run_cargo_capture, run_cargo_capture_with_env};
use crate::layers::Workflow;
use crate::runner::install::{InstallOutcome, install_layer, install_warp_tui_binary};
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
    if workflow == Workflow::AppTui {
        build_warp_tui(output, repo_root)?;
    }
    output.step_started(&format!("{} / install", workflow.id()));
    let install_started_at = Instant::now();
    let InstallOutcome {
        warnings,
        shell_bootstrap,
    } = install_layer(workflow, &target_dir)?;
    if let Some(shell_bootstrap) = &shell_bootstrap {
        output.record_shell_path_bootstrap(shell_bootstrap);
    }
    let install_warning_count = warnings.len();
    for warning in warnings {
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
    if workflow == Workflow::AppTui {
        install_warp_tui(output, repo_root)?;
    }
    Ok(())
}

fn build_warp_tui(output: &mut WorkflowOutput, repo_root: &Path) -> Result<(), String> {
    if !has_protoc() {
        return Err("未找到 protoc，无法编译 Warp fork TUI。\n请先安装 protobuf compiler，例如 Debian/Ubuntu: sudo apt-get install protobuf-compiler。".to_string());
    }

    let warp_root = repo_root.join("apps/aicore-tui-warp");
    let warp_target = target_dir_for(repo_root, Workflow::AppTui).join("warp-fork");
    cleanup_target_if_needed(&warp_target, output)?;
    run_cargo_step_with_env(
        output,
        &warp_root,
        Some(&warp_target),
        "app-tui",
        "warp-build",
        "cargo build -p warp --bin aicore-tui-warp",
        &["build", "-p", "warp", "--bin", "aicore-tui-warp"],
        &[("RUSTUP_TOOLCHAIN", "1.95.0")],
    )
}

fn install_warp_tui(output: &mut WorkflowOutput, repo_root: &Path) -> Result<(), String> {
    output.step_started("app-tui / warp-install");
    let started_at = Instant::now();
    let warp_target = target_dir_for(repo_root, Workflow::AppTui).join("warp-fork");
    install_warp_tui_binary(&warp_target)?;
    output.record_local_step(
        "app-tui",
        "warp-install",
        "install aicore-tui-warp",
        Status::Ok,
        started_at.elapsed(),
    );
    Ok(())
}

fn has_protoc() -> bool {
    std::process::Command::new("protoc")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
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

pub(crate) fn cargo_args_for_workflow(workflow: Workflow, subcommand: &str) -> Vec<String> {
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
    run_cargo_step_with_env(
        output,
        repo_root,
        target_dir,
        layer,
        step,
        command,
        args,
        &[],
    )
}

fn run_cargo_step_with_env(
    output: &mut WorkflowOutput,
    repo_root: &Path,
    target_dir: Option<&Path>,
    layer: &str,
    step: &str,
    command: &str,
    args: &[&str],
    envs: &[(&str, &str)],
) -> Result<(), String> {
    output.step_started(&format!("{layer} / {step}"));
    let report = if envs.is_empty() {
        run_cargo_capture(repo_root, target_dir, args)?
    } else {
        run_cargo_capture_with_env(repo_root, target_dir, args, envs)?
    };
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

pub(crate) fn target_dir_for(repo_root: &Path, workflow: Workflow) -> PathBuf {
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
