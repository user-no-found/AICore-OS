use ratatui::layout::{Margin, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Paragraph, Wrap};
use unicode_width::UnicodeWidthStr;

use crate::state::{TuiBlock, TuiBlockKind};
use crate::ui::app_state::{AicoreTuiApp, UiAction};
use crate::ui::theme::{
    approval_style, assistant_style, code_style, diff_add_style, diff_remove_style, diff_style,
    dim_style, media_style, prompt_style, terminal_style, title_style, tool_style,
};
use crate::ui::widgets::panel_block;

pub fn render_conversation(frame: &mut ratatui::Frame, app: &mut AicoreTuiApp, area: Rect) {
    let inner = area.inner(Margin {
        vertical: 1,
        horizontal: 1,
    });
    frame.render_widget(panel_block("消息流 / 富内容", app.focus_index == 0), area);

    let mut lines = Vec::new();
    for (index, block) in app.model.blocks.iter().enumerate() {
        lines.extend(block_lines(block, index, app.copied_block == Some(index)));
        lines.push(Line::raw(""));
    }
    let paragraph = Paragraph::new(Text::from(lines))
        .scroll((app.scroll, 0))
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, inner);
    register_visible_actions(app, inner);
}

fn block_lines(block: &TuiBlock, index: usize, copied: bool) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let actions = action_label(block, copied);
    lines.push(Line::from(vec![
        Span::styled(
            format!(" {} ", block_marker(&block.kind)),
            block_style(&block.kind).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{} / {}", block_label(&block.kind), block.title),
            title_style(),
        ),
        Span::raw(" "),
        Span::styled(actions, action_style(&block.kind)),
        Span::styled(format!(" #{}", index + 1), dim_style()),
    ]));
    for body in &block.body {
        match block.kind {
            TuiBlockKind::Code => lines.push(Line::from(vec![
                Span::styled("│ ", dim_style()),
                Span::styled(body.clone(), code_style()),
            ])),
            TuiBlockKind::Diff => lines.push(diff_line(body)),
            TuiBlockKind::Media => lines.push(Line::from(vec![
                Span::styled("▣ ", media_style()),
                Span::raw(body.clone()),
            ])),
            _ => lines.push(Line::from(vec![Span::raw("  "), Span::raw(body.clone())])),
        }
    }
    lines
}

fn diff_line(value: &str) -> Line<'static> {
    let style = if value.starts_with('+') {
        diff_add_style()
    } else if value.starts_with('-') {
        diff_remove_style()
    } else {
        diff_style()
    };
    Line::from(vec![Span::styled(value.to_string(), style)])
}

fn register_visible_actions(app: &mut AicoreTuiApp, area: Rect) {
    let mut content_row = 0u16;
    let blocks = app.model.blocks.clone();
    for (index, block) in blocks.iter().enumerate() {
        let Some(row) = visible_row(area, content_row, app.scroll) else {
            content_row = content_row.saturating_add(block.body.len() as u16 + 2);
            continue;
        };
        if row >= area.bottom() {
            break;
        }
        register_block_actions(app, block, index, area.x, row);
        content_row = content_row.saturating_add(block.body.len() as u16 + 2);
    }
}

fn visible_row(area: Rect, content_row: u16, scroll: u16) -> Option<u16> {
    let visible_offset = content_row.checked_sub(scroll)?;
    let row = area.y.saturating_add(visible_offset);
    (row < area.bottom()).then_some(row)
}

fn register_block_actions(
    app: &mut AicoreTuiApp,
    block: &TuiBlock,
    index: usize,
    line_x: u16,
    row: u16,
) {
    let action_x = action_start_x(line_x, block);
    match block.kind {
        TuiBlockKind::Code => {
            app.push_hit(
                Rect::new(action_x, row, 8, 1),
                UiAction::CopyBlock { block_index: index },
            );
            app.push_hit(
                Rect::new(action_x.saturating_add(9), row, 8, 1),
                UiAction::SaveSnippet { block_index: index },
            );
            app.push_hit(
                Rect::new(action_x.saturating_add(18), row, 8, 1),
                UiAction::ExpandBlock { block_index: index },
            );
        }
        TuiBlockKind::Diff => {
            app.push_hit(
                Rect::new(action_x, row, 8, 1),
                UiAction::CopyBlock { block_index: index },
            );
        }
        TuiBlockKind::Media => {
            app.push_hit(
                Rect::new(action_x, row, 8, 1),
                UiAction::PreviewMedia { block_index: index },
            );
            app.push_hit(
                Rect::new(action_x.saturating_add(9), row, 8, 1),
                UiAction::OpenMedia { block_index: index },
            );
        }
        _ => {}
    }
}

pub(crate) fn action_start_x(line_x: u16, block: &TuiBlock) -> u16 {
    line_x.saturating_add(header_prefix_width(block))
}

fn header_prefix_width(block: &TuiBlock) -> u16 {
    let prefix = format!(
        " {} {} / {} ",
        block_marker(&block.kind),
        block_label(&block.kind),
        block.title
    );
    UnicodeWidthStr::width(prefix.as_str()).min(u16::MAX as usize) as u16
}

fn block_style(kind: &TuiBlockKind) -> Style {
    match kind {
        TuiBlockKind::Prompt => prompt_style(),
        TuiBlockKind::Agent => assistant_style(),
        TuiBlockKind::Tool => tool_style(),
        TuiBlockKind::Approval => approval_style(),
        TuiBlockKind::Terminal => terminal_style(),
        TuiBlockKind::Assistant => assistant_style(),
        TuiBlockKind::Code => code_style(),
        TuiBlockKind::Diff => diff_style(),
        TuiBlockKind::Media => media_style(),
    }
}

fn action_style(kind: &TuiBlockKind) -> Style {
    match kind {
        TuiBlockKind::Code => code_style().add_modifier(Modifier::BOLD),
        TuiBlockKind::Diff => diff_style().add_modifier(Modifier::BOLD),
        TuiBlockKind::Media => media_style().add_modifier(Modifier::BOLD),
        _ => dim_style(),
    }
}

fn action_label(block: &TuiBlock, copied: bool) -> &'static str {
    match block.kind {
        TuiBlockKind::Code if copied => "[已复制] [保存] [展开]",
        TuiBlockKind::Code => "[复制] [保存] [展开]",
        TuiBlockKind::Diff => "[复制] [查看]",
        TuiBlockKind::Media => "[预览] [打开]",
        _ => "",
    }
}

fn block_marker(kind: &TuiBlockKind) -> &'static str {
    match kind {
        TuiBlockKind::Prompt => "›",
        TuiBlockKind::Agent => "◆",
        TuiBlockKind::Tool => "$",
        TuiBlockKind::Approval => "!",
        TuiBlockKind::Terminal => "▸",
        TuiBlockKind::Assistant => "●",
        TuiBlockKind::Code => "{}",
        TuiBlockKind::Diff => "±",
        TuiBlockKind::Media => "▣",
    }
}

fn block_label(kind: &TuiBlockKind) -> &'static str {
    match kind {
        TuiBlockKind::Prompt => "用户",
        TuiBlockKind::Agent => "运行",
        TuiBlockKind::Tool => "工具",
        TuiBlockKind::Approval => "审批",
        TuiBlockKind::Terminal => "事件",
        TuiBlockKind::Assistant => "系统",
        TuiBlockKind::Code => "代码",
        TuiBlockKind::Diff => "变更",
        TuiBlockKind::Media => "媒体",
    }
}
