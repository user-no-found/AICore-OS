use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

use aicore_terminal::WarningDiagnostic;

use crate::cargo_diagnostics::parse_warnings;

#[derive(Debug, Clone)]
pub struct CommandReport {
    pub command: String,
    pub exit_code: Option<i32>,
    pub duration: Duration,
    pub stdout: String,
    pub stderr: String,
    pub warnings: Vec<WarningDiagnostic>,
}

impl CommandReport {
    #[cfg(test)]
    pub fn for_tests(
        command: &str,
        exit_code: Option<i32>,
        stdout: &str,
        stderr: &str,
        duration: Duration,
    ) -> Self {
        Self {
            command: command.to_string(),
            exit_code,
            duration,
            stdout: stdout.to_string(),
            stderr: stderr.to_string(),
            warnings: parse_warnings(command, stdout, stderr),
        }
    }

    pub fn succeeded(&self) -> bool {
        self.exit_code == Some(0)
    }

    pub fn warning_count(&self) -> usize {
        self.warnings.len()
    }
}

pub fn run_cargo_capture(
    repo_root: &Path,
    target_dir: Option<&Path>,
    args: &[&str],
) -> Result<CommandReport, String> {
    let mut command = Command::new("cargo");
    command.args(args).current_dir(repo_root);
    if let Some(target_dir) = target_dir {
        command.env("CARGO_TARGET_DIR", target_dir);
    }

    let command_label = format!("cargo {}", args.join(" "));
    let started_at = Instant::now();
    let output = command
        .output()
        .map_err(|error| format!("执行 {command_label} 失败: {error}"))?;
    let duration = started_at.elapsed();
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let warnings = parse_warnings(&command_label, &stdout, &stderr);

    Ok(CommandReport {
        command: command_label,
        exit_code: output.status.code(),
        duration,
        stdout,
        stderr,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn command_report_exposes_warning_count_and_success() {
        let report = CommandReport::for_tests(
            "cargo test",
            Some(0),
            "",
            "warning: unused variable\n",
            Duration::from_millis(10),
        );

        assert!(report.succeeded());
        assert_eq!(report.warning_count(), 1);
    }
}
