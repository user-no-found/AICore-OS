use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::layers::Workflow;

const TARGET_LIMIT_BYTES: u64 = 30 * 1024 * 1024 * 1024;

pub fn run(workflow: Workflow) -> Result<(), String> {
    let repo_root = find_repo_root()?;
    println!("开始执行{} workflow。", workflow.label_zh());

    cleanup_target_if_needed(&repo_root)?;
    run_cargo(&repo_root, &["fmt", "--check"])?;
    run_cargo_for_workflow(&repo_root, workflow, "test")?;
    run_cargo_for_workflow(&repo_root, workflow, "build")?;

    println!("{} workflow 执行完成。", workflow.label_zh());
    Ok(())
}

fn run_cargo_for_workflow(
    repo_root: &Path,
    workflow: Workflow,
    subcommand: &str,
) -> Result<(), String> {
    let mut args = vec![subcommand];
    for crate_name in workflow.crates() {
        args.push("-p");
        args.push(crate_name);
    }
    args.push("--offline");
    run_cargo(repo_root, &args)
}

fn run_cargo(repo_root: &Path, args: &[&str]) -> Result<(), String> {
    let status = Command::new("cargo")
        .args(args)
        .current_dir(repo_root)
        .status()
        .map_err(|error| format!("执行 cargo {:?} 失败: {error}", args))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("cargo {:?} 执行失败。", args))
    }
}

fn cleanup_target_if_needed(repo_root: &Path) -> Result<(), String> {
    let target_dir = repo_root.join("target");
    if !target_dir.exists() {
        return Ok(());
    }

    let size = dir_size(&target_dir)?;
    if size > TARGET_LIMIT_BYTES {
        println!("target/ 超过 30GiB，正在清理后重新编译。");
        fs::remove_dir_all(&target_dir).map_err(|error| format!("删除 target/ 失败: {error}"))?;
    }

    Ok(())
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
