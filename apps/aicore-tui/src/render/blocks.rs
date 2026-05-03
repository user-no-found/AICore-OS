use crate::state::TuiBlock;

use super::theme::style_for;
use super::width::{fit, wrap};

pub fn render_block(block: &TuiBlock, width: usize) -> Vec<String> {
    let style = style_for(&block.kind);
    let action = match block.kind {
        crate::state::TuiBlockKind::Code => " [复制] [保存] [展开] ",
        crate::state::TuiBlockKind::Diff => " [复制] [查看] ",
        crate::state::TuiBlockKind::Media => " [预览] [打开] ",
        _ => "",
    };
    let title = format!(
        "╭─ {} {} / {}{} ",
        style.marker, style.label, block.title, action
    );
    let mut lines = vec![border_title(&title, width)];

    for body in &block.body {
        for row in wrap(body, width.saturating_sub(6)) {
            lines.push(fit(&format!("│  {row}"), width));
        }
    }

    lines.push(format!("╰{}╯", "─".repeat(width.saturating_sub(2))));
    lines
}

fn border_title(title: &str, width: usize) -> String {
    let suffix = "╮";
    let title = fit(title, width.saturating_sub(1));
    let used = super::width::display_width(&title) + super::width::display_width(suffix);
    if used >= width {
        format!("{}{}", fit(&title, width.saturating_sub(1)), suffix)
    } else {
        format!("{title}{}{suffix}", "─".repeat(width - used))
    }
}
