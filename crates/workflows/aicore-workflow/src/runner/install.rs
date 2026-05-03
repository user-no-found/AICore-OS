use std::ffi::OsString;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use aicore_foundation::AicoreLayout;
use aicore_terminal::WarningDiagnostic;

use crate::layers::Workflow;
use crate::runtime_install::{install_app_manifest, install_global_runtime_metadata};
use crate::shell_integration::{
    ShellPathBootstrapEnv, ShellPathBootstrapResult, ShellPathBootstrapStatus,
    bootstrap_shell_path, has_managed_path_block,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InstallOutcome {
    pub(crate) warnings: Vec<WarningDiagnostic>,
    pub(crate) shell_bootstrap: Option<ShellPathBootstrapResult>,
}

pub(crate) fn install_layer(
    workflow: Workflow,
    target_dir: &Path,
) -> Result<InstallOutcome, String> {
    install_layer_with_shell_env(workflow, target_dir, &ShellPathBootstrapEnv::current())
}

pub(crate) fn install_layer_with_shell_env(
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
        install_runtime_binary(workflow, target_dir, &layout)?;
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

pub(crate) fn install_manifest_for(target_dir: &Path) -> PathBuf {
    target_dir.join("install/install.toml")
}

pub(crate) fn install_bin_dir_for(home_root: &Path) -> PathBuf {
    home_root.join(".aicore/bin")
}

pub(crate) fn installed_binary_path(home_root: &Path, workflow: Workflow) -> PathBuf {
    install_bin_dir_for(home_root).join(binary_name_for(workflow))
}

fn built_binary_path(target_dir: &Path, workflow: Workflow) -> PathBuf {
    target_dir.join("debug").join(binary_name_for(workflow))
}

fn binary_name_for(workflow: Workflow) -> &'static str {
    match workflow {
        Workflow::Foundation => "aicore-foundation",
        Workflow::Kernel => "aicore-kernel",
        Workflow::AppAicore => "aicore",
        Workflow::AppCli => "aicore-cli",
        Workflow::AppTui => "aicore-tui",
        Workflow::Core => unreachable!("core workflow does not install a single binary"),
    }
}

const INSTALLED_COMMANDS: [&str; 3] = ["aicore", "aicore-cli", "aicore-tui"];
const WARP_TUI_BINARY: &str = "aicore-tui-warp";

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

pub(crate) fn install_warp_tui_binary(target_dir: &Path) -> Result<(), String> {
    let layout = AicoreLayout::new(
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .ok_or_else(|| "HOME 不可用，无法安装 AICore TUI Warp fork。".to_string())?,
    );
    install_warp_tui_binary_for_layout(target_dir, &layout)
}

pub(crate) fn install_warp_tui_binary_for_layout(
    target_dir: &Path,
    layout: &AicoreLayout,
) -> Result<(), String> {
    let install_dir = install_bin_dir_for(&layout.home_root);
    fs::create_dir_all(&install_dir)
        .map_err(|error| format!("创建应用安装目录 {} 失败: {error}", install_dir.display()))?;

    let source_path = target_dir.join("debug").join(WARP_TUI_BINARY);
    if !source_path.exists() {
        return Err(format!(
            "未找到待安装 Warp fork 二进制: {}",
            source_path.display()
        ));
    }

    let target_path = install_dir.join(WARP_TUI_BINARY);
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

fn install_runtime_binary(
    workflow: Workflow,
    target_dir: &Path,
    layout: &AicoreLayout,
) -> Result<(), String> {
    let install_dir = install_bin_dir_for(&layout.home_root);
    fs::create_dir_all(&install_dir).map_err(|error| {
        format!(
            "创建 runtime 安装目录 {} 失败: {error}",
            install_dir.display()
        )
    })?;

    let source_path = built_binary_path(target_dir, workflow);
    if !source_path.exists() {
        return Err(format!(
            "未找到待安装 runtime binary: {}",
            source_path.display()
        ));
    }

    let target_path = installed_binary_path(&layout.home_root, workflow);
    fs::copy(&source_path, &target_path).map_err(|error| {
        format!(
            "复制 runtime binary {} -> {} 失败: {error}",
            source_path.display(),
            target_path.display()
        )
    })?;

    #[cfg(unix)]
    {
        let mut permissions = fs::metadata(&target_path)
            .map_err(|error| format!("读取 runtime binary 权限失败: {error}"))?
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&target_path, permissions)
            .map_err(|error| format!("设置 runtime binary 可执行权限失败: {error}"))?;
    }

    Ok(())
}

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
