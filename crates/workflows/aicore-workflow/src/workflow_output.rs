mod format;
mod header;
mod panels;
mod run;
mod steps;
#[cfg(test)]
mod tests;
mod warnings;

#[cfg(test)]
use aicore_terminal::WarningDiagnostic;
use aicore_terminal::{Block, Document, Status, TerminalConfig, render_document, safe_text};

use crate::cargo_runner::CommandReport;
use crate::shell_integration::ShellPathBootstrapResult;

pub use run::WorkflowOutput;

#[derive(Debug, Clone, PartialEq, Eq)]
struct WorkflowStepRecord {
    layer: String,
    step: String,
    command: String,
    status: Status,
    warning_count: usize,
    duration: std::time::Duration,
}

pub fn deny_warnings_error(warning_count: usize) -> Option<String> {
    if warning_count == 0 {
        None
    } else {
        Some(format!(
            "已启用 AICORE_WORKFLOW_DENY_WARNINGS=1。\n检测到 warning，因此 workflow 失败。Warnings {warning_count}"
        ))
    }
}

pub fn render_command_report(
    report: &CommandReport,
    verbose: bool,
    force_raw_output: bool,
    config: &TerminalConfig,
) -> String {
    if !verbose && !force_raw_output {
        return String::new();
    }

    let mut text = format!(
        "{}\nexit_code = {:?}\nduration_ms = {}\n",
        report.command,
        report.exit_code,
        report.duration.as_millis()
    );
    if !report.stdout.trim().is_empty() {
        text.push_str("\nstdout:\n");
        text.push_str(&report.stdout);
    }
    if !report.stderr.trim().is_empty() {
        text.push_str("\nstderr:\n");
        text.push_str(&report.stderr);
    }

    render_document(&Document::new(vec![Block::text(&text)]), config)
}

fn render_shell_path_bootstrap(
    result: &ShellPathBootstrapResult,
    config: &TerminalConfig,
) -> String {
    let rows = shell_path_bootstrap_rows(result);
    let body = if config.mode == aicore_terminal::TerminalMode::Rich {
        panels::render_colon_rows(&rows, config)
    } else {
        panels::render_key_rows(&rows)
    };
    if config.mode == aicore_terminal::TerminalMode::Json {
        render_document(
            &Document::new(vec![Block::panel("Shell PATH Bootstrap", &body)]),
            config,
        )
    } else {
        panels::render_panel("Shell PATH Bootstrap", &body, config)
    }
}

fn shell_path_bootstrap_rows(result: &ShellPathBootstrapResult) -> Vec<(&'static str, String)> {
    let mut rows = vec![
        ("status", result.status.label().to_string()),
        ("shell", safe_text(&result.shell)),
        (
            "rc file",
            result
                .rc_file
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "-".to_string()),
        ),
        ("bin path", result.bin_path.display().to_string()),
        ("action", safe_text(&result.action)),
    ];
    if let Some(reload) = &result.reload {
        rows.push(("reload", safe_text(reload)));
    }
    if let Some(rollback) = &result.rollback {
        rows.push(("rollback", safe_text(rollback)));
    }
    if let Some(message) = &result.message {
        rows.push(("message", safe_text(message)));
    }
    rows
}

#[cfg(test)]
fn render_run_started_for_tests(workflow_name: &str, config: &TerminalConfig) -> String {
    header::render_run_started(workflow_name, "/repo", "foundation + kernel", config)
}

#[cfg(test)]
fn render_finished_for_tests(
    workflow_name: &str,
    status: Status,
    step_count: usize,
    warning_count: usize,
    config: &TerminalConfig,
) -> String {
    header::render_finished(
        workflow_name,
        status,
        &sample_step_records(step_count),
        warning_count,
        std::time::Duration::from_millis(1420),
        config,
    )
}

#[cfg(test)]
fn render_warnings_for_tests(warnings: Vec<WarningDiagnostic>, config: &TerminalConfig) -> String {
    warnings::render_warnings(warnings, config)
}

#[cfg(test)]
fn render_workflow_steps_for_tests(config: &TerminalConfig) -> String {
    steps::render_workflow_steps(&sample_step_records(8), config)
}

#[cfg(test)]
fn render_command_report_for_tests(
    report: &CommandReport,
    verbose: bool,
    force_raw_output: bool,
    config: &TerminalConfig,
) -> String {
    render_command_report(report, verbose, force_raw_output, config)
}

#[cfg(test)]
fn sample_step_records(count: usize) -> Vec<WorkflowStepRecord> {
    let layers = [
        "foundation",
        "foundation",
        "foundation",
        "foundation",
        "kernel",
        "kernel",
        "kernel",
        "kernel",
    ];
    let steps = [
        "fmt", "test", "build", "install", "fmt", "test", "build", "install",
    ];
    (0..count)
        .map(|index| {
            let sample_index = index % layers.len();
            WorkflowStepRecord {
                layer: layers[sample_index].to_string(),
                step: steps[sample_index].to_string(),
                command: format!("cargo {}", steps[sample_index]),
                status: Status::Ok,
                warning_count: 0,
                duration: std::time::Duration::from_millis(80 + (index as u64 * 10)),
            }
        })
        .collect()
}
