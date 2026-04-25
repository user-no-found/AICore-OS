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
        Workflow::Foundation | Workflow::Kernel | Workflow::AppAicore | Workflow::AppCli => {
            run_single(&repo_root, workflow)
        }
    }
}

fn run_single(repo_root: &Path, workflow: Workflow) -> Result<(), String> {
    println!("开始执行{} workflow。", workflow.label_zh());
    let target_dir = target_dir_for(repo_root, workflow);
    cleanup_target_if_needed(&target_dir)?;
    run_cargo(repo_root, None, &["fmt", "--check"])?;
    run_cargo_for_workflow(repo_root, workflow, &target_dir, "test")?;
    run_cargo_for_workflow(repo_root, workflow, &target_dir, "build")?;
    install_layer(workflow, &target_dir)?;
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
        Workflow::AppAicore => repo_root.join("target/apps/aicore"),
        Workflow::AppCli => repo_root.join("target/apps/aicore-cli"),
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
        },
        target_dir_escaped,
        packages
    )
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use crate::layers::Workflow;

    use super::{install_manifest_for, target_dir_for};

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
    fn foundation_install_manifest_path_is_under_install_dir() {
        let target_dir = PathBuf::from("/repo/target/layers/foundation");
        assert_eq!(
            install_manifest_for(&target_dir),
            PathBuf::from("/repo/target/layers/foundation/install/install.toml")
        );
    }
}
