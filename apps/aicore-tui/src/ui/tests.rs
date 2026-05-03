#[cfg(test)]
mod contract {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use crate::build_tui_model;
    use crate::state::TuiBlockKind;
    use crate::ui::conversation::action_start_x;
    use crate::ui::{AicoreTuiApp, UiAction, draw};

    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn ratatui_frame_contains_rich_terminal_regions() {
        let home = TestDir::new("home");
        let workspace = TestDir::new("workspace");
        let model = build_tui_model(workspace.path(), home.path()).unwrap();
        let mut app = AicoreTuiApp::new(model);
        let backend = TestBackend::new(150, 42);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        let rendered = buffer_text(terminal.backend().buffer());

        assert!(rendered.contains("AICore OS"));
        assert!(rendered.contains("复 制"));
        assert!(rendered.contains("Diff"));
        assert!(rendered.contains("媒 体"));
        assert!(rendered.contains("不 启 动 智 能 体 运 行 时"));
    }

    #[test]
    fn copy_button_has_mouse_hit_rect() {
        let home = TestDir::new("home");
        let workspace = TestDir::new("workspace");
        let model = build_tui_model(workspace.path(), home.path()).unwrap();
        let mut app = AicoreTuiApp::new(model);
        let backend = TestBackend::new(150, 42);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.draw(|frame| draw(frame, &mut app)).unwrap();

        assert!(
            app.hit_rects
                .iter()
                .any(|hit| matches!(hit.action, UiAction::CopyBlock { .. })),
            "code copy action should be registered for mouse hit-test"
        );
    }

    #[test]
    fn copy_hit_rect_matches_rendered_button_column() {
        let home = TestDir::new("home");
        let workspace = TestDir::new("workspace");
        let model = build_tui_model(workspace.path(), home.path()).unwrap();
        let code_index = model
            .blocks
            .iter()
            .position(|block| block.kind == TuiBlockKind::Code)
            .expect("default model should include code block");
        let expected_x = action_start_x(33, &model.blocks[code_index]);
        let mut app = AicoreTuiApp::new(model);
        let backend = TestBackend::new(150, 42);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.draw(|frame| draw(frame, &mut app)).unwrap();

        let copy_hit = app
            .hit_rects
            .iter()
            .find(|hit| matches!(hit.action, UiAction::CopyBlock { block_index } if block_index == code_index))
            .expect("copy hit rect should exist for code block");

        assert_eq!(copy_hit.area.x, expected_x);
        assert!(copy_hit.area.width >= 6);
    }

    #[test]
    fn media_buttons_have_preview_and_open_hit_rects() {
        let home = TestDir::new("home");
        let workspace = TestDir::new("workspace");
        let model = build_tui_model(workspace.path(), home.path()).unwrap();
        let mut app = AicoreTuiApp::new(model);
        let backend = TestBackend::new(150, 42);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.draw(|frame| draw(frame, &mut app)).unwrap();

        assert!(
            app.hit_rects
                .iter()
                .any(|hit| matches!(hit.action, UiAction::PreviewMedia { .. })),
            "media preview action should be registered for mouse hit-test"
        );
        assert!(
            app.hit_rects
                .iter()
                .any(|hit| matches!(hit.action, UiAction::OpenMedia { .. })),
            "media open action should be registered for mouse hit-test"
        );
    }

    fn buffer_text(buffer: &ratatui::buffer::Buffer) -> String {
        let mut out = String::new();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                if let Some(cell) = buffer.cell((x, y)) {
                    out.push_str(cell.symbol());
                }
            }
            out.push('\n');
        }
        out
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
                "aicore-tui-ui-{name}-{}-{unique}",
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
