use crate::state::TuiModel;

use super::blocks::render_block;
use super::width::{fit, truncate_display};

pub const WIDTH: usize = 112;
const CONTENT: usize = WIDTH - 4;

pub fn render_snapshot(model: &TuiModel) -> String {
    render_frame(model, true)
}

pub fn render_live_view(model: &TuiModel) -> String {
    render_frame(model, false)
}

pub fn input_box_prompt() -> String {
    format!("│{}│", fit(" aicore > ", WIDTH - 2))
}

pub fn input_box_bottom() -> String {
    format!("╰{}╯", "─".repeat(WIDTH - 2))
}

fn render_frame(model: &TuiModel, include_composer: bool) -> String {
    let mut out = String::new();
    push_line(&mut out, &top_bar(model));
    push_line(&mut out, &status_strip(model));
    push_line(&mut out, &instance_line(model));
    push_line(&mut out, &rule("会话"));
    push_line(&mut out, "");

    for block in &model.blocks {
        for line in render_block(block, CONTENT) {
            push_line(&mut out, &format!("  {line}"));
        }
        push_line(&mut out, "");
    }

    push_line(&mut out, &boundary_line());
    if include_composer {
        for line in composer_snapshot("继续") {
            push_line(&mut out, &line);
        }
    } else {
        push_line(&mut out, &input_box_top());
    }
    out
}

fn top_bar(model: &TuiModel) -> String {
    fit(
        &format!(
            " AICore OS  当前实例 {}  {}  回合 {}  本地显示 ",
            model.instance_id,
            model.instance_kind_label(),
            model.active_turn
        ),
        WIDTH,
    )
}

fn status_strip(model: &TuiModel) -> String {
    fit(
        &format!(
            "  本地显示  不启动智能体运行时  工具 {}  记忆 {}  技能 {}  提案 {}",
            model.tools_count, model.memories_count, model.skills_count, model.proposals_count
        ),
        WIDTH,
    )
}

fn instance_line(model: &TuiModel) -> String {
    let workspace = compact_path(&model.workspace_root, 32);
    let state = compact_path(&model.state_root, 32);
    fit(
        &format!(
            "  工作区 {}  状态目录 {}  会话 {}",
            workspace, state, model.conversation_id
        ),
        WIDTH,
    )
}

fn boundary_line() -> String {
    fit(
        "  边界：TUI 只绑定当前实例用于显示与输入；不启动智能体运行时，不执行工具，不写入记忆。",
        WIDTH,
    )
}

fn composer_snapshot(input: &str) -> Vec<String> {
    vec![
        input_box_top(),
        format!("│{}│", fit(&format!(" aicore > {input}_"), WIDTH - 2)),
        input_box_hint(),
        input_box_bottom(),
    ]
}

fn input_box_top() -> String {
    let top_prefix = "╭─ 输入 ";
    let top_suffix = "╮";
    format!(
        "{top_prefix}{}{top_suffix}",
        "─".repeat(
            WIDTH
                .saturating_sub(super::width::display_width(top_prefix))
                .saturating_sub(super::width::display_width(top_suffix))
        )
    )
}

fn input_box_hint() -> String {
    format!(
        "│{}│",
        fit(
            " Enter 提交到本地会话流 · q 退出 · 后续接入统一 I/O 后同步到同一 instance",
            WIDTH - 2
        )
    )
}

fn rule(title: &str) -> String {
    let prefix = format!("╾─ {title} ");
    format!(
        "{prefix}{}",
        "─".repeat(WIDTH.saturating_sub(super::width::display_width(&prefix)))
    )
}

fn push_line(out: &mut String, value: &str) {
    out.push_str(&fit(value, WIDTH));
    out.push('\n');
}

fn compact_path(value: &str, width: usize) -> String {
    if super::width::display_width(value) <= width {
        return value.to_string();
    }
    let reversed: String = value.chars().rev().collect();
    let tail: String = truncate_display(&reversed, width.saturating_sub(1))
        .chars()
        .rev()
        .collect();
    format!("…{tail}")
}
