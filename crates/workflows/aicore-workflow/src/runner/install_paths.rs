use std::ffi::OsString;
use std::path::{Path, PathBuf};

use aicore_terminal::WarningDiagnostic;

use crate::layers::Workflow;
use crate::shell_integration::has_managed_path_block;

pub(crate) fn install_manifest_for(target_dir: &Path) -> PathBuf {
    target_dir.join("install/install.toml")
}

pub(crate) fn install_bin_dir_for(home_root: &Path) -> PathBuf {
    home_root.join(".aicore/bin")
}

pub(crate) fn installed_binary_path(home_root: &Path, workflow: Workflow) -> PathBuf {
    install_bin_dir_for(home_root).join(binary_name_for(workflow))
}

pub(crate) fn built_binary_path(target_dir: &Path, workflow: Workflow) -> PathBuf {
    target_dir.join("debug").join(binary_name_for(workflow))
}

pub(crate) fn binary_name_for(workflow: Workflow) -> &'static str {
    match workflow {
        Workflow::Foundation => "aicore-foundation",
        Workflow::Kernel => "aicore-kernel",
        Workflow::AppAicore => "aicore",
        Workflow::AppCli => "aicore-cli",
        Workflow::AppTui => "aicore-tui",
        Workflow::AppWeb => "aicore-web",
        Workflow::Core => unreachable!("core workflow does not install a single binary"),
    }
}

const INSTALLED_COMMANDS: [&str; 4] = ["aicore", "aicore-cli", "aicore-tui", "aicore-web"];

pub(crate) fn install_visibility_warnings(
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
        let installed_paths = render_installed_paths(&installed, &install_dir);
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

fn render_installed_paths(installed: &[(&str, PathBuf)], install_dir: &Path) -> String {
    if installed.is_empty() {
        return format!("- {}", install_dir.display());
    }
    installed
        .iter()
        .map(|(_, path)| format!("- {}", path.display()))
        .collect::<Vec<_>>()
        .join("\n")
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
