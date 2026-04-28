use crate::config::{LogoMode, TerminalConfig};
use crate::diagnostics::{Diagnostic, Severity, WarningDiagnostic};
use crate::document::{Block, Document};
use crate::redaction::safe_text;
use crate::summary::{RunSummary, StepSummary};
use crate::symbols::{Status, symbols_for};
use crate::width::{display_width, pad_display};

const RICH_PANEL_MIN_WIDTH: usize = 51;
const RICH_PANEL_MAX_WIDTH: usize = 78;

pub fn render_human(document: &Document, config: &TerminalConfig, rich: bool) -> String {
    let mut output = String::new();
    for block in &document.blocks {
        if let Some(rendered) = render_block_human(block, config, rich) {
            output.push_str(rendered.trim_end());
            output.push('\n');
        }
    }
    output
}

fn render_block_human(block: &Block, config: &TerminalConfig, rich: bool) -> Option<String> {
    match block {
        Block::Logo => match config.logo {
            LogoMode::Off => None,
            LogoMode::Compact | LogoMode::Full if rich => Some(
                "╭─ AICore OS ─────────────────────────────────────╮\n\
                 │ Composable Rust AgentOS Platform                │\n\
                 ╰─────────────────────────────────────────────────╯"
                    .to_string(),
            ),
            LogoMode::Compact | LogoMode::Full => {
                Some("AICore OS - Composable Rust AgentOS Platform".to_string())
            }
        },
        Block::Panel { title, body } if rich => Some(render_panel_rich(title, body)),
        Block::Panel { title, body } => Some(format!("{}\n{}", safe_text(title), safe_text(body))),
        Block::KeyValue(rows) => Some(render_key_value(rows)),
        Block::Table { headers, rows } => Some(render_table(headers, rows)),
        Block::Diagnostic(diagnostic) => Some(render_diagnostic(diagnostic, config)),
        Block::Markdown(markdown) | Block::Toml(markdown) | Block::Text(markdown) => {
            Some(safe_text(markdown))
        }
        Block::Json(source) => Some(render_json_block(source)),
        Block::StructuredJson { payload, .. } => Some(render_json_block(payload)),
        Block::WarningSummary { warnings, limit } => Some(render_warning_summary(warnings, *limit)),
        Block::FinalSummary(summary) | Block::RunFinished(summary) => {
            Some(render_final_summary(summary))
        }
        Block::RunStarted(name) => Some(format!("{} workflow 开始。", safe_text(name))),
        Block::StepStarted(name) => {
            let symbols = symbols_for(config);
            Some(format!("{} {}", symbols.running, safe_text(name)))
        }
        Block::StepFinished(summary) => Some(render_step_summary(summary, config)),
        Block::Warning(warning) => Some(render_warning(warning)),
    }
}

fn render_panel_rich(title: &str, body: &str) -> String {
    let title = safe_text(title);
    let body = safe_text(body);
    let lines = wrap_body_lines(&body, RICH_PANEL_MAX_WIDTH);
    let body_width = lines
        .iter()
        .map(|line| display_width(line))
        .max()
        .unwrap_or(0);
    let inner_width = RICH_PANEL_MIN_WIDTH
        .max(display_width(&title) + 1)
        .max(body_width)
        .min(RICH_PANEL_MAX_WIDTH);
    let title_line = format!(
        "╭─ {title} {}",
        "─".repeat(inner_width.saturating_sub(display_width(&title) + 1))
    );
    let mut output = format!("{title_line}╮\n");
    for line in lines {
        output.push_str(&format!("│ {} │\n", pad_display(&line, inner_width)));
    }
    output.push_str(&format!("╰{}╯", "─".repeat(inner_width + 2)));
    output
}

fn wrap_body_lines(body: &str, max_width: usize) -> Vec<String> {
    let source_lines = body.lines().collect::<Vec<_>>();
    if source_lines.is_empty() {
        return vec![String::new()];
    }

    source_lines
        .into_iter()
        .flat_map(|line| wrap_display_line(line, max_width))
        .collect()
}

fn wrap_display_line(line: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 || display_width(line) <= max_width {
        return vec![line.to_string()];
    }

    let mut lines = Vec::new();
    let mut current = String::new();
    let mut width = 0usize;
    for ch in line.chars() {
        let ch_width = display_width(&ch.to_string());
        if width > 0 && width + ch_width > max_width {
            lines.push(current.trim_end().to_string());
            current.clear();
            width = 0;
            if ch.is_whitespace() {
                continue;
            }
        }
        current.push(ch);
        width += ch_width;
    }
    if !current.is_empty() {
        lines.push(current.trim_end().to_string());
    }
    lines
}

fn render_key_value(rows: &[(String, String)]) -> String {
    let key_width = rows
        .iter()
        .map(|(key, _)| display_width(key))
        .max()
        .unwrap_or(0);
    rows.iter()
        .map(|(key, value)| {
            format!(
                "{}{} : {}",
                safe_text(key),
                " ".repeat(key_width.saturating_sub(display_width(key))),
                safe_text(value)
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_table(headers: &[String], rows: &[Vec<String>]) -> String {
    let column_count = headers.len();
    let mut widths = headers
        .iter()
        .map(|header| display_width(header))
        .collect::<Vec<_>>();
    for row in rows {
        for (index, cell) in row.iter().enumerate().take(column_count) {
            widths[index] = widths[index].max(display_width(cell));
        }
    }

    let mut output = render_table_row(headers, &widths);
    output.push('\n');
    output.push_str(
        &widths
            .iter()
            .map(|width| "-".repeat(*width))
            .collect::<Vec<_>>()
            .join("  "),
    );
    for row in rows {
        output.push('\n');
        output.push_str(&render_table_row(row, &widths));
    }
    output
}

fn render_table_row(row: &[String], widths: &[usize]) -> String {
    row.iter()
        .enumerate()
        .map(|(index, cell)| pad_display(&safe_text(cell), widths[index]))
        .collect::<Vec<_>>()
        .join("  ")
}

fn render_diagnostic(diagnostic: &Diagnostic, config: &TerminalConfig) -> String {
    let severity = format!("{:?}", diagnostic.severity);
    let severity = match diagnostic.severity {
        Severity::Info => paint(config, &severity, "36"),
        Severity::Warning => paint(config, &severity, "33"),
        Severity::Error => paint(config, &severity, "31"),
    };
    let mut output = format!("{} {}", severity, safe_text(&diagnostic.message));
    if let Some(code) = &diagnostic.code {
        output.push_str(&format!(" [{code}]"));
    }
    if let Some(path) = &diagnostic.path {
        output.push_str(&format!(" at {}", safe_text(path)));
        if let Some(line) = diagnostic.line {
            output.push_str(&format!(":{line}"));
        }
        if let Some(column) = diagnostic.column {
            output.push_str(&format!(":{column}"));
        }
    }
    if let Some(help) = &diagnostic.help {
        output.push_str(&format!("\nhelp: {}", safe_text(help)));
    }
    output
}

fn render_json_block(source: &str) -> String {
    match serde_json::from_str::<serde_json::Value>(source) {
        Ok(value) => serde_json::to_string_pretty(&value).unwrap_or_else(|_| safe_text(source)),
        Err(error) => render_diagnostic(
            &Diagnostic::error("AICORE_JSON_INVALID", &format!("无效 JSON: {error}")),
            &TerminalConfig::plain_for_tests(),
        ),
    }
}

fn render_warning_summary(warnings: &[WarningDiagnostic], limit: usize) -> String {
    let mut output = format!("Warnings {}", warnings.len());
    if warnings.is_empty() {
        output.push_str(" scanned this run");
        return output;
    }

    for warning in warnings.iter().take(limit) {
        output.push('\n');
        output.push_str(&render_warning(warning));
    }

    if warnings.len() > limit {
        output.push_str(&format!("\n... 还有 {} 条 warning", warnings.len() - limit));
    }
    output
}

fn render_warning(warning: &WarningDiagnostic) -> String {
    let mut output = format!(
        "[WARN] {}: {}",
        safe_text(&warning.step),
        safe_text(&warning.message)
    );
    if let Some(path) = &warning.path {
        output.push_str(&format!(" ({path}"));
        if let Some(line) = warning.line {
            output.push_str(&format!(":{line}"));
        }
        if let Some(column) = warning.column {
            output.push_str(&format!(":{column}"));
        }
        output.push(')');
    }
    output
}

fn render_final_summary(summary: &RunSummary) -> String {
    format!(
        "Summary {}: {} | Steps {} | Warnings {} scanned this run",
        safe_text(&summary.name),
        summary.status.label(),
        summary.step_count,
        summary.warning_count
    )
}

fn render_step_summary(summary: &StepSummary, config: &TerminalConfig) -> String {
    let status_text = render_status_text(summary.status, config);
    format!(
        "{} {} | Warnings {}",
        status_text,
        safe_text(&summary.name),
        summary.warning_count
    )
}

fn render_status_text(status: Status, config: &TerminalConfig) -> String {
    let symbols = symbols_for(config);
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
    match status {
        Status::Ok => paint(config, &rendered, "32"),
        Status::Warn => paint(config, &rendered, "33"),
        Status::Failed => paint(config, &rendered, "31"),
        Status::Running => paint(config, &rendered, "36"),
        Status::Info => paint(config, &rendered, "36"),
        Status::Skipped => paint(config, &rendered, "2"),
    }
}

fn paint(config: &TerminalConfig, value: &str, ansi_code: &str) -> String {
    if config.use_ansi() {
        format!("\u{1b}[{ansi_code}m{value}\u{1b}[0m")
    } else {
        value.to_string()
    }
}
