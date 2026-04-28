use std::time::Duration;

use aicore_terminal::{SymbolMode, TerminalConfig, TerminalMode, safe_text};

pub const RICH_PANEL_WIDTH: usize = 58;
pub const RICH_PANEL_MAX_WIDTH: usize = 78;
pub const ANSI_RESET: &str = "\u{1b}[0m";
pub const ANSI_DIM: &str = "\u{1b}[2m";
pub const ANSI_LABEL: &str = "\u{1b}[38;2;167;139;250m";
pub const ANSI_CYAN: &str = "\u{1b}[96m";
pub const ANSI_GREEN: &str = "\u{1b}[32m";
pub const ANSI_YELLOW: &str = "\u{1b}[33m";
pub const ANSI_RED: &str = "\u{1b}[31m";

pub fn terminal_mode_label(mode: TerminalMode) -> &'static str {
    match mode {
        TerminalMode::Rich => "rich",
        TerminalMode::Plain => "plain",
        TerminalMode::Json => "json",
    }
}

pub fn warning_policy_label(config: &TerminalConfig) -> &'static str {
    if config.deny_warnings {
        "deny"
    } else {
        "report"
    }
}

pub fn result_label(status: aicore_terminal::Status) -> &'static str {
    match status {
        aicore_terminal::Status::Ok => "workflow completed successfully",
        aicore_terminal::Status::Warn => "workflow completed with warnings",
        aicore_terminal::Status::Failed => "workflow failed",
        aicore_terminal::Status::Running => "workflow running",
        aicore_terminal::Status::Info => "workflow reported information",
        aicore_terminal::Status::Skipped => "workflow skipped",
    }
}

pub fn result_text(status: aicore_terminal::Status, config: &TerminalConfig) -> String {
    let label = result_label(status);
    if !config.use_ansi() {
        return label.to_string();
    }

    match status {
        aicore_terminal::Status::Ok => success(label, config),
        aicore_terminal::Status::Warn => warning(label, config),
        aicore_terminal::Status::Failed => failure(label, config),
        aicore_terminal::Status::Running | aicore_terminal::Status::Info => accent(label, config),
        aicore_terminal::Status::Skipped => dim(label, config),
    }
}

pub fn format_duration(duration: Duration) -> String {
    format!("{:.2}s", duration.as_secs_f64())
}

pub fn visible_width(value: &str) -> usize {
    terminal_width(&strip_ansi(value))
}

pub fn terminal_width(value: &str) -> usize {
    value.chars().map(char_width).sum()
}

pub fn char_width(ch: char) -> usize {
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

pub fn pad_visible(value: &str, width: usize) -> String {
    let visible = visible_width(value);
    if visible >= width {
        value.to_string()
    } else {
        format!("{}{}", value, " ".repeat(width - visible))
    }
}

pub fn strip_ansi(value: &str) -> String {
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

pub fn render_inline_pair(key: &str, value: &str) -> String {
    format!(
        "{key:<10}{value}",
        key = safe_text(key),
        value = safe_text(value)
    )
}

pub fn icon<'a>(unicode: &'a str, config: &TerminalConfig) -> &'a str {
    match config.symbols {
        SymbolMode::Unicode => unicode,
        SymbolMode::Ascii => "*",
    }
}

pub fn border(value: &str, config: &TerminalConfig) -> String {
    dim(value, config)
}

pub fn label_style(value: &str, config: &TerminalConfig) -> String {
    style(value, ANSI_LABEL, config)
}

pub fn accent(value: &str, config: &TerminalConfig) -> String {
    style(value, ANSI_CYAN, config)
}

pub fn success(value: &str, config: &TerminalConfig) -> String {
    style(value, ANSI_GREEN, config)
}

pub fn warning(value: &str, config: &TerminalConfig) -> String {
    style(value, ANSI_YELLOW, config)
}

pub fn failure(value: &str, config: &TerminalConfig) -> String {
    style(value, ANSI_RED, config)
}

pub fn dim(value: &str, config: &TerminalConfig) -> String {
    style(value, ANSI_DIM, config)
}

pub fn style(value: &str, code: &str, config: &TerminalConfig) -> String {
    if config.use_ansi() {
        format!("{code}{value}{ANSI_RESET}")
    } else {
        safe_text(value)
    }
}
