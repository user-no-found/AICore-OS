use std::path::{Path, PathBuf};
use std::process::Command;

use crate::state::TuiLaunchContext;

const WARP_BIN_ENV: &str = "AICORE_TUI_WARP_BIN";
const SKIP_LAUNCH_ENV: &str = "AICORE_TUI_SKIP_WARP_LAUNCH";

pub fn run_warp_tui(context: &TuiLaunchContext) -> i32 {
    if std::env::var_os(SKIP_LAUNCH_ENV).is_some() {
        print_launch_ready(context);
        return 0;
    }

    if !graphical_session_available() {
        print_missing_graphical_session(context);
        return 3;
    }

    let Some(binary) = locate_warp_binary() else {
        print_missing_binary(context);
        return 2;
    };

    let mut command = Command::new(&binary);
    apply_context_env(&mut command, context);
    match command.status() {
        Ok(status) => status.code().unwrap_or(1),
        Err(error) => {
            eprintln!("启动 AICore TUI Warp fork 失败：{error}");
            eprintln!("路径：{}", binary.display());
            1
        }
    }
}

fn locate_warp_binary() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os(WARP_BIN_ENV).map(PathBuf::from) {
        return executable_candidate(path);
    }

    for path in default_candidates() {
        if let Some(path) = executable_candidate(path) {
            return Some(path);
        }
    }
    None
}

fn executable_candidate(path: PathBuf) -> Option<PathBuf> {
    if !path.is_file() {
        return None;
    }
    if let Ok(current_exe) = std::env::current_exe() {
        if same_path(&path, &current_exe) {
            return None;
        }
    }
    Some(path)
}

fn default_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    if let Some(repo_root) = manifest_dir.parent().and_then(Path::parent) {
        candidates.push(repo_root.join("apps/aicore-tui-warp/target/debug/aicore-tui-warp"));
        candidates.push(repo_root.join("apps/aicore-tui-warp/target/release/aicore-tui-warp"));
    }

    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(bin_dir) = current_exe.parent() {
            candidates.push(bin_dir.join("aicore-tui-warp"));
        }
    }

    candidates
}

fn same_path(left: &Path, right: &Path) -> bool {
    match (left.canonicalize(), right.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
    }
}

fn apply_context_env(command: &mut Command, context: &TuiLaunchContext) {
    command.env("AICORE_TUI_SOURCE", "warp_fork");
    command.env("AICORE_INSTANCE_ID", &context.instance_id);
    command.env("AICORE_INSTANCE_KIND", &context.instance_kind);
    command.env("AICORE_WORKSPACE_ROOT", &context.workspace_root);
    command.env("AICORE_INSTANCE_ROOT", &context.instance_root);
}

fn graphical_session_available() -> bool {
    ["WAYLAND_DISPLAY", "WAYLAND_SOCKET", "DISPLAY"]
        .iter()
        .any(|name| env_value_present(name))
}

fn env_value_present(name: &str) -> bool {
    std::env::var_os(name).is_some_and(|value| !value.is_empty())
}

fn print_launch_ready(context: &TuiLaunchContext) {
    println!("AICore TUI Warp fork 启动检查通过。");
    println!("实例：{}", context.instance_id);
    println!("类型：{}", context.instance_kind_label());
    println!("工作区：{}", context.workspace_root);
    println!("状态目录：{}", context.instance_root);
    println!("已完成实例绑定，测试模式未启动 Warp fork 进程。");
}

fn print_missing_binary(context: &TuiLaunchContext) {
    eprintln!("AICore TUI 已切换为 Warp fork 启动器，但未找到 fork 二进制。");
    eprintln!("实例：{}", context.instance_id);
    eprintln!("类型：{}", context.instance_kind_label());
    eprintln!("工作区：{}", context.workspace_root);
    eprintln!("状态目录：{}", context.instance_root);
    eprintln!("构建命令：");
    eprintln!("  cd apps/aicore-tui-warp");
    eprintln!("  cargo build -p aicore-tui-warp --bin aicore-tui-warp");
    eprintln!("也可以设置 {WARP_BIN_ENV}=<aicore-tui-warp 路径>。");
}

fn print_missing_graphical_session(context: &TuiLaunchContext) {
    eprintln!("AICore TUI Warp UI 无法启动：当前环境没有图形会话。");
    eprintln!("实例：{}", context.instance_id);
    eprintln!("类型：{}", context.instance_kind_label());
    eprintln!("工作区：{}", context.workspace_root);
    eprintln!("状态目录：{}", context.instance_root);
    eprintln!("需要设置 WAYLAND_DISPLAY、WAYLAND_SOCKET 或 DISPLAY 后再启动。");
    eprintln!(
        "如果你在 SSH / headless shell 中运行，请先进入图形桌面会话，或等待后续终端 fallback 接入。"
    );
}

#[cfg(test)]
mod tests {
    use super::default_candidates;

    #[test]
    fn default_candidates_point_to_warp_fork_binary() {
        let candidates = default_candidates();

        assert!(
            candidates
                .iter()
                .any(|path| path.ends_with("apps/aicore-tui-warp/target/debug/aicore-tui-warp"))
        );
    }
}
