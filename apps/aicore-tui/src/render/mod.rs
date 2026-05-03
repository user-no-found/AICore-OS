mod blocks;
mod frame;
mod theme;
mod width;

use crate::state::{TuiBlock, TuiBlockKind, TuiModel};

pub use frame::{input_box_bottom, input_box_prompt, render_live_view, render_snapshot};

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
        title: "本地显示".to_string(),
        body: vec!["已接收输入；当前仅更新本地会话流，不启动智能体运行时。".to_string()],
    });
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::{build_tui_model, render_snapshot, render_transcript};

    #[test]
    fn snapshot_contains_terminal_product_regions() {
        let home = TestDir::new("home");
        let workspace = TestDir::new("workspace");
        let model = build_tui_model(workspace.path(), home.path()).unwrap();
        let rendered = render_snapshot(&model);

        assert!(rendered.contains("AICore OS"));
        assert!(rendered.contains("当前实例"));
        assert!(rendered.contains("会话"));
        assert!(rendered.contains("实例已绑定"));
        assert!(rendered.contains("能力快照"));
        assert!(rendered.contains("本地显示"));
        assert!(rendered.contains("输入"));
        assert!(rendered.contains("Enter 提交"));
        assert!(rendered.contains("不启动智能体运行时"));
    }

    #[test]
    fn transcript_echoes_local_input_without_runtime_claim() {
        let home = TestDir::new("home");
        let workspace = TestDir::new("workspace");
        let model = build_tui_model(workspace.path(), home.path()).unwrap();
        let rendered = render_transcript(&model, "测试输入");

        assert!(rendered.contains("测试输入"));
        assert!(rendered.contains("已接收输入"));
        assert!(rendered.contains("不启动智能体运行时"));
    }

    #[test]
    fn local_echo_appends_user_and_display_blocks() {
        let home = TestDir::new("home");
        let workspace = TestDir::new("workspace");
        let mut model = build_tui_model(workspace.path(), home.path()).unwrap();
        let original_len = model.blocks.len();

        super::append_local_echo(&mut model, "第一条输入");
        super::append_local_echo(&mut model, "第二条输入");

        assert_eq!(model.blocks.len(), original_len + 4);
        let rendered = render_snapshot(&model);
        assert!(rendered.contains("第一条输入"));
        assert!(rendered.contains("第二条输入"));
        assert!(rendered.contains("本地显示"));
    }

    #[test]
    fn snapshot_lines_have_stable_display_width() {
        let home = TestDir::new("home");
        let workspace = TestDir::new("workspace");
        let model = build_tui_model(workspace.path(), home.path()).unwrap();
        let rendered = render_snapshot(&model);

        for line in rendered.lines() {
            assert!(
                super::width::display_width(line) <= super::frame::WIDTH,
                "line exceeds width: {line}"
            );
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
