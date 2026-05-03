use crate::state::{TuiBlock, TuiBlockKind, TuiModel};

const WIDTH: usize = 112;
const SIDE: usize = 30;

pub fn render_snapshot(model: &TuiModel) -> String {
    let mut out = String::new();
    out.push_str(&top_bar(model));
    out.push('\n');
    out.push_str(&rule());
    out.push('\n');
    for line in compose_rows(model, WIDTH) {
        out.push_str(&line);
        out.push('\n');
    }
    out.push_str(&rule());
    out.push('\n');
    out.push_str(&composer("继续"));
    out.push('\n');
    out
}

pub fn render_transcript(model: &TuiModel, input: &str) -> String {
    let mut model = model.clone();
    append_local_echo(&mut model, input);
    render_snapshot(&model)
}

pub fn append_local_echo(model: &mut TuiModel, input: &str) {
    model.blocks.push(TuiBlock {
        kind: TuiBlockKind::Prompt,
        title: "用户输入".to_string(),
        body: vec![input.to_string()],
    });
    model.blocks.push(TuiBlock {
        kind: TuiBlockKind::Assistant,
        title: "本地回显".to_string(),
        body: vec!["TUI 已接收输入；当前版本只写入本地显示，不启动智能体运行时。".to_string()],
    });
}

fn top_bar(model: &TuiModel) -> String {
    fit(
        &format!(
            " AICore OS  实例:{}  类型:{}  回合:{}  工具:{}  记忆:{} ",
            model.instance_id,
            model.instance_kind_label(),
            model.active_turn,
            model.tools_count,
            model.memories_count
        ),
        WIDTH,
    )
}

fn compose_rows(model: &TuiModel, width: usize) -> Vec<String> {
    let stream_width = width.saturating_sub(SIDE + 3);
    let stream = stream_panel(model, stream_width);
    let side = side_panel(model);
    let rows = stream.len().max(side.len());
    (0..rows)
        .map(|index| {
            format!(
                "{}   {}",
                cell(stream.get(index), stream_width),
                cell(side.get(index), SIDE)
            )
        })
        .collect()
}

fn stream_panel(model: &TuiModel, width: usize) -> Vec<String> {
    let mut rows = vec![
        section("会话流", width),
        "当前 TUI 绑定到本目录 instance，用于显示与输入。".to_string(),
        "统一 I/O 接入前，输入只在本地会话流中回显。".to_string(),
        String::new(),
    ];
    for block in &model.blocks {
        rows.push(block_header(block));
        for body in &block.body {
            rows.extend(wrap(&format!("  {body}"), width));
        }
        rows.push(String::new());
    }
    rows
}

fn side_panel(model: &TuiModel) -> Vec<String> {
    vec![
        section("实例", SIDE),
        model.instance_id.clone(),
        model.instance_kind_label().to_string(),
        String::new(),
        section("路径", SIDE),
        label_value("工作区", &model.workspace_root),
        label_value("状态目录", &model.state_root),
        String::new(),
        section("运行时", SIDE),
        label_value("会话", &model.conversation_id),
        label_value("事件", &model.event_count.to_string()),
        label_value("队列", &model.queue_len.to_string()),
        label_value("回合", &model.active_turn),
        String::new(),
        section("能力", SIDE),
        label_value("工具", &model.tools_count.to_string()),
        label_value("记忆", &model.memories_count.to_string()),
        label_value("技能", &model.skills_count.to_string()),
        label_value("提案", &model.proposals_count.to_string()),
        String::new(),
        section("边界", SIDE),
        "不启动智能体运行时".to_string(),
        "不执行工具".to_string(),
        "不写入记忆".to_string(),
    ]
}

fn block_header(block: &TuiBlock) -> String {
    let marker = match block.kind {
        TuiBlockKind::Prompt => ">",
        TuiBlockKind::Agent => "◇",
        TuiBlockKind::Tool => "$",
        TuiBlockKind::Approval => "!",
        TuiBlockKind::Terminal => "$",
        TuiBlockKind::Assistant => "◆",
    };
    format!("{marker} {}", block.title)
}

fn label_value(label: &str, value: &str) -> String {
    format!("{label}: {value}")
}

fn composer(input: &str) -> String {
    fit(&format!(" aicore > {input}_"), WIDTH)
}

fn section(title: &str, width: usize) -> String {
    let prefix = format!("─ {title} ");
    format!(
        "{prefix}{}",
        "─".repeat(width.saturating_sub(display_width(&prefix)))
    )
}

fn rule() -> String {
    "─".repeat(WIDTH)
}

fn cell(value: Option<&String>, width: usize) -> String {
    fit(value.map(String::as_str).unwrap_or_default(), width)
}

fn fit(value: &str, width: usize) -> String {
    let value = value.replace('\n', " ");
    let mut text = if display_width(&value) > width {
        let mut text = truncate_display(&value, width.saturating_sub(1));
        text.push('…');
        text
    } else {
        value
    };
    text = truncate_display(&text, width);
    let padding = width.saturating_sub(display_width(&text));
    format!("{text}{}", " ".repeat(padding))
}

fn wrap(value: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![String::new()];
    }
    let mut rows = Vec::new();
    let mut current = String::new();
    for ch in value.chars() {
        if display_width(&current) + char_width(ch) > width {
            rows.push(current);
            current = String::new();
        }
        current.push(ch);
    }
    if current.is_empty() {
        rows.push(String::new());
    } else {
        rows.push(current);
    }
    rows
}

fn truncate_display(value: &str, width: usize) -> String {
    let mut out = String::new();
    let mut used = 0;
    for ch in value.chars() {
        let char_width = char_width(ch);
        if used + char_width > width {
            break;
        }
        out.push(ch);
        used += char_width;
    }
    out
}

fn display_width(value: &str) -> usize {
    value.chars().map(char_width).sum()
}

fn char_width(ch: char) -> usize {
    let value = ch as u32;
    if (0x1100..=0x115f).contains(&value)
        || (0x2e80..=0xa4cf).contains(&value)
        || (0xac00..=0xd7a3).contains(&value)
        || (0xf900..=0xfaff).contains(&value)
        || (0xfe10..=0xfe19).contains(&value)
        || (0xfe30..=0xfe6f).contains(&value)
        || (0xff00..=0xff60).contains(&value)
        || (0xffe0..=0xffe6).contains(&value)
    {
        2
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::{build_tui_model, render_snapshot, render_transcript};

    #[test]
    fn snapshot_contains_tui_regions() {
        let home = TestDir::new("home");
        let workspace = TestDir::new("workspace");
        let model = build_tui_model(workspace.path(), home.path()).unwrap();
        let rendered = render_snapshot(&model);

        assert!(rendered.contains("AICore OS"));
        assert!(rendered.contains("实例"));
        assert!(rendered.contains("会话流"));
        assert!(rendered.contains("运行时"));
        assert!(rendered.contains("aicore >"));
    }

    #[test]
    fn transcript_echoes_local_input_without_agent_claim() {
        let home = TestDir::new("home");
        let workspace = TestDir::new("workspace");
        let model = build_tui_model(workspace.path(), home.path()).unwrap();
        let rendered = render_transcript(&model, "测试输入");

        assert!(rendered.contains("测试输入"));
        assert!(rendered.contains("不启动智能体运行时"));
    }

    #[test]
    fn snapshot_lines_have_stable_display_width() {
        let home = TestDir::new("home");
        let workspace = TestDir::new("workspace");
        let model = build_tui_model(workspace.path(), home.path()).unwrap();
        let rendered = render_snapshot(&model);

        for line in rendered.lines() {
            assert!(super::display_width(line) <= super::WIDTH);
        }
    }

    struct TestDir {
        path: PathBuf,
    }

    impl TestDir {
        fn new(name: &str) -> Self {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time should be available")
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "aicore-tui-render-{name}-{}-{unique}",
                std::process::id()
            ));
            std::fs::create_dir_all(&path).expect("test dir should create");
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }
}
