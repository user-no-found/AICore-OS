use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::layers::Workflow;

const TARGET_LIMIT_BYTES: u64 = 30 * 1024 * 1024 * 1024;

pub fn run(workflow: Workflow) -> Result<(), String> {
    let repo_root = find_repo_root()?;
    match workflow {
        Workflow::Core => {
            println!("开始执行{} workflow。", workflow.label_zh());
            run_single(&repo_root, Workflow::Foundation)?;
            run_single(&repo_root, Workflow::Kernel)?;
            println!("{} workflow 执行完成。", workflow.label_zh());
            Ok(())
        }
        Workflow::Foundation | Workflow::Kernel => run_single(&repo_root, workflow),
    }
}

fn run_single(repo_root: &Path, workflow: Workflow) -> Result<(), String> {
    println!("开始执行{} workflow。", workflow.label_zh());
    let target_dir = target_dir_for(repo_root, workflow);
    cleanup_target_if_needed(&target_dir)?;
    run_cargo(repo_root, None, &["fmt", "--check"])?;
    run_cargo_for_workflow(repo_root, workflow, &target_dir, "test")?;
    run_cargo_for_workflow(repo_root, workflow, &target_dir, "build")?;
    println!("{} workflow 执行完成。", workflow.label_zh());
    Ok(())
}

fn run_cargo_for_workflow(
    repo_root: &Path,
    workflow: Workflow,
    target_dir: &Path,
    subcommand: &str,
) -> Result<(), String> {
    let mut args = vec![subcommand];
    for crate_name in workflow.crates() {
        args.push("-p");
        args.push(crate_name);
    }
    args.push("--offline");
    run_cargo(repo_root, Some(target_dir), &args)
}

fn run_cargo(repo_root: &Path, target_dir: Option<&Path>, args: &[&str]) -> Result<(), String> {
    let mut command = Command::new("cargo");
    command.args(args).current_dir(repo_root);
    if let Some(target_dir) = target_dir {
        command.env("CARGO_TARGET_DIR", target_dir);
    }
    let status = command
        .status()
        .map_err(|error| format!("执行 cargo {:?} 失败: {error}", args))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("cargo {:?} 执行失败。", args))
    }
}

fn cleanup_target_if_needed(target_dir: &Path) -> Result<(), String> {
    if !target_dir.exists() {
        return Ok(());
    }

    let size = dir_size(target_dir)?;
    if size > TARGET_LIMIT_BYTES {
        println!("{} 超过 30GiB，正在清理后重新编译。", target_dir.display());
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

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::layers::Workflow;

    use super::target_dir_for;

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
}
