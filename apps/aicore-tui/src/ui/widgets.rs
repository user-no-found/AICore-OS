use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem, Paragraph, Wrap};

use crate::ui::app_state::AicoreTuiApp;
use crate::ui::theme::{
    approval_style, assistant_style, code_style, focus_style, media_style, normal_style,
    prompt_style, surface_style, terminal_style, title_style, tool_style,
};

pub fn render_top_bar(frame: &mut ratatui::Frame, app: &AicoreTuiApp, area: Rect) {
    let model = &app.model;
    let title = Line::from(vec![
        Span::styled(" AICore OS ", title_style().add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::styled(&model.instance_id, assistant_style()),
        Span::raw("  "),
        Span::styled(model.instance_kind_label(), prompt_style()),
        Span::raw("  "),
        Span::styled("终端富内容显示器", tool_style()),
    ]);
    let status = Line::from(vec![
        Span::raw("能力 "),
        Span::styled(
            format!("mouse:{}", yes_no(model.terminal_capabilities.mouse)),
            tool_style(),
        ),
        Span::raw("  copy:"),
        Span::styled(&model.terminal_capabilities.clipboard, tool_style()),
        Span::raw("  media:"),
        Span::styled(&model.terminal_capabilities.inline_image, media_style()),
        Span::raw("  mux:"),
        Span::styled(&model.terminal_capabilities.multiplexer, code_style()),
    ])
    .right_aligned();
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(assistant_style())
        .title(title)
        .title(status)
        .style(surface_style());
    frame.render_widget(block, area);
}

pub fn render_left_pane(frame: &mut ratatui::Frame, app: &AicoreTuiApp, area: Rect) {
    let model = &app.model;
    let lines = vec![
        Line::from(vec![Span::styled("实例", title_style())]),
        Line::from(model.instance_kind_label()),
        Line::from(model.instance_id.clone()),
        Line::from(""),
        Line::from(vec![Span::styled("工作区", title_style())]),
        Line::from(compact(&model.workspace_root, 28)),
        Line::from(""),
        Line::from(vec![Span::styled("状态目录", title_style())]),
        Line::from(compact(&model.state_root, 28)),
        Line::from(""),
        Line::from(vec![Span::styled("边界", title_style())]),
        Line::from("TUI 只绑定当前实例"),
        Line::from("不执行工具 / 不写记忆"),
        Line::from("不启动智能体运行时"),
    ];
    let paragraph = Paragraph::new(Text::from(lines))
        .block(panel_block("实例 / 边界", app.focus_index == 1))
        .wrap(Wrap { trim: true });
    frame.render_widget(paragraph, area);
}

pub fn render_right_pane(frame: &mut ratatui::Frame, app: &AicoreTuiApp, area: Rect) {
    let model = &app.model;
    let items = vec![
        metric_item("工具 ", model.tools_count, tool_style()),
        metric_item("记忆 ", model.memories_count, assistant_style()),
        metric_item("技能 ", model.skills_count, code_style()),
        metric_item("提案 ", model.proposals_count, approval_style()),
        ListItem::new(""),
        ListItem::new(Line::from(vec![
            Span::styled("审批 ", approval_style()),
            Span::raw("0 待处理"),
        ])),
        ListItem::new(Line::from(vec![
            Span::styled("媒体 ", media_style()),
            Span::raw(&model.terminal_capabilities.inline_image),
        ])),
        ListItem::new(Line::from(vec![
            Span::styled("复制 ", code_style()),
            Span::raw("可见按钮 + 鼠标点击"),
        ])),
        ListItem::new(""),
        ListItem::new(Line::from("视图：Chat / Plan / Diff")),
        ListItem::new(Line::from("后续接 Unified I/O stream")),
    ];
    let list = List::new(items)
        .block(panel_block("运行 / 检查器", app.focus_index == 2))
        .style(normal_style());
    frame.render_widget(list, area);
}

pub fn render_composer(frame: &mut ratatui::Frame, app: &AicoreTuiApp, area: Rect) {
    let text = if app.composer.is_empty() {
        "输入任务，或粘贴文件 / 图片引用；当前只进入本地显示。".to_string()
    } else {
        app.composer.clone()
    };
    let paragraph = Paragraph::new(text)
        .block(
            panel_block("输入", app.focus_index == 3)
                .title(Line::from("[发送]").right_aligned())
                .title(Line::from("[附件] [图片] [引用]")),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

pub fn render_action_bar(frame: &mut ratatui::Frame, app: &AicoreTuiApp, area: Rect) {
    let line = Line::from(vec![
        Span::styled(" Esc/Ctrl+C 退出 ", approval_style()),
        Span::raw(" Enter 本地提交 "),
        Span::raw(" Ctrl+J 换行 "),
        Span::raw(" 鼠标点击 [复制] "),
        Span::styled(format!(" {} ", app.toast), title_style()),
    ]);
    let paragraph = Paragraph::new(line).block(Block::default().borders(Borders::TOP));
    frame.render_widget(paragraph, area);
}

fn metric_item(label: &'static str, value: usize, style: Style) -> ListItem<'static> {
    ListItem::new(Line::from(vec![
        Span::styled(label, style),
        Span::raw(value.to_string()),
    ]))
}

pub fn panel_block(title: &'static str, focused: bool) -> Block<'static> {
    let style = if focused {
        focus_style()
    } else {
        terminal_style()
    };
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(style)
        .title(Line::from(title).alignment(Alignment::Left))
        .style(surface_style())
}

fn compact(value: &str, width: usize) -> String {
    if unicode_width::UnicodeWidthStr::width(value) <= width {
        return value.to_string();
    }
    let mut chars = value.chars().rev();
    let mut out = String::new();
    while unicode_width::UnicodeWidthStr::width(out.as_str()) < width.saturating_sub(1) {
        let Some(ch) = chars.next() else {
            break;
        };
        out.insert(0, ch);
    }
    format!("…{out}")
}

fn yes_no(value: bool) -> &'static str {
    if value { "on" } else { "off" }
}
