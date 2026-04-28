use aicore_terminal::{Status, StatusSymbols, SymbolMode, TerminalConfig, TerminalMode, safe_text};

use super::format::{
    ANSI_CYAN, ANSI_DIM, ANSI_GREEN, ANSI_RED, ANSI_RESET, ANSI_YELLOW, accent, border, dim, icon,
    label_style, pad_visible, render_inline_pair, result_text, terminal_mode_label, visible_width,
    warning_policy_label,
};
use super::panels::render_panel;

pub fn render_run_started(
    workflow_id: &str,
    repo_root: &str,
    target: &str,
    config: &TerminalConfig,
) -> String {
    if config.mode == TerminalMode::Json {
        return aicore_terminal::render_document(
            &aicore_terminal::Document::new(vec![aicore_terminal::Block::run_started(workflow_id)]),
            config,
        );
    }

    render_header_panel(workflow_id, repo_root, target, config)
}

pub fn status_text(status: Status, config: &TerminalConfig) -> String {
    let symbols = match config.symbols {
        SymbolMode::Unicode => StatusSymbols::unicode(),
        SymbolMode::Ascii => StatusSymbols::ascii(),
    };
    let symbol = match status {
        Status::Ok => symbols.ok,
        Status::Warn
            if config.mode == TerminalMode::Rich && config.symbols == SymbolMode::Unicode =>
        {
            "!"
        }
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
    let inner_width = body_width.max(super::format::RICH_PANEL_WIDTH);
    let top_border = border(&"─".repeat(inner_width), config);

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
    let warnings = render_rich_meta_pair("◇", "Warnings", warning_policy_label(config), config);
    let root = render_rich_meta_pair("□", "Root", repo_root, config);

    format!(
        "{brand}\n\n{}  {}\n{}  {}\n{}",
        pad_visible(&workflow, 35),
        mode,
        pad_visible(&target, 35),
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

pub fn render_section_title(title: &str, config: &TerminalConfig) -> String {
    let title = safe_text(title);
    let icon_value = match title.as_str() {
        "Workflow Steps" => ">",
        "Summary" => "=",
        "Warnings" => "!",
        _ => "*",
    };
    format!(
        "{} {}",
        accent(icon(icon_value, config), config),
        accent(&title, config)
    )
}

pub fn render_rich_meta_pair(
    icon_value: &str,
    key: &str,
    value: &str,
    config: &TerminalConfig,
) -> String {
    let label = format!("{:<8}", safe_text(key));
    format!(
        "{} {} : {}",
        accent(icon(icon_value, config), config),
        label_style(&label, config),
        safe_text(value)
    )
}

pub fn render_finished(
    workflow_id: &str,
    status: Status,
    steps: &[super::WorkflowStepRecord],
    warning_count: usize,
    duration: std::time::Duration,
    config: &TerminalConfig,
) -> String {
    if config.mode == TerminalMode::Json {
        return aicore_terminal::render_document(
            &aicore_terminal::Document::new(vec![aicore_terminal::Block::run_finished(
                aicore_terminal::RunSummary::new(workflow_id, status, steps.len(), warning_count),
            )]),
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
        ("Duration", super::format::format_duration(duration)),
        ("Result", result_text(status, config)),
    ];
    let body = if config.mode == TerminalMode::Rich {
        super::panels::render_colon_rows(&rows, config)
    } else {
        super::panels::render_key_rows(&rows)
    };
    render_panel("Summary", &body, config)
}
