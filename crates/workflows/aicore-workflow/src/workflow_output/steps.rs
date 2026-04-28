use aicore_terminal::{TerminalConfig, safe_text};

use super::format::{RICH_PANEL_WIDTH, dim, pad_visible, visible_width};
use super::header::status_text;
use super::panels::render_panel;

pub fn render_workflow_steps(
    steps: &[super::WorkflowStepRecord],
    config: &TerminalConfig,
) -> String {
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
                super::format::format_duration(step.duration),
            ]
        })
        .collect::<Vec<_>>();
    let table = render_table(&headers, &rows, config);
    render_panel("Workflow Steps", &table, config)
}

fn render_table(headers: &[&str], rows: &[Vec<String>], config: &TerminalConfig) -> String {
    let mut widths = headers
        .iter()
        .map(|header| super::format::terminal_width(header))
        .collect::<Vec<_>>();
    for row in rows {
        for (index, cell) in row.iter().enumerate() {
            widths[index] = widths[index].max(visible_width(cell));
        }
    }
    if config.mode == aicore_terminal::TerminalMode::Rich {
        let current_width = widths.iter().sum::<usize>() + widths.len().saturating_sub(1) * 2;
        let target_width = RICH_PANEL_WIDTH.saturating_sub(2);
        if current_width < target_width {
            if let Some(last_width) = widths.last_mut() {
                *last_width += target_width - current_width;
            }
        }
    }

    let header_line = render_table_row(
        &headers
            .iter()
            .map(|value| table_header(value, config))
            .collect::<Vec<_>>(),
        &widths,
    );
    let separator = if config.mode == aicore_terminal::TerminalMode::Rich {
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
        if config.mode == aicore_terminal::TerminalMode::Rich && index + 1 < rows.len() {
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

fn table_header(value: &str, config: &TerminalConfig) -> String {
    if config.mode == aicore_terminal::TerminalMode::Rich {
        super::format::label_style(value, config)
    } else {
        safe_text(value)
    }
}

fn row_number(value: usize, config: &TerminalConfig) -> String {
    let text = value.to_string();
    if config.mode == aicore_terminal::TerminalMode::Rich {
        super::format::accent(&text, config)
    } else {
        text
    }
}
