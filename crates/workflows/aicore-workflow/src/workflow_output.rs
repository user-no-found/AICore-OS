use aicore_terminal::{
    Block, Document, LogoMode, RunSummary, Status, StepSummary, TerminalConfig, TerminalMode,
    WarningDiagnostic, render_document,
};

use crate::cargo_runner::CommandReport;

pub struct WorkflowOutput {
    config: TerminalConfig,
    workflow_name: String,
    step_count: usize,
    warnings: Vec<WarningDiagnostic>,
}

impl WorkflowOutput {
    pub fn new(workflow_name: &str, config: TerminalConfig) -> Self {
        Self {
            config,
            workflow_name: workflow_name.to_string(),
            step_count: 0,
            warnings: Vec::new(),
        }
    }

    pub fn from_current(workflow_name: &str) -> Self {
        Self::new(workflow_name, TerminalConfig::current())
    }

    pub fn warning_count(&self) -> usize {
        self.warnings.len()
    }

    pub fn start(&self) {
        print!("{}", render_run_started(&self.workflow_name, &self.config));
    }

    pub fn step_started(&self, step: &str) {
        print!(
            "{}",
            render_document(
                &Document::new(vec![Block::step_started(step)]),
                &self.config
            )
        );
    }

    pub fn record_command_report(
        &mut self,
        step: &str,
        report: &CommandReport,
        force_raw_output: bool,
    ) {
        self.step_count += 1;
        self.warnings.extend(report.warnings.clone());
        for warning in &report.warnings {
            print!(
                "{}",
                render_document(
                    &Document::new(vec![Block::warning(warning.clone())]),
                    &self.config
                )
            );
        }
        print!(
            "{}",
            render_command_report(report, self.config.verbose, force_raw_output, &self.config)
        );

        let status = if report.succeeded() {
            if report.warning_count() > 0 {
                Status::Warn
            } else {
                Status::Ok
            }
        } else {
            Status::Failed
        };
        let summary = StepSummary::new(step, status, report.warning_count());
        print!(
            "{}",
            render_document(
                &Document::new(vec![Block::step_finished(summary)]),
                &self.config
            )
        );
    }

    pub fn record_local_step(&mut self, step: &str, status: Status) {
        self.step_count += 1;
        let summary = StepSummary::new(step, status, 0);
        print!(
            "{}",
            render_document(
                &Document::new(vec![Block::step_finished(summary)]),
                &self.config
            )
        );
    }

    pub fn message(&self, message: &str) {
        print!(
            "{}",
            render_document(&Document::new(vec![Block::text(message)]), &self.config)
        );
    }

    pub fn finish(&self, status: Status) -> Result<(), String> {
        print!("{}", render_warnings(self.warnings.clone(), &self.config));
        print!(
            "{}",
            render_finished(
                &self.workflow_name,
                status,
                self.step_count,
                self.warning_count(),
                &self.config,
            )
        );

        if self.config.deny_warnings {
            if let Some(error) = deny_warnings_error(self.warning_count()) {
                return Err(error);
            }
        }
        Ok(())
    }
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

fn render_run_started(workflow_name: &str, config: &TerminalConfig) -> String {
    let mut blocks = Vec::new();
    if config.mode != TerminalMode::Json && config.logo != LogoMode::Off {
        blocks.push(Block::logo());
    }
    blocks.push(Block::run_started(workflow_name));
    render_document(&Document::new(blocks), config)
}

fn render_finished(
    workflow_name: &str,
    status: Status,
    step_count: usize,
    warning_count: usize,
    config: &TerminalConfig,
) -> String {
    render_document(
        &Document::new(vec![Block::run_finished(RunSummary::new(
            workflow_name,
            status,
            step_count,
            warning_count,
        ))]),
        config,
    )
}

fn render_warnings(warnings: Vec<WarningDiagnostic>, config: &TerminalConfig) -> String {
    render_document(
        &Document::new(vec![Block::warning_summary(warnings, 20)]),
        config,
    )
}

#[cfg(test)]
fn render_run_started_for_tests(workflow_name: &str, config: &TerminalConfig) -> String {
    render_run_started(workflow_name, config)
}

#[cfg(test)]
fn render_finished_for_tests(
    workflow_name: &str,
    status: Status,
    step_count: usize,
    warning_count: usize,
    config: &TerminalConfig,
) -> String {
    render_finished(workflow_name, status, step_count, warning_count, config)
}

#[cfg(test)]
fn render_warnings_for_tests(warnings: Vec<WarningDiagnostic>, config: &TerminalConfig) -> String {
    render_warnings(warnings, config)
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
mod tests {
    use aicore_terminal::{Status, TerminalConfig, WarningDiagnostic};

    use crate::cargo_runner::CommandReport;

    use super::*;

    #[test]
    fn workflow_report_includes_logo_header_in_rich_mode() {
        let output = render_run_started_for_tests("kernel", &TerminalConfig::rich_for_tests());

        assert!(output.contains("AICore OS"));
        assert!(output.contains("kernel workflow 开始"));
    }

    #[test]
    fn workflow_report_includes_step_cards_and_final_summary() {
        let output = render_finished_for_tests(
            "kernel",
            Status::Ok,
            2,
            0,
            &TerminalConfig::plain_for_tests(),
        );

        assert!(output.contains("Summary kernel"));
        assert!(output.contains("Steps 2"));
        assert!(output.contains("Warnings 0 scanned this run"));
    }

    #[test]
    fn workflow_report_reports_warning_count() {
        let warning = WarningDiagnostic::new("cargo test", "unused variable");
        let output = render_warnings_for_tests(vec![warning], &TerminalConfig::plain_for_tests());

        assert!(output.contains("Warnings 1"));
        assert!(output.contains("unused variable"));
    }

    #[test]
    fn workflow_deny_warnings_fails_when_warning_count_is_positive() {
        let error = deny_warnings_error(1).expect("warning should fail strict mode");

        assert!(error.contains("已启用 AICORE_WORKFLOW_DENY_WARNINGS=1"));
        assert!(error.contains("检测到 warning，因此 workflow 失败"));
    }

    #[test]
    fn workflow_plain_mode_has_no_ansi_or_unicode_border() {
        let output = render_run_started_for_tests("kernel", &TerminalConfig::plain_for_tests());

        assert!(!output.contains("\u{1b}"));
        assert!(!output.contains('╭'));
    }

    #[test]
    fn workflow_json_mode_emits_valid_json_lines() {
        let output = render_run_started_for_tests("kernel", &TerminalConfig::json_for_tests());

        for line in output.lines() {
            let value: serde_json::Value = serde_json::from_str(line).expect("json line");
            assert_eq!(value["schema"], "aicore.terminal.v1");
        }
    }

    #[test]
    fn workflow_verbose_mode_keeps_raw_output() {
        let report = CommandReport::for_tests(
            "cargo test",
            Some(0),
            "stdout text",
            "stderr text",
            std::time::Duration::from_millis(1),
        );
        let output = render_command_report_for_tests(
            &report,
            true,
            false,
            &TerminalConfig::plain_for_tests(),
        );

        assert!(output.contains("stdout text"));
        assert!(output.contains("stderr text"));
    }
}
