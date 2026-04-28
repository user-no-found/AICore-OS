use aicore_terminal::{TerminalConfig, TerminalMode, safe_text};

use super::format::{RICH_PANEL_MAX_WIDTH, RICH_PANEL_WIDTH, border, label_style, visible_width};
use super::header::render_section_title;

pub fn render_key_rows(rows: &[(&str, String)]) -> String {
    let key_width = rows
        .iter()
        .filter(|(_, value)| !value.is_empty())
        .map(|(key, _)| super::format::terminal_width(key))
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
                    " ".repeat(key_width.saturating_sub(super::format::terminal_width(key))),
                    value
                )
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn render_colon_rows(rows: &[(&str, String)], config: &TerminalConfig) -> String {
    let key_width = rows
        .iter()
        .filter(|(_, value)| !value.is_empty())
        .map(|(key, _)| super::format::terminal_width(key))
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
                    " ".repeat(key_width.saturating_sub(super::format::terminal_width(key)))
                );
                format!("{} : {}", label_style(&label, config), value)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn render_panel(title: &str, body: &str, config: &TerminalConfig) -> String {
    if config.mode == TerminalMode::Rich {
        return render_rich_panel(title, body, config);
    }

    format!("{}\n{}\n", safe_text(title), safe_text(body))
}

fn render_rich_panel(title: &str, body: &str, config: &TerminalConfig) -> String {
    let title = render_section_title(title, config);
    let inner_width = RICH_PANEL_WIDTH
        .max(visible_width(&title) + 4)
        .max(RICH_PANEL_WIDTH)
        .min(RICH_PANEL_MAX_WIDTH);
    let lines = wrap_visible_body_lines(body, inner_width.saturating_sub(2));
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
            " ".repeat(inner_width.saturating_sub(visible_width(&line) + 2)),
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

fn wrap_visible_body_lines(body: &str, max_width: usize) -> Vec<String> {
    let source_lines = body.lines().collect::<Vec<_>>();
    if source_lines.is_empty() {
        return vec![String::new()];
    }

    source_lines
        .into_iter()
        .flat_map(|line| wrap_visible_line(line, max_width))
        .collect()
}

fn wrap_visible_line(line: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 || visible_width(line) <= max_width {
        return vec![line.to_string()];
    }

    let mut lines = Vec::new();
    let mut current = String::new();
    let mut width = 0usize;
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' && chars.peek() == Some(&'[') {
            current.push(ch);
            current.push(chars.next().expect("peeked ansi introducer"));
            for next in chars.by_ref() {
                current.push(next);
                if next == 'm' {
                    break;
                }
            }
            continue;
        }

        let ch_width = super::format::char_width(ch);
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
