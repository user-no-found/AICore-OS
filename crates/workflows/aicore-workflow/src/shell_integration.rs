use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

pub const MANAGED_BLOCK_START: &str = "# >>> AICore OS >>>";
pub const MANAGED_BLOCK_END: &str = "# <<< AICore OS <<<";
pub const MANAGED_PATH_LINE: &str = "export PATH=\"$HOME/.aicore/bin:$PATH\"";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellKind {
    Bash,
    Zsh,
    Unsupported(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellPathBootstrapStatus {
    AlreadyConfigured,
    Appended,
    Updated,
    SkippedCi,
    UnsupportedShell,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellPathBootstrapEnv {
    pub home: Option<PathBuf>,
    pub shell: Option<String>,
    pub path: String,
    pub ci: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellPathBootstrapResult {
    pub status: ShellPathBootstrapStatus,
    pub shell: String,
    pub rc_file: Option<PathBuf>,
    pub bin_path: PathBuf,
    pub action: String,
    pub reload: Option<String>,
    pub rollback: Option<String>,
    pub message: Option<String>,
}

pub fn bootstrap_shell_path(env: &ShellPathBootstrapEnv) -> ShellPathBootstrapResult {
    let home = match &env.home {
        Some(home) => home,
        None => {
            return failed_result(
                "unknown",
                None,
                PathBuf::from("/home/unknown/.aicore/bin"),
                "HOME 不可用，无法写入 shell PATH 配置。",
            );
        }
    };
    let bin_path = home.join(".aicore/bin");
    let shell_kind = detect_shell_kind(env.shell.as_deref());
    let shell_label = shell_kind.label().to_string();

    if env.ci {
        return ShellPathBootstrapResult {
            status: ShellPathBootstrapStatus::SkippedCi,
            shell: shell_label,
            rc_file: None,
            bin_path,
            action: "skipped ci".to_string(),
            reload: None,
            rollback: None,
            message: Some("检测到 CI 环境，跳过 shell PATH 写入。".to_string()),
        };
    }

    let ShellKind::Bash = shell_kind else {
        return ShellPathBootstrapResult {
            status: ShellPathBootstrapStatus::UnsupportedShell,
            shell: shell_label,
            rc_file: None,
            bin_path,
            action: "unsupported shell".to_string(),
            reload: None,
            rollback: None,
            message: Some("暂未自动写入该 shell，请手动配置 PATH。".to_string()),
        };
    };

    let rc_file = home.join(".bashrc");
    let current = match fs::read_to_string(&rc_file) {
        Ok(content) => content,
        Err(error) if error.kind() == ErrorKind::NotFound => String::new(),
        Err(error) => {
            return failed_result(
                &shell_label,
                Some(rc_file),
                bin_path,
                &format!("读取 shell rc 失败: {error}"),
            );
        }
    };

    let block = managed_block();
    let (status, updated) = if current.contains(&block) {
        (ShellPathBootstrapStatus::AlreadyConfigured, current)
    } else if let Some(updated) = replace_managed_block(&current, &block) {
        (ShellPathBootstrapStatus::Updated, updated)
    } else {
        (
            ShellPathBootstrapStatus::Appended,
            append_managed_block(&current, &block),
        )
    };

    if status != ShellPathBootstrapStatus::AlreadyConfigured {
        if let Some(parent) = rc_file.parent() {
            if let Err(error) = fs::create_dir_all(parent) {
                return failed_result(
                    &shell_label,
                    Some(rc_file),
                    bin_path,
                    &format!("创建 shell rc 目录失败: {error}"),
                );
            }
        }
        if let Err(error) = fs::write(&rc_file, updated) {
            return failed_result(
                &shell_label,
                Some(rc_file),
                bin_path,
                &format!("写入 shell rc 失败: {error}"),
            );
        }
    }

    ShellPathBootstrapResult {
        action: match status {
            ShellPathBootstrapStatus::AlreadyConfigured => "no change".to_string(),
            ShellPathBootstrapStatus::Appended => "appended managed block".to_string(),
            ShellPathBootstrapStatus::Updated => "updated managed block".to_string(),
            ShellPathBootstrapStatus::SkippedCi
            | ShellPathBootstrapStatus::UnsupportedShell
            | ShellPathBootstrapStatus::Failed => unreachable!("handled before bash write result"),
        },
        status,
        shell: shell_label,
        rc_file: Some(rc_file),
        bin_path,
        reload: Some("source ~/.bashrc && hash -r".to_string()),
        rollback: Some("remove managed block".to_string()),
        message: None,
    }
}

pub fn has_managed_path_block(home_root: &Path) -> bool {
    fs::read_to_string(home_root.join(".bashrc"))
        .map(|content| content.contains(MANAGED_BLOCK_START) && content.contains(MANAGED_PATH_LINE))
        .unwrap_or(false)
}

impl ShellPathBootstrapEnv {
    pub fn current() -> Self {
        Self {
            home: std::env::var_os("HOME").map(PathBuf::from),
            shell: std::env::var("SHELL").ok(),
            path: std::env::var("PATH").unwrap_or_default(),
            ci: std::env::var("CI").ok().as_deref() == Some("1"),
        }
    }
}

impl ShellPathBootstrapStatus {
    pub fn label(&self) -> &'static str {
        match self {
            Self::AlreadyConfigured => "already configured",
            Self::Appended => "updated",
            Self::Updated => "updated",
            Self::SkippedCi => "skipped",
            Self::UnsupportedShell => "skipped",
            Self::Failed => "failed",
        }
    }
}

impl ShellKind {
    fn label(&self) -> &str {
        match self {
            Self::Bash => "bash",
            Self::Zsh => "zsh",
            Self::Unsupported(shell) => shell,
        }
    }
}

fn detect_shell_kind(shell: Option<&str>) -> ShellKind {
    match shell.unwrap_or_default() {
        value if value.contains("bash") => ShellKind::Bash,
        value if value.contains("zsh") => ShellKind::Zsh,
        "" => ShellKind::Unsupported("unknown".to_string()),
        value => ShellKind::Unsupported(value.to_string()),
    }
}

fn managed_block() -> String {
    format!("{MANAGED_BLOCK_START}\n{MANAGED_PATH_LINE}\n{MANAGED_BLOCK_END}\n")
}

fn append_managed_block(current: &str, block: &str) -> String {
    if current.is_empty() {
        return block.to_string();
    }
    let mut updated = current.to_string();
    if !updated.ends_with('\n') {
        updated.push('\n');
    }
    updated.push_str(block);
    updated
}

fn replace_managed_block(current: &str, block: &str) -> Option<String> {
    let start = current.find(MANAGED_BLOCK_START)?;
    let end_start = current[start..].find(MANAGED_BLOCK_END)? + start;
    let end = end_start + MANAGED_BLOCK_END.len();
    let mut updated = String::new();
    updated.push_str(&current[..start]);
    updated.push_str(block);
    if end < current.len() {
        if !updated.ends_with('\n') {
            updated.push('\n');
        }
        let remainder = &current[end..];
        updated.push_str(remainder.trim_start_matches('\n'));
    }
    Some(updated)
}

fn failed_result(
    shell: &str,
    rc_file: Option<PathBuf>,
    bin_path: PathBuf,
    message: &str,
) -> ShellPathBootstrapResult {
    ShellPathBootstrapResult {
        status: ShellPathBootstrapStatus::Failed,
        shell: shell.to_string(),
        rc_file,
        bin_path,
        action: "failed".to_string(),
        reload: None,
        rollback: Some("remove managed block".to_string()),
        message: Some(message.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    #[test]
    fn foundation_shell_bootstrap_appends_managed_block() {
        let home = temp_home("append");
        fs::write(home.join(".bashrc"), "# user config\n").expect("write bashrc");
        let result = bootstrap_shell_path(&bash_env(&home));
        let bashrc = fs::read_to_string(home.join(".bashrc")).expect("read bashrc");

        assert_eq!(result.status, ShellPathBootstrapStatus::Appended);
        assert!(bashrc.contains(MANAGED_BLOCK_START));
        assert!(bashrc.contains(MANAGED_PATH_LINE));
        assert!(bashrc.contains(MANAGED_BLOCK_END));
    }

    #[test]
    fn foundation_shell_bootstrap_updates_existing_managed_block() {
        let home = temp_home("update");
        fs::write(
            home.join(".bashrc"),
            format!("{MANAGED_BLOCK_START}\nexport PATH=\"$PATH:$HOME/.aicore/bin\"\n{MANAGED_BLOCK_END}\n"),
        )
        .expect("write bashrc");
        let result = bootstrap_shell_path(&bash_env(&home));
        let bashrc = fs::read_to_string(home.join(".bashrc")).expect("read bashrc");

        assert_eq!(result.status, ShellPathBootstrapStatus::Updated);
        assert!(bashrc.contains(MANAGED_PATH_LINE));
        assert!(!bashrc.contains("export PATH=\"$PATH:$HOME/.aicore/bin\""));
    }

    #[test]
    fn foundation_shell_bootstrap_is_idempotent() {
        let home = temp_home("idempotent");
        let first = bootstrap_shell_path(&bash_env(&home));
        let second = bootstrap_shell_path(&bash_env(&home));
        let bashrc = fs::read_to_string(home.join(".bashrc")).expect("read bashrc");

        assert_eq!(first.status, ShellPathBootstrapStatus::Appended);
        assert_eq!(second.status, ShellPathBootstrapStatus::AlreadyConfigured);
        assert_eq!(bashrc.matches(MANAGED_BLOCK_START).count(), 1);
        assert_eq!(bashrc.matches(MANAGED_PATH_LINE).count(), 1);
    }

    #[test]
    fn foundation_shell_bootstrap_detects_already_configured_path() {
        let home = temp_home("already");
        fs::write(
            home.join(".bashrc"),
            format!("{MANAGED_BLOCK_START}\n{MANAGED_PATH_LINE}\n{MANAGED_BLOCK_END}\n"),
        )
        .expect("write bashrc");
        let result = bootstrap_shell_path(&bash_env(&home));

        assert_eq!(result.status, ShellPathBootstrapStatus::AlreadyConfigured);
    }

    #[test]
    fn foundation_shell_bootstrap_creates_missing_bashrc() {
        let home = temp_home("missing");
        let result = bootstrap_shell_path(&bash_env(&home));

        assert_eq!(result.status, ShellPathBootstrapStatus::Appended);
        assert!(home.join(".bashrc").exists());
    }

    #[test]
    fn foundation_shell_bootstrap_skips_in_ci() {
        let home = temp_home("ci");
        let mut env = bash_env(&home);
        env.ci = true;
        let result = bootstrap_shell_path(&env);

        assert_eq!(result.status, ShellPathBootstrapStatus::SkippedCi);
        assert!(!home.join(".bashrc").exists());
    }

    #[test]
    fn foundation_shell_bootstrap_reports_reload_command() {
        let home = temp_home("reload");
        let result = bootstrap_shell_path(&bash_env(&home));

        assert_eq!(
            result.reload.as_deref(),
            Some("source ~/.bashrc && hash -r")
        );
    }

    #[test]
    fn foundation_shell_bootstrap_reports_rollback_instruction() {
        let home = temp_home("rollback");
        let result = bootstrap_shell_path(&bash_env(&home));

        assert_eq!(result.rollback.as_deref(), Some("remove managed block"));
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
            "aicore-shell-bootstrap-{name}-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("create temp home");
        path
    }
}
