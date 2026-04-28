use std::path::Path;
use std::time::{Duration, Instant};

use aicore_terminal::{
    Block, Document, Status, StepSummary, TerminalConfig, TerminalMode, WarningDiagnostic,
    render_document,
};

use crate::cargo_runner::CommandReport;
use crate::shell_integration::ShellPathBootstrapResult;

use super::header::{render_finished, render_run_started};
use super::steps::render_workflow_steps;
use super::warnings::{render_warnings, warning_for_json};

pub struct WorkflowOutput {
    config: TerminalConfig,
    workflow_id: String,
    repo_root: String,
    target: String,
    started_at: Instant,
    warnings: Vec<WarningDiagnostic>,
    steps: Vec<super::WorkflowStepRecord>,
}

impl WorkflowOutput {
    pub fn new(workflow_id: &str, repo_root: &Path, target: &str, config: TerminalConfig) -> Self {
        Self {
            config,
            workflow_id: workflow_id.to_string(),
            repo_root: repo_root.display().to_string(),
            target: target.to_string(),
            started_at: Instant::now(),
            warnings: Vec::new(),
            steps: Vec::new(),
        }
    }

    pub fn from_current(workflow_id: &str, repo_root: &Path, target: &str) -> Self {
        Self::new(workflow_id, repo_root, target, TerminalConfig::current())
    }

    pub fn warning_count(&self) -> usize {
        self.warnings.len()
    }

    pub fn start(&self) {
        print!(
            "{}",
            render_run_started(
                &self.workflow_id,
                &self.repo_root,
                &self.target,
                &self.config
            )
        );
    }

    pub fn step_started(&self, step: &str) {
        if self.config.mode == TerminalMode::Json {
            print!(
                "{}",
                render_document(
                    &Document::new(vec![Block::step_started(step)]),
                    &self.config
                )
            );
        }
    }

    pub fn record_command_report(
        &mut self,
        layer: &str,
        step: &str,
        command: &str,
        report: &CommandReport,
        force_raw_output: bool,
    ) {
        self.warnings.extend(report.warnings.clone());
        if self.config.mode == TerminalMode::Json {
            for warning in &report.warnings {
                print!(
                    "{}",
                    render_document(
                        &Document::new(vec![Block::warning(warning.clone())]),
                        &self.config
                    )
                );
            }
        }
        print!(
            "{}",
            super::render_command_report(
                report,
                self.config.verbose,
                force_raw_output,
                &self.config
            )
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
        self.steps.push(super::WorkflowStepRecord {
            layer: layer.to_string(),
            step: step.to_string(),
            command: command.to_string(),
            status,
            warning_count: report.warning_count(),
            duration: report.duration,
        });

        if self.config.mode == TerminalMode::Json {
            let summary = StepSummary::new(step, status, report.warning_count());
            print!(
                "{}",
                render_document(
                    &Document::new(vec![Block::step_finished(summary)]),
                    &self.config
                )
            );
        }
    }

    pub fn record_local_step(
        &mut self,
        layer: &str,
        step: &str,
        command: &str,
        status: Status,
        duration: Duration,
    ) {
        self.record_local_step_with_warning_count(layer, step, command, status, duration, 0);
    }

    pub fn record_local_step_with_warning_count(
        &mut self,
        layer: &str,
        step: &str,
        command: &str,
        status: Status,
        duration: Duration,
        warning_count: usize,
    ) {
        self.steps.push(super::WorkflowStepRecord {
            layer: layer.to_string(),
            step: step.to_string(),
            command: command.to_string(),
            status,
            warning_count,
            duration,
        });

        if self.config.mode == TerminalMode::Json {
            let summary = StepSummary::new(step, status, warning_count);
            print!(
                "{}",
                render_document(
                    &Document::new(vec![Block::step_finished(summary)]),
                    &self.config
                )
            );
        }
    }

    pub fn record_warning(&mut self, warning: WarningDiagnostic) {
        if self.config.mode == TerminalMode::Json {
            print!(
                "{}",
                render_document(
                    &Document::new(vec![Block::warning(warning_for_json(&warning))]),
                    &self.config
                )
            );
        }
        self.warnings.push(warning);
    }

    pub fn message(&self, message: &str) {
        print!(
            "{}",
            render_document(&Document::new(vec![Block::text(message)]), &self.config)
        );
    }

    pub fn record_shell_path_bootstrap(&self, result: &ShellPathBootstrapResult) {
        print!(
            "{}",
            super::render_shell_path_bootstrap(result, &self.config)
        );
    }

    pub fn finish(&self, status: Status) -> Result<(), String> {
        if self.config.mode != TerminalMode::Json {
            print!("{}", render_warnings(self.warnings.clone(), &self.config));
            print!("{}", render_workflow_steps(&self.steps, &self.config));
        }
        print!(
            "{}",
            render_finished(
                &self.workflow_id,
                status,
                &self.steps,
                self.warning_count(),
                self.started_at.elapsed(),
                &self.config,
            )
        );

        if self.config.deny_warnings {
            if let Some(error) = super::deny_warnings_error(self.warning_count()) {
                return Err(error);
            }
        }
        Ok(())
    }
}
