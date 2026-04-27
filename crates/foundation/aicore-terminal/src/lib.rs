use std::collections::BTreeMap;
use std::io::IsTerminal;

use serde::Serialize;
use serde_json::json;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalEnv {
    values: BTreeMap<String, String>,
}

impl TerminalEnv {
    pub fn current() -> Self {
        Self {
            values: std::env::vars().collect(),
        }
    }

    pub fn from_pairs<const N: usize>(pairs: [(&str, &str); N]) -> Self {
        Self {
            values: pairs
                .into_iter()
                .map(|(key, value)| (key.to_string(), value.to_string()))
                .collect(),
        }
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(String::as_str)
    }

    pub fn is_truthy(&self, key: &str) -> bool {
        matches!(self.get(key), Some("1" | "true" | "TRUE" | "yes" | "YES"))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalCapabilities {
    pub is_tty: bool,
}

impl TerminalCapabilities {
    pub fn stdout() -> Self {
        Self {
            is_tty: std::io::stdout().is_terminal(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminalMode {
    Rich,
    Plain,
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMode {
    Auto,
    Always,
    Never,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogoMode {
    Compact,
    Full,
    Off,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolMode {
    Unicode,
    Ascii,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressMode {
    Auto,
    Always,
    Never,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalConfig {
    pub mode: TerminalMode,
    pub color: ColorMode,
    pub logo: LogoMode,
    pub symbols: SymbolMode,
    pub progress: ProgressMode,
    pub verbose: bool,
    pub deny_warnings: bool,
}

impl TerminalConfig {
    pub fn current() -> Self {
        Self::from_env_and_capabilities(&TerminalEnv::current(), TerminalCapabilities::stdout())
    }

    pub fn from_env_and_capabilities(
        env: &TerminalEnv,
        capabilities: TerminalCapabilities,
    ) -> Self {
        let ci = env.is_truthy("CI");
        let requested_mode = env.get("AICORE_TERMINAL").unwrap_or("auto");
        let mode = match requested_mode {
            "rich" => TerminalMode::Rich,
            "plain" => TerminalMode::Plain,
            "json" => TerminalMode::Json,
            _ if capabilities.is_tty && !ci => TerminalMode::Rich,
            _ => TerminalMode::Plain,
        };

        let color = if mode == TerminalMode::Json || env.is_truthy("NO_COLOR") {
            ColorMode::Never
        } else {
            match env.get("AICORE_COLOR").unwrap_or("auto") {
                "always" => ColorMode::Always,
                "never" => ColorMode::Never,
                _ => ColorMode::Auto,
            }
        };

        let logo = if mode == TerminalMode::Json {
            LogoMode::Off
        } else {
            match env.get("AICORE_LOGO").unwrap_or("compact") {
                "full" => LogoMode::Full,
                "off" => LogoMode::Off,
                _ => LogoMode::Compact,
            }
        };

        let symbols = match env.get("AICORE_SYMBOLS") {
            Some("unicode") => SymbolMode::Unicode,
            Some("ascii") => SymbolMode::Ascii,
            _ if mode == TerminalMode::Rich && !ci => SymbolMode::Unicode,
            _ => SymbolMode::Ascii,
        };

        let progress = if mode == TerminalMode::Json || env.is_truthy("AICORE_VERBOSE") {
            ProgressMode::Never
        } else {
            match env.get("AICORE_PROGRESS").unwrap_or("auto") {
                "always" => ProgressMode::Always,
                "never" => ProgressMode::Never,
                _ => ProgressMode::Auto,
            }
        };

        Self {
            mode,
            color,
            logo,
            symbols,
            progress,
            verbose: env.is_truthy("AICORE_VERBOSE"),
            deny_warnings: env.is_truthy("AICORE_WORKFLOW_DENY_WARNINGS"),
        }
    }

    pub fn use_ansi(&self) -> bool {
        match self.color {
            ColorMode::Always => self.mode != TerminalMode::Json,
            ColorMode::Never => false,
            ColorMode::Auto => self.mode == TerminalMode::Rich,
        }
    }

    pub fn rich_for_tests() -> Self {
        Self {
            mode: TerminalMode::Rich,
            color: ColorMode::Never,
            logo: LogoMode::Compact,
            symbols: SymbolMode::Unicode,
            progress: ProgressMode::Never,
            verbose: false,
            deny_warnings: false,
        }
    }

    pub fn plain_for_tests() -> Self {
        Self {
            mode: TerminalMode::Plain,
            color: ColorMode::Never,
            logo: LogoMode::Off,
            symbols: SymbolMode::Ascii,
            progress: ProgressMode::Never,
            verbose: false,
            deny_warnings: false,
        }
    }

    pub fn json_for_tests() -> Self {
        Self {
            mode: TerminalMode::Json,
            color: ColorMode::Never,
            logo: LogoMode::Off,
            symbols: SymbolMode::Ascii,
            progress: ProgressMode::Never,
            verbose: false,
            deny_warnings: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusSymbols {
    pub ok: &'static str,
    pub warn: &'static str,
    pub failed: &'static str,
    pub running: &'static str,
    pub info: &'static str,
    pub skipped: &'static str,
}

impl StatusSymbols {
    pub fn unicode() -> Self {
        Self {
            ok: "✓",
            warn: "⚠",
            failed: "✗",
            running: "⏳",
            info: "•",
            skipped: "–",
        }
    }

    pub fn ascii() -> Self {
        Self {
            ok: "[OK]",
            warn: "[WARN]",
            failed: "[FAILED]",
            running: "[RUNNING]",
            info: "[INFO]",
            skipped: "[SKIPPED]",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Ok,
    Warn,
    Failed,
    Running,
    Info,
    Skipped,
}

impl Status {
    fn label(self) -> &'static str {
        match self {
            Self::Ok => "OK",
            Self::Warn => "WARN",
            Self::Failed => "FAILED",
            Self::Running => "RUNNING",
            Self::Info => "INFO",
            Self::Skipped => "SKIPPED",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: Option<String>,
    pub message: String,
    pub path: Option<String>,
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub help: Option<String>,
}

impl Diagnostic {
    pub fn warning(code: &str, message: &str) -> Self {
        Self {
            severity: Severity::Warning,
            code: Some(code.to_string()),
            message: message.to_string(),
            path: None,
            line: None,
            column: None,
            help: None,
        }
    }

    pub fn error(code: &str, message: &str) -> Self {
        Self {
            severity: Severity::Error,
            code: Some(code.to_string()),
            message: message.to_string(),
            path: None,
            line: None,
            column: None,
            help: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WarningSource {
    CargoDiagnostic,
    RustcRendered,
    Rustdoc,
    BuildScript,
    TextScanner,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WarningDiagnostic {
    pub step: String,
    pub message: String,
    pub path: Option<String>,
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub source: WarningSource,
    pub raw_lines: Vec<String>,
}

impl WarningDiagnostic {
    pub fn new(step: &str, message: &str) -> Self {
        Self {
            step: step.to_string(),
            message: message.to_string(),
            path: None,
            line: None,
            column: None,
            source: WarningSource::TextScanner,
            raw_lines: vec![message.to_string()],
        }
    }

    pub fn with_source(mut self, source: WarningSource) -> Self {
        self.source = source;
        self
    }

    pub fn with_location(mut self, path: &str, line: u32, column: u32) -> Self {
        self.path = Some(path.to_string());
        self.line = Some(line);
        self.column = Some(column);
        self
    }

    pub fn fingerprint(&self) -> String {
        format!(
            "{}|{}|{}|{}|{}",
            self.step,
            self.path.as_deref().unwrap_or("-"),
            self.line
                .map(|line| line.to_string())
                .unwrap_or_else(|| "-".to_string()),
            self.column
                .map(|column| column.to_string())
                .unwrap_or_else(|| "-".to_string()),
            normalize_warning_message(&self.message)
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StepSummary {
    pub name: String,
    pub status: Status,
    pub warning_count: usize,
}

impl StepSummary {
    pub fn new(name: &str, status: Status, warning_count: usize) -> Self {
        Self {
            name: name.to_string(),
            status,
            warning_count,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RunSummary {
    pub name: String,
    pub status: Status,
    pub step_count: usize,
    pub warning_count: usize,
}

impl RunSummary {
    pub fn new(name: &str, status: Status, step_count: usize, warning_count: usize) -> Self {
        Self {
            name: name.to_string(),
            status,
            step_count,
            warning_count,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    pub blocks: Vec<Block>,
}

impl Document {
    pub fn new(blocks: Vec<Block>) -> Self {
        Self { blocks }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    Logo,
    Panel {
        title: String,
        body: String,
    },
    KeyValue(Vec<(String, String)>),
    Table {
        headers: Vec<String>,
        rows: Vec<Vec<String>>,
    },
    Diagnostic(Diagnostic),
    Markdown(String),
    Json(String),
    Toml(String),
    Text(String),
    WarningSummary {
        warnings: Vec<WarningDiagnostic>,
        limit: usize,
    },
    FinalSummary(RunSummary),
    RunStarted(String),
    StepStarted(String),
    StepFinished(StepSummary),
    Warning(WarningDiagnostic),
    RunFinished(RunSummary),
}

impl Block {
    pub fn logo() -> Self {
        Self::Logo
    }

    pub fn panel(title: &str, body: &str) -> Self {
        Self::Panel {
            title: title.to_string(),
            body: body.to_string(),
        }
    }

    pub fn key_value(rows: Vec<(&str, &str)>) -> Self {
        Self::KeyValue(
            rows.into_iter()
                .map(|(key, value)| (key.to_string(), value.to_string()))
                .collect(),
        )
    }

    pub fn table(headers: Vec<&str>, rows: Vec<Vec<&str>>) -> Self {
        Self::Table {
            headers: headers.into_iter().map(ToString::to_string).collect(),
            rows: rows
                .into_iter()
                .map(|row| row.into_iter().map(ToString::to_string).collect())
                .collect(),
        }
    }

    pub fn diagnostic(diagnostic: Diagnostic) -> Self {
        Self::Diagnostic(diagnostic)
    }

    pub fn markdown(markdown: &str) -> Self {
        Self::Markdown(markdown.to_string())
    }

    pub fn json(json: &str) -> Self {
        Self::Json(json.to_string())
    }

    pub fn toml(toml: &str) -> Self {
        Self::Toml(toml.to_string())
    }

    pub fn text(text: &str) -> Self {
        Self::Text(text.to_string())
    }

    pub fn warning_summary(warnings: Vec<WarningDiagnostic>, limit: usize) -> Self {
        Self::WarningSummary { warnings, limit }
    }

    pub fn final_summary(summary: RunSummary) -> Self {
        Self::FinalSummary(summary)
    }

    pub fn run_started(name: &str) -> Self {
        Self::RunStarted(name.to_string())
    }

    pub fn step_started(name: &str) -> Self {
        Self::StepStarted(name.to_string())
    }

    pub fn step_finished(summary: StepSummary) -> Self {
        Self::StepFinished(summary)
    }

    pub fn warning(warning: WarningDiagnostic) -> Self {
        Self::Warning(warning)
    }

    pub fn run_finished(summary: RunSummary) -> Self {
        Self::RunFinished(summary)
    }
}

pub fn render_document(document: &Document, config: &TerminalConfig) -> String {
    match config.mode {
        TerminalMode::Json => render_json_lines(document),
        TerminalMode::Rich => render_human(document, config, true),
        TerminalMode::Plain => render_human(document, config, false),
    }
}

pub fn sanitize_text(value: &str) -> String {
    value
        .chars()
        .filter(|ch| *ch == '\n' || *ch == '\t' || !ch.is_control())
        .collect()
}

pub fn redact_text(value: &str) -> String {
    value
        .split_inclusive(char::is_whitespace)
        .map(redact_token)
        .collect()
}

pub fn safe_text(value: &str) -> String {
    redact_text(&sanitize_text(value))
}

fn redact_token(token: &str) -> String {
    let trimmed = token.trim_end_matches(char::is_whitespace);
    let suffix = &token[trimmed.len()..];
    let lower = trimmed.to_ascii_lowercase();
    if lower.contains("sk-")
        || lower.contains("secret_ref")
        || lower.contains("credential_lease_ref")
        || lower.contains("api_key")
    {
        format!("[REDACTED]{suffix}")
    } else {
        token.to_string()
    }
}

fn render_human(document: &Document, config: &TerminalConfig, rich: bool) -> String {
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
    let width = 53usize.max(display_width(&title) + 6);
    let title_line = format!(
        "╭─ {title} {}",
        "─".repeat(width.saturating_sub(display_width(&title) + 4))
    );
    let mut output = format!("{title_line}╮\n");
    for line in body.lines() {
        output.push_str(&format!(
            "│ {}{} │\n",
            line,
            " ".repeat(width.saturating_sub(display_width(line) + 2))
        ));
    }
    output.push_str(&format!("╰{}╯", "─".repeat(width)));
    output
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
    let symbols = symbols_for(config);
    let symbol = match summary.status {
        Status::Ok => symbols.ok,
        Status::Warn => symbols.warn,
        Status::Failed => symbols.failed,
        Status::Running => symbols.running,
        Status::Info => symbols.info,
        Status::Skipped => symbols.skipped,
    };
    let rendered = format!(
        "{} {} | Warnings {}",
        symbol,
        safe_text(&summary.name),
        summary.warning_count
    );
    match summary.status {
        Status::Ok => paint(config, &rendered, "32"),
        Status::Warn => paint(config, &rendered, "33"),
        Status::Failed => paint(config, &rendered, "31"),
        Status::Running => paint(config, &rendered, "36"),
        Status::Info => paint(config, &rendered, "36"),
        Status::Skipped => paint(config, &rendered, "2"),
    }
}

fn render_json_lines(document: &Document) -> String {
    let mut lines = Vec::new();
    for block in &document.blocks {
        match block {
            Block::RunStarted(name) => {
                lines.push(json_event("run.started", json!({ "name": name })))
            }
            Block::StepStarted(name) => {
                lines.push(json_event("step.started", json!({ "name": name })))
            }
            Block::StepFinished(summary) => lines.push(json_event(
                "step.finished",
                json!({ "summary": safe_step_summary(summary) }),
            )),
            Block::Warning(warning) => lines.push(json_event(
                "warning",
                json!({ "warning": safe_warning_json(warning) }),
            )),
            Block::WarningSummary { warnings, .. } => {
                for warning in warnings {
                    lines.push(json_event(
                        "warning",
                        json!({ "warning": safe_warning_json(warning) }),
                    ));
                }
            }
            Block::RunFinished(summary) | Block::FinalSummary(summary) => lines.push(json_event(
                "run.finished",
                json!({ "summary": safe_run_summary(summary) }),
            )),
            Block::Logo => lines.push(json_event("block.logo", json!({ "enabled": false }))),
            Block::Panel { title, body } => lines.push(json_event(
                "block.panel",
                json!({ "title": safe_text(title), "body": safe_text(body) }),
            )),
            Block::KeyValue(rows) => lines.push(json_event(
                "block.key_value",
                json!({ "rows": safe_rows(rows) }),
            )),
            Block::Table { headers, rows } => lines.push(json_event(
                "block.table",
                json!({ "headers": safe_string_vec(headers), "rows": safe_table_rows(rows) }),
            )),
            Block::Diagnostic(diagnostic) => lines.push(json_event(
                "diagnostic",
                json!({ "diagnostic": safe_diagnostic_json(diagnostic) }),
            )),
            Block::Markdown(markdown) => lines.push(json_event(
                "block.markdown",
                json!({ "markdown": safe_text(markdown) }),
            )),
            Block::Json(source) => lines.push(json_event(
                "block.json",
                json!({ "json": safe_text(source) }),
            )),
            Block::Toml(source) => lines.push(json_event(
                "block.toml",
                json!({ "toml": safe_text(source) }),
            )),
            Block::Text(text) => {
                lines.push(json_event("block.text", json!({ "text": safe_text(text) })))
            }
        }
    }
    if lines.is_empty() {
        String::new()
    } else {
        format!("{}\n", lines.join("\n"))
    }
}

fn json_event(kind: &str, payload: serde_json::Value) -> String {
    serde_json::to_string(&json!({
        "schema": "aicore.terminal.v1",
        "event": kind,
        "payload": payload,
    }))
    .unwrap_or_else(|_| "{\"schema\":\"aicore.terminal.v1\",\"event\":\"error\"}".to_string())
}

fn safe_string_vec(values: &[String]) -> Vec<String> {
    values.iter().map(|value| safe_text(value)).collect()
}

fn safe_rows(rows: &[(String, String)]) -> Vec<(String, String)> {
    rows.iter()
        .map(|(key, value)| (safe_text(key), safe_text(value)))
        .collect()
}

fn safe_table_rows(rows: &[Vec<String>]) -> Vec<Vec<String>> {
    rows.iter().map(|row| safe_string_vec(row)).collect()
}

fn safe_diagnostic_json(diagnostic: &Diagnostic) -> serde_json::Value {
    json!({
        "severity": diagnostic.severity,
        "code": diagnostic.code.as_ref().map(|value| safe_text(value)),
        "message": safe_text(&diagnostic.message),
        "path": diagnostic.path.as_ref().map(|value| safe_text(value)),
        "line": diagnostic.line,
        "column": diagnostic.column,
        "help": diagnostic.help.as_ref().map(|value| safe_text(value)),
    })
}

fn safe_warning_json(warning: &WarningDiagnostic) -> serde_json::Value {
    json!({
        "step": safe_text(&warning.step),
        "message": safe_text(&warning.message),
        "path": warning.path.as_ref().map(|value| safe_text(value)),
        "line": warning.line,
        "column": warning.column,
        "source": warning.source,
        "raw_lines": warning.raw_lines.iter().map(|value| safe_text(value)).collect::<Vec<_>>(),
    })
}

fn safe_step_summary(summary: &StepSummary) -> serde_json::Value {
    json!({
        "name": safe_text(&summary.name),
        "status": summary.status,
        "warning_count": summary.warning_count,
    })
}

fn safe_run_summary(summary: &RunSummary) -> serde_json::Value {
    json!({
        "name": safe_text(&summary.name),
        "status": summary.status,
        "step_count": summary.step_count,
        "warning_count": summary.warning_count,
    })
}

fn symbols_for(config: &TerminalConfig) -> StatusSymbols {
    match config.symbols {
        SymbolMode::Unicode => StatusSymbols::unicode(),
        SymbolMode::Ascii => StatusSymbols::ascii(),
    }
}

fn paint(config: &TerminalConfig, value: &str, ansi_code: &str) -> String {
    if config.use_ansi() {
        format!("\u{1b}[{ansi_code}m{value}\u{1b}[0m")
    } else {
        value.to_string()
    }
}

pub fn display_width(value: &str) -> usize {
    value
        .chars()
        .map(|ch| if ch as u32 >= 0x1100 { 2 } else { 1 })
        .sum()
}

fn pad_display(value: &str, target_width: usize) -> String {
    let width = display_width(value);
    if width >= target_width {
        value.to_string()
    } else {
        format!("{}{}", value, " ".repeat(target_width - width))
    }
}

fn normalize_warning_message(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_mode_auto_detects_plain_under_ci() {
        let env = TerminalEnv::from_pairs([("CI", "1")]);
        let config =
            TerminalConfig::from_env_and_capabilities(&env, TerminalCapabilities { is_tty: true });

        assert_eq!(config.mode, TerminalMode::Plain);
    }

    #[test]
    fn no_color_disables_ansi() {
        let env = TerminalEnv::from_pairs([("AICORE_TERMINAL", "rich"), ("NO_COLOR", "1")]);
        let config =
            TerminalConfig::from_env_and_capabilities(&env, TerminalCapabilities { is_tty: true });

        assert_eq!(config.color, ColorMode::Never);
        assert!(!config.use_ansi());
    }

    #[test]
    fn color_always_emits_ansi_in_rich_mode() {
        let env =
            TerminalEnv::from_pairs([("AICORE_TERMINAL", "rich"), ("AICORE_COLOR", "always")]);
        let config =
            TerminalConfig::from_env_and_capabilities(&env, TerminalCapabilities { is_tty: false });
        let document = Document::new(vec![Block::step_finished(StepSummary::new(
            "cargo test",
            Status::Ok,
            0,
        ))]);

        let output = render_document(&document, &config);

        assert!(output.contains("\u{1b}[32m"));
    }

    #[test]
    fn unicode_and_ascii_symbols_render() {
        assert_eq!(StatusSymbols::unicode().ok, "✓");
        assert_eq!(StatusSymbols::ascii().ok, "[OK]");
    }

    #[test]
    fn panel_renderer_supports_rich_and_plain() {
        let document = Document::new(vec![Block::panel("AICore OS", "Composable")]);

        let rich = render_document(&document, &TerminalConfig::rich_for_tests());
        let plain = render_document(&document, &TerminalConfig::plain_for_tests());

        assert!(rich.contains("╭─ AICore OS"));
        assert!(plain.contains("AICore OS"));
        assert!(!plain.contains('╭'));
    }

    #[test]
    fn table_renderer_handles_mixed_chinese_and_english_width() {
        let document = Document::new(vec![Block::table(
            vec!["名称", "Status"],
            vec![vec!["内核", "OK"], vec!["provider", "WARN"]],
        )]);

        let output = render_document(&document, &TerminalConfig::plain_for_tests());

        assert!(output.contains("名称"));
        assert!(output.contains("provider"));
        assert!(output.contains("WARN"));
    }

    #[test]
    fn json_renderer_pretty_prints_json_and_reports_invalid_json() {
        let valid = Document::new(vec![Block::json(r#"{"b":1,"a":true}"#)]);
        let invalid = Document::new(vec![Block::json("{bad json")]);

        let valid_output = render_document(&valid, &TerminalConfig::plain_for_tests());
        let invalid_output = render_document(&invalid, &TerminalConfig::plain_for_tests());

        assert!(valid_output.contains("\n  \"a\": true"));
        assert!(invalid_output.contains("无效 JSON"));
    }

    #[test]
    fn toml_and_markdown_blocks_preserve_content() {
        let document = Document::new(vec![
            Block::toml("[section]\nkey = \"value\""),
            Block::markdown("# 标题\n\n```bash\ncargo core\n```"),
        ]);

        let output = render_document(&document, &TerminalConfig::plain_for_tests());

        assert!(output.contains("[section]"));
        assert!(output.contains("```bash"));
    }

    #[test]
    fn diagnostic_and_warning_summary_render() {
        let warning = WarningDiagnostic::new("cargo test", "unused variable").with_location(
            "src/lib.rs",
            10,
            5,
        );
        let document = Document::new(vec![
            Block::diagnostic(Diagnostic::warning("W0001", "warning message")),
            Block::warning_summary(vec![warning.clone()], 1),
            Block::final_summary(RunSummary::new("kernel", Status::Warn, 2, 1)),
        ]);

        let output = render_document(&document, &TerminalConfig::plain_for_tests());

        assert!(output.contains("warning message"));
        assert!(output.contains("unused variable"));
        assert!(output.contains("Warnings 1"));
    }

    #[test]
    fn sanitizer_and_redaction_strip_unsafe_output() {
        let unsafe_text = "token sk-test-secret \u{1b}[31mred\u{7}";
        let document = Document::new(vec![Block::text(unsafe_text)]);

        let output = render_document(&document, &TerminalConfig::plain_for_tests());

        assert!(!output.contains("sk-test-secret"));
        assert!(!output.contains("\u{1b}"));
        assert!(!output.contains('\u{7}'));
        assert!(output.contains("[REDACTED]"));
    }

    #[test]
    fn json_mode_emits_json_lines_events() {
        let document = Document::new(vec![
            Block::run_started("kernel"),
            Block::run_finished(RunSummary::new("kernel", Status::Ok, 2, 0)),
        ]);

        let output = render_document(&document, &TerminalConfig::json_for_tests());

        for line in output.lines() {
            let value: serde_json::Value = serde_json::from_str(line).expect("valid json line");
            assert_eq!(value["schema"], "aicore.terminal.v1");
        }
    }

    #[test]
    fn json_mode_redacts_warning_payload() {
        let warning = WarningDiagnostic::new("cargo test", "leaked sk-test-secret");
        let document = Document::new(vec![Block::warning(warning)]);

        let output = render_document(&document, &TerminalConfig::json_for_tests());

        assert!(!output.contains("sk-test-secret"));
        assert!(output.contains("[REDACTED]"));
    }
}
