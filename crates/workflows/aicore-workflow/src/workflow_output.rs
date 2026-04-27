use std::path::Path;
use std::time::{Duration, Instant};

use aicore_terminal::{
    Block, Document, RunSummary, Status, StatusSymbols, StepSummary, SymbolMode, TerminalConfig,
    TerminalMode, WarningDiagnostic, render_document, safe_text,
};

use crate::cargo_runner::CommandReport;

const RICH_PANEL_WIDTH: usize = 76;
const ANSI_RESET: &str = "\u{1b}[0m";
const ANSI_DIM: &str = "\u{1b}[2m";
const ANSI_CYAN: &str = "\u{1b}[96m";
const ANSI_GREEN: &str = "\u{1b}[32m";
const ANSI_YELLOW: &str = "\u{1b}[33m";
const ANSI_RED: &str = "\u{1b}[31m";

pub struct WorkflowOutput {
    config: TerminalConfig,
    workflow_id: String,
    repo_root: String,
    target: String,
    started_at: Instant,
    warnings: Vec<WarningDiagnostic>,
    steps: Vec<WorkflowStepRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WorkflowStepRecord {
    layer: String,
    step: String,
    command: String,
    status: Status,
    warning_count: usize,
    duration: Duration,
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
        self.steps.push(WorkflowStepRecord {
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
        self.steps.push(WorkflowStepRecord {
            layer: layer.to_string(),
            step: step.to_string(),
            command: command.to_string(),
            status,
            warning_count: 0,
            duration,
        });

        if self.config.mode == TerminalMode::Json {
            let summary = StepSummary::new(step, status, 0);
            print!(
                "{}",
                render_document(
                    &Document::new(vec![Block::step_finished(summary)]),
                    &self.config
                )
            );
        }
    }

    pub fn message(&self, message: &str) {
        print!(
            "{}",
            render_document(&Document::new(vec![Block::text(message)]), &self.config)
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

fn render_run_started(
    workflow_id: &str,
    repo_root: &str,
    target: &str,
    config: &TerminalConfig,
) -> String {
    if config.mode == TerminalMode::Json {
        return render_document(
            &Document::new(vec![Block::run_started(workflow_id)]),
            config,
        );
    }

    render_header_panel(workflow_id, repo_root, target, config)
}

fn render_finished(
    workflow_id: &str,
    status: Status,
    steps: &[WorkflowStepRecord],
    warning_count: usize,
    duration: Duration,
    config: &TerminalConfig,
) -> String {
    if config.mode == TerminalMode::Json {
        return render_document(
            &Document::new(vec![Block::run_finished(RunSummary::new(
                workflow_id,
                status,
                steps.len(),
                warning_count,
            ))]),
            config,
        );
    }

    let ok_count = steps
        .iter()
        .filter(|step| step.status == Status::Ok)
        .count();
    let failed_count = steps
        .iter()
        .filter(|step| step.status == Status::Failed)
        .count();
    let rows = vec![
        ("Workflow", safe_text(workflow_id)),
        ("Status", status_text(status, config)),
        (
            "Steps",
            format!(
                "{} total / {ok_count} ok / {failed_count} failed",
                steps.len()
            ),
        ),
        ("Warnings", format!("{warning_count} scanned this run")),
        ("Duration", format_duration(duration)),
        ("Result", result_text(status, config)),
    ];
    let body = if config.mode == TerminalMode::Rich {
        render_colon_rows(&rows, config)
    } else {
        render_key_rows(&rows)
    };
    render_panel("Summary", &body, config)
}

fn render_warnings(warnings: Vec<WarningDiagnostic>, config: &TerminalConfig) -> String {
    if warnings.is_empty() {
        return String::new();
    }

    let mut lines = vec![format!("Warnings {}", warnings.len())];
    for warning in warnings.iter().take(20) {
        lines.push(render_warning_line(warning));
    }
    if warnings.len() > 20 {
        lines.push(format!("... 还有 {} 条 warning", warnings.len() - 20));
    }
    render_panel("Warnings", &lines.join("\n"), config)
}

fn render_workflow_steps(steps: &[WorkflowStepRecord], config: &TerminalConfig) -> String {
    if steps.is_empty() {
        return String::new();
    }

    let headers = ["#", "Layer", "Step", "Status", "Warn", "Time"];
    let rows = steps
        .iter()
        .enumerate()
        .map(|(index, step)| {
            vec![
                row_number(index + 1, config),
                safe_text(&step.layer),
                safe_text(&step.step),
                status_text(step.status, config),
                step.warning_count.to_string(),
                format_duration(step.duration),
            ]
        })
        .collect::<Vec<_>>();
    let table = render_table(&headers, &rows, config);
    render_panel("Workflow Steps", &table, config)
}

fn render_warning_line(warning: &WarningDiagnostic) -> String {
    let mut output = format!(
        "[WARN] {}: {}",
        safe_text(&warning.step),
        safe_text(&warning.message)
    );
    if let Some(path) = &warning.path {
        output.push_str(&format!(" ({})", safe_text(path)));
    }
    output
}

fn render_table(headers: &[&str], rows: &[Vec<String>], config: &TerminalConfig) -> String {
    let mut widths = headers
        .iter()
        .map(|header| terminal_width(header))
        .collect::<Vec<_>>();
    for row in rows {
        for (index, cell) in row.iter().enumerate() {
            widths[index] = widths[index].max(visible_width(cell));
        }
    }

    let header_line = render_table_row(
        &headers
            .iter()
            .map(|value| table_header(value, config))
            .collect::<Vec<_>>(),
        &widths,
    );
    let separator = if config.mode == TerminalMode::Rich {
        dim(&"─".repeat(visible_width(&header_line)), config)
    } else {
        render_table_row(
            &widths
                .iter()
                .map(|width| "-".repeat(*width))
                .collect::<Vec<_>>(),
            &widths,
        )
    };
    let mut lines = vec![header_line];
    lines.push(separator.clone());
    for (index, row) in rows.iter().enumerate() {
        lines.push(render_table_row(row, &widths));
        if config.mode == TerminalMode::Rich && index + 1 < rows.len() {
            lines.push(separator.clone());
        }
    }
    lines.join("\n")
}

fn render_table_row(row: &[String], widths: &[usize]) -> String {
    row.iter()
        .enumerate()
        .map(|(index, cell)| pad_visible(cell, widths[index]))
        .collect::<Vec<_>>()
        .join("  ")
}

fn render_key_rows(rows: &[(&str, String)]) -> String {
    let key_width = rows
        .iter()
        .filter(|(_, value)| !value.is_empty())
        .map(|(key, _)| terminal_width(key))
        .max()
        .unwrap_or(0);

    rows.iter()
        .map(|(key, value)| {
            if value.is_empty() {
                safe_text(key)
            } else {
                format!(
                    "{}{}  {}",
                    safe_text(key),
                    " ".repeat(key_width.saturating_sub(terminal_width(key))),
                    value
                )
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_colon_rows(rows: &[(&str, String)], config: &TerminalConfig) -> String {
    let key_width = rows
        .iter()
        .filter(|(_, value)| !value.is_empty())
        .map(|(key, _)| terminal_width(key))
        .max()
        .unwrap_or(0);

    rows.iter()
        .map(|(key, value)| {
            if value.is_empty() {
                safe_text(key)
            } else {
                let label = format!(
                    "{}{}",
                    safe_text(key),
                    " ".repeat(key_width.saturating_sub(terminal_width(key)))
                );
                format!("{} : {}", dim(&label, config), value)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_panel(title: &str, body: &str, config: &TerminalConfig) -> String {
    if config.mode == TerminalMode::Rich {
        return render_rich_panel(title, body, config);
    }

    format!("{}\n{}\n", safe_text(title), safe_text(body))
}

fn render_rich_panel(title: &str, body: &str, config: &TerminalConfig) -> String {
    let title = render_section_title(title, config);
    let lines = body.lines().collect::<Vec<_>>();
    let body_width = lines
        .iter()
        .map(|line| visible_width(line))
        .max()
        .unwrap_or(0);
    let inner_width = body_width
        .max(visible_width(&title) + 4)
        .max(RICH_PANEL_WIDTH);
    let dash_count = inner_width.saturating_sub(visible_width(&title) + 3);

    let mut output = format!(
        "{}{}{}{}{}\n",
        border("╭─ ", config),
        title,
        border(" ", config),
        border(&"─".repeat(dash_count), config),
        border("╮", config)
    );
    for line in lines {
        output.push_str(&format!(
            "{} {}{} {}\n",
            border("│", config),
            line,
            " ".repeat(inner_width.saturating_sub(visible_width(line) + 2)),
            border("│", config)
        ));
    }
    output.push_str(&format!(
        "{}{}{}\n",
        border("╰", config),
        border(&"─".repeat(inner_width), config),
        border("╯", config)
    ));
    output
}

fn render_header_panel(
    workflow_id: &str,
    repo_root: &str,
    target: &str,
    config: &TerminalConfig,
) -> String {
    if config.mode != TerminalMode::Rich {
        let body = render_plain_header_body(workflow_id, repo_root, target, config);
        return render_panel("AICore OS", &body, config);
    }

    let body = render_header_body(workflow_id, repo_root, target, config);
    let lines = body.lines().collect::<Vec<_>>();
    let body_width = lines
        .iter()
        .map(|line| visible_width(line))
        .max()
        .unwrap_or(0);
    let inner_width = body_width.max(RICH_PANEL_WIDTH);
    let top_border = if config.use_ansi() {
        accent(&"─".repeat(inner_width), config)
    } else {
        "─".repeat(inner_width)
    };

    let mut output = format!(
        "{}{}{}\n",
        border("╭", config),
        top_border,
        border("╮", config)
    );
    for line in lines {
        output.push_str(&format!(
            "{} {}{} {}\n",
            border("│", config),
            line,
            " ".repeat(inner_width.saturating_sub(visible_width(line) + 2)),
            border("│", config)
        ));
    }
    output.push_str(&format!(
        "{}{}{}\n",
        border("╰", config),
        border(&"─".repeat(inner_width), config),
        border("╯", config)
    ));
    output
}

fn status_text(status: Status, config: &TerminalConfig) -> String {
    let symbols = match config.symbols {
        SymbolMode::Unicode => StatusSymbols::unicode(),
        SymbolMode::Ascii => StatusSymbols::ascii(),
    };
    let symbol = match status {
        Status::Ok => symbols.ok,
        Status::Warn => symbols.warn,
        Status::Failed => symbols.failed,
        Status::Running => symbols.running,
        Status::Info => symbols.info,
        Status::Skipped => symbols.skipped,
    };
    let rendered = if symbol.starts_with('[') {
        symbol.to_string()
    } else {
        format!("{} {}", symbol, status.label())
    };
    if !config.use_ansi() {
        return rendered;
    }

    let style = match status {
        Status::Ok => ANSI_GREEN,
        Status::Warn => ANSI_YELLOW,
        Status::Failed => ANSI_RED,
        Status::Running | Status::Info => ANSI_CYAN,
        Status::Skipped => ANSI_DIM,
    };
    if symbol.starts_with('[') {
        return format!("{style}{rendered}{ANSI_RESET}");
    }
    format!(
        "{style}{symbol}{ANSI_RESET} {style}{}{ANSI_RESET}",
        status.label()
    )
}

fn terminal_mode_label(mode: TerminalMode) -> &'static str {
    match mode {
        TerminalMode::Rich => "rich",
        TerminalMode::Plain => "plain",
        TerminalMode::Json => "json",
    }
}

fn warning_policy_label(config: &TerminalConfig) -> &'static str {
    if config.deny_warnings {
        "deny"
    } else {
        "report"
    }
}

fn result_label(status: Status) -> &'static str {
    match status {
        Status::Ok => "workflow completed successfully",
        Status::Warn => "workflow completed with warnings",
        Status::Failed => "workflow failed",
        Status::Running => "workflow running",
        Status::Info => "workflow reported information",
        Status::Skipped => "workflow skipped",
    }
}

fn result_text(status: Status, config: &TerminalConfig) -> String {
    let label = result_label(status);
    if !config.use_ansi() {
        return label.to_string();
    }

    match status {
        Status::Ok => success(label, config),
        Status::Warn => warning(label, config),
        Status::Failed => failure(label, config),
        Status::Running | Status::Info => accent(label, config),
        Status::Skipped => dim(label, config),
    }
}

fn render_header_body(
    workflow_id: &str,
    repo_root: &str,
    target: &str,
    config: &TerminalConfig,
) -> String {
    let brand = format!(
        "{} {} {} {}",
        accent(icon("⎇", config), config),
        accent("AICore OS", config),
        dim("—", config),
        safe_text("Composable Rust AgentOS Platform")
    );
    let workflow = render_rich_meta_pair("⎇", "Workflow", workflow_id, config);
    let mode = render_rich_meta_pair("◈", "Mode", terminal_mode_label(config.mode), config);
    let target = render_rich_meta_pair("◎", "Target", target, config);
    let warnings = render_rich_meta_pair("⚠", "Warnings", warning_policy_label(config), config);
    let root = render_rich_meta_pair("□", "Root", repo_root, config);

    format!(
        "{brand}\n\n{}  {}\n{}  {}\n{}",
        pad_visible(&workflow, 46),
        mode,
        pad_visible(&target, 46),
        warnings,
        root
    )
}

fn render_plain_header_body(
    workflow_id: &str,
    repo_root: &str,
    target: &str,
    config: &TerminalConfig,
) -> String {
    let workflow = render_inline_pair("Workflow", workflow_id);
    let mode = render_inline_pair("Mode", terminal_mode_label(config.mode));
    let target = render_inline_pair("Target", target);
    let warnings = render_inline_pair("Warnings", warning_policy_label(config));
    let root = render_inline_pair("Root", repo_root);

    format!(
        "Composable Rust AgentOS Platform\n\n{}  {}\n{}  {}\n{}",
        pad_visible(&workflow, 30),
        mode,
        pad_visible(&target, 30),
        warnings,
        root
    )
}

fn render_inline_pair(key: &str, value: &str) -> String {
    format!(
        "{key:<10}{value}",
        key = safe_text(key),
        value = safe_text(value)
    )
}

fn render_rich_meta_pair(
    icon_value: &str,
    key: &str,
    value: &str,
    config: &TerminalConfig,
) -> String {
    let label = format!("{:<8}", safe_text(key));
    format!(
        "{} {} : {}",
        accent(icon(icon_value, config), config),
        dim(&label, config),
        safe_text(value)
    )
}

fn render_section_title(title: &str, config: &TerminalConfig) -> String {
    let title = safe_text(title);
    let icon_value = match title.as_str() {
        "Workflow Steps" => "☷",
        "Summary" => "▥",
        "Warnings" => "⚠",
        _ => "◇",
    };
    format!(
        "{} {}",
        accent(icon(icon_value, config), config),
        accent(&title, config)
    )
}

fn table_header(value: &str, config: &TerminalConfig) -> String {
    if config.mode == TerminalMode::Rich {
        dim(value, config)
    } else {
        safe_text(value)
    }
}

fn row_number(value: usize, config: &TerminalConfig) -> String {
    let text = value.to_string();
    if config.mode == TerminalMode::Rich {
        accent(&text, config)
    } else {
        text
    }
}

fn icon<'a>(unicode: &'a str, config: &TerminalConfig) -> &'a str {
    match config.symbols {
        SymbolMode::Unicode => unicode,
        SymbolMode::Ascii => "*",
    }
}

fn border(value: &str, config: &TerminalConfig) -> String {
    dim(value, config)
}

fn accent(value: &str, config: &TerminalConfig) -> String {
    style(value, ANSI_CYAN, config)
}

fn success(value: &str, config: &TerminalConfig) -> String {
    style(value, ANSI_GREEN, config)
}

fn warning(value: &str, config: &TerminalConfig) -> String {
    style(value, ANSI_YELLOW, config)
}

fn failure(value: &str, config: &TerminalConfig) -> String {
    style(value, ANSI_RED, config)
}

fn dim(value: &str, config: &TerminalConfig) -> String {
    style(value, ANSI_DIM, config)
}

fn style(value: &str, code: &str, config: &TerminalConfig) -> String {
    if config.use_ansi() {
        format!("{code}{value}{ANSI_RESET}")
    } else {
        safe_text(value)
    }
}

fn format_duration(duration: Duration) -> String {
    format!("{:.2}s", duration.as_secs_f64())
}

fn visible_width(value: &str) -> usize {
    terminal_width(&strip_ansi(value))
}

fn terminal_width(value: &str) -> usize {
    value.chars().map(char_width).sum()
}

fn char_width(ch: char) -> usize {
    if matches!(
        ch,
        '\u{1100}'..='\u{115f}'
            | '\u{2e80}'..='\u{a4cf}'
            | '\u{ac00}'..='\u{d7a3}'
            | '\u{f900}'..='\u{faff}'
            | '\u{fe10}'..='\u{fe19}'
            | '\u{fe30}'..='\u{fe6f}'
            | '\u{ff00}'..='\u{ff60}'
            | '\u{ffe0}'..='\u{ffe6}'
    ) {
        2
    } else {
        1
    }
}

fn pad_visible(value: &str, width: usize) -> String {
    let visible = visible_width(value);
    if visible >= width {
        value.to_string()
    } else {
        format!("{}{}", value, " ".repeat(width - visible))
    }
}

fn strip_ansi(value: &str) -> String {
    let mut output = String::new();
    let mut chars = value.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' && chars.peek() == Some(&'[') {
            chars.next();
            for next in chars.by_ref() {
                if next == 'm' {
                    break;
                }
            }
        } else {
            output.push(ch);
        }
    }
    output
}

#[cfg(test)]
fn render_run_started_for_tests(workflow_name: &str, config: &TerminalConfig) -> String {
    render_run_started(workflow_name, "/repo", "foundation + kernel", config)
}

#[cfg(test)]
fn render_finished_for_tests(
    workflow_name: &str,
    status: Status,
    step_count: usize,
    warning_count: usize,
    config: &TerminalConfig,
) -> String {
    render_finished(
        workflow_name,
        status,
        &sample_step_records(step_count),
        warning_count,
        Duration::from_millis(1420),
        config,
    )
}

#[cfg(test)]
fn render_warnings_for_tests(warnings: Vec<WarningDiagnostic>, config: &TerminalConfig) -> String {
    render_warnings(warnings, config)
}

#[cfg(test)]
fn render_workflow_steps_for_tests(config: &TerminalConfig) -> String {
    render_workflow_steps(&sample_step_records(8), config)
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
                duration: Duration::from_millis(80 + (index as u64 * 10)),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use aicore_terminal::{
        Status, TerminalCapabilities, TerminalConfig, TerminalEnv, WarningDiagnostic,
    };

    use crate::cargo_runner::CommandReport;

    use super::*;

    #[test]
    fn workflow_report_includes_logo_header_in_rich_mode() {
        let output = render_run_started_for_tests("kernel", &TerminalConfig::rich_for_tests());

        assert!(output.contains("AICore OS"));
        assert!(output.contains("Workflow"));
        assert!(output.contains("Mode"));
        assert!(output.contains("Root"));
        assert!(output.contains("Target"));
        assert!(output.contains("Warnings"));
        assert!(output.contains('╭'));
        assert!(output.contains('╰'));
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

        assert!(output.contains("Summary"));
        assert!(output.contains("Workflow"));
        assert!(output.contains("Status"));
        assert!(output.contains("Steps"));
        assert!(output.contains("Warnings  0 scanned this run"));
    }

    #[test]
    fn workflow_rich_summary_renders_panel() {
        let output =
            render_finished_for_tests("core", Status::Ok, 8, 0, &TerminalConfig::rich_for_tests());

        assert!(output.contains("Summary"));
        assert!(output.contains("Workflow"));
        assert!(output.contains("core"));
        assert!(output.contains("✓ OK"));
        assert!(output.contains("8 total"));
        assert!(output.contains("0 scanned this run"));
    }

    #[test]
    fn workflow_rich_output_renders_step_table() {
        let output = render_workflow_steps_for_tests(&TerminalConfig::rich_for_tests());

        assert!(output.contains("Workflow Steps"));
        assert!(output.contains("Layer"));
        assert!(output.contains("Step"));
        assert!(output.contains("Status"));
        assert!(output.contains("Warn"));
        assert!(output.contains("Time"));
        assert!(output.contains("foundation"));
        assert!(output.contains("test"));
        assert!(!output.contains('⏳'));
    }

    #[test]
    fn workflow_rich_table_has_header_separator() {
        let output = render_workflow_steps_for_tests(&TerminalConfig::rich_for_tests());

        let separator = output
            .lines()
            .map(strip_ansi)
            .find(|line| line.starts_with('│') && line.contains('─') && !line.contains("Layer"))
            .expect("rich table should render a separator line");
        assert!(!separator.contains("─  ─"));
    }

    #[test]
    fn workflow_rich_header_uses_two_column_summary_layout() {
        let output = render_run_started_for_tests("core", &TerminalConfig::rich_for_tests());

        assert!(output.contains("Workflow : core"));
        assert!(output.contains("Mode     : rich"));
        assert!(
            output
                .lines()
                .any(|line| line.contains("Workflow") && line.contains("Mode"))
        );
        assert!(
            output
                .lines()
                .any(|line| line.contains("Target") && line.contains("Warnings"))
        );
    }

    #[test]
    fn workflow_rich_header_matches_target_structure() {
        let output = render_run_started_for_tests("core", &TerminalConfig::rich_for_tests());

        assert!(output.contains("⎇ AICore OS"));
        assert!(output.contains("AICore OS — Composable Rust AgentOS Platform"));
        assert!(
            output
                .lines()
                .any(|line| line.contains("Workflow") && line.contains("Mode"))
        );
        assert!(
            output
                .lines()
                .any(|line| line.contains("Target") && line.contains("Warnings"))
        );
        assert!(output.contains("Workflow : core"));
        assert!(output.contains("Target   : foundation + kernel"));
        assert!(output.contains("Root     : /repo"));
    }

    #[test]
    fn workflow_rich_header_uses_accent_brand_and_metadata_icons() {
        let output = render_run_started_for_tests("core", &rich_color_config());

        assert!(output.contains("\u{1b}[96mAICore OS\u{1b}[0m"));
        assert!(output.contains('⎇'));
        assert!(output.contains('◈'));
        assert!(output.contains('◎'));
        assert!(output.contains('□'));
        assert!(output.contains('⚠'));
    }

    #[test]
    fn workflow_rich_steps_use_accent_row_numbers_and_status_cell_only() {
        let output = render_workflow_steps_for_tests(&rich_color_config());

        assert!(output.contains("\u{1b}[96m1\u{1b}[0m"));
        assert!(output.contains("\u{1b}[32m✓\u{1b}[0m"));
        assert!(output.contains("\u{1b}[32mOK\u{1b}[0m"));
        assert!(!output.contains("\u{1b}[32mfoundation"));
        assert!(!output.contains('⏳'));
    }

    #[test]
    fn workflow_rich_summary_uses_colon_labels_and_green_result() {
        let output = render_finished_for_tests("core", Status::Ok, 8, 0, &rich_color_config());
        let plain = strip_ansi(&output);

        assert!(plain.contains("Workflow : core"));
        assert!(plain.contains("Status   :"));
        assert!(plain.contains("Result   :"));
        assert!(output.contains("\u{1b}[32mworkflow completed successfully\u{1b}[0m"));
    }

    #[test]
    fn workflow_rich_colored_panels_have_aligned_right_border() {
        for output in [
            render_run_started_for_tests("core", &rich_color_config()),
            render_workflow_steps_for_tests(&rich_color_config()),
            render_finished_for_tests("core", Status::Ok, 8, 0, &rich_color_config()),
        ] {
            assert_panel_lines_have_equal_width(&output);
        }
    }

    #[test]
    fn workflow_rich_output_handles_mixed_chinese_english_width() {
        let output = render_panel(
            "Summary",
            "Result   : workflow completed successfully\n说明     : 底层与内核层 OK",
            &rich_color_config(),
        );

        assert_panel_lines_have_equal_width(&output);
    }

    #[test]
    fn workflow_rich_panels_have_aligned_right_border() {
        for output in [
            render_run_started_for_tests("core", &TerminalConfig::rich_for_tests()),
            render_workflow_steps_for_tests(&TerminalConfig::rich_for_tests()),
            render_finished_for_tests("core", Status::Ok, 8, 0, &TerminalConfig::rich_for_tests()),
        ] {
            assert_panel_lines_have_equal_width(&output);
        }
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
        let steps = render_workflow_steps_for_tests(&TerminalConfig::plain_for_tests());

        assert!(!output.contains("\u{1b}"));
        assert!(!output.contains('╭'));
        assert!(!output.contains('⎇'));
        assert!(!output.contains('◈'));
        assert!(!steps.contains('─'));
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

    fn assert_panel_lines_have_equal_width(output: &str) {
        let widths = output
            .lines()
            .map(strip_ansi)
            .filter(|line| line.starts_with('╭') || line.starts_with('│') || line.starts_with('╰'))
            .map(|line| test_terminal_width(&line))
            .collect::<Vec<_>>();
        assert!(!widths.is_empty());
        assert!(
            widths.windows(2).all(|pair| pair[0] == pair[1]),
            "panel line widths differ: {widths:?}\n{output}"
        );
    }

    fn rich_color_config() -> TerminalConfig {
        TerminalConfig::from_env_and_capabilities(
            &TerminalEnv::from_pairs([("AICORE_TERMINAL", "rich"), ("AICORE_COLOR", "always")]),
            TerminalCapabilities { is_tty: true },
        )
    }

    fn test_terminal_width(line: &str) -> usize {
        strip_ansi(line)
            .chars()
            .map(|ch| {
                if matches!(
                    ch,
                    '\u{1100}'..='\u{115f}'
                        | '\u{2e80}'..='\u{a4cf}'
                        | '\u{ac00}'..='\u{d7a3}'
                        | '\u{f900}'..='\u{faff}'
                        | '\u{fe10}'..='\u{fe19}'
                        | '\u{fe30}'..='\u{fe6f}'
                        | '\u{ff00}'..='\u{ff60}'
                        | '\u{ffe0}'..='\u{ffe6}'
                ) {
                    2
                } else {
                    1
                }
            })
            .sum()
    }
}
