use std::io::{self, Write};
use std::time::Duration;

use crossterm::clipboard::CopyToClipboard;
use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
    MouseButton, MouseEventKind,
};
use crossterm::{execute, queue};
use ratatui::DefaultTerminal;
use ratatui::layout::Position;

use crate::render::append_local_echo;
use crate::state::TuiModel;
use crate::ui::{AicoreTuiApp, UiAction};

pub fn run_terminal(model: TuiModel) -> i32 {
    match run_terminal_inner(model) {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("TUI 运行失败：{error}");
            1
        }
    }
}

fn run_terminal_inner(model: TuiModel) -> io::Result<()> {
    let mut terminal = match ratatui::try_init() {
        Ok(terminal) => terminal,
        Err(error) => {
            let _ = ratatui::try_restore();
            return Err(error);
        }
    };
    let _session = TerminalSession::enable_mouse()?;
    run_event_loop(&mut terminal, AicoreTuiApp::new(model))
}

fn run_event_loop(terminal: &mut DefaultTerminal, mut app: AicoreTuiApp) -> io::Result<()> {
    loop {
        terminal.draw(|frame| crate::ui::draw(frame, &mut app))?;
        if app.should_quit {
            return Ok(());
        }
        if !event::poll(Duration::from_millis(250))? {
            continue;
        }
        match event::read()? {
            Event::Key(key)
                if key.kind == KeyEventKind::Press
                    && handle_key(&mut app, key.code, key.modifiers)? =>
            {
                return Ok(());
            }
            Event::Mouse(mouse) => handle_mouse(&mut app, mouse.kind, mouse.column, mouse.row)?,
            Event::Resize(_, _) => app.set_toast("终端尺寸已更新。"),
            _ => {}
        }
    }
}

fn handle_key(app: &mut AicoreTuiApp, code: KeyCode, modifiers: KeyModifiers) -> io::Result<bool> {
    match code {
        KeyCode::Esc => return Ok(true),
        KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => return Ok(true),
        KeyCode::Char('l') if modifiers.contains(KeyModifiers::CONTROL) => {
            app.set_toast("已重绘当前终端界面。");
        }
        KeyCode::Char('j') if modifiers.contains(KeyModifiers::CONTROL) => {
            app.composer.push('\n');
        }
        KeyCode::Enter => {
            let input = app.composer.trim().to_string();
            if !input.is_empty() {
                append_local_echo(&mut app.model, &input);
                app.composer.clear();
                app.set_toast("已加入本地显示；当前未启动智能体运行时。");
            }
        }
        KeyCode::Backspace => _ = app.composer.pop(),
        KeyCode::Char(ch) if modifiers.is_empty() || modifiers == KeyModifiers::SHIFT => {
            app.composer.push(ch);
        }
        KeyCode::Up | KeyCode::PageUp => app.scroll_up(),
        KeyCode::Down | KeyCode::PageDown => app.scroll_down(),
        KeyCode::Tab => app.next_focus(),
        _ => {}
    }
    Ok(false)
}

fn handle_mouse(
    app: &mut AicoreTuiApp,
    kind: MouseEventKind,
    column: u16,
    row: u16,
) -> io::Result<()> {
    match kind {
        MouseEventKind::Down(MouseButton::Left) => {
            if let Some(action) = app.action_at(Position { x: column, y: row }).cloned() {
                run_action(app, action)?;
            }
        }
        MouseEventKind::ScrollUp => app.scroll_up(),
        MouseEventKind::ScrollDown => app.scroll_down(),
        _ => {}
    }
    Ok(())
}

fn run_action(app: &mut AicoreTuiApp, action: UiAction) -> io::Result<()> {
    match action {
        UiAction::CopyBlock { block_index } => {
            let Some(block) = app.model.blocks.get(block_index) else {
                app.set_toast("复制失败：内容块不存在。");
                return Ok(());
            };
            let text = block.body.join("\n");
            let line_count = block.body.len();
            let mut stdout = io::stdout();
            match queue!(stdout, CopyToClipboard::to_clipboard_from(text.as_bytes())) {
                Ok(()) => {
                    stdout.flush()?;
                    app.mark_copied(block_index);
                    app.set_toast(&format!("已发送复制请求，共 {line_count} 行。"));
                }
                Err(error) => {
                    app.set_toast(&format!("复制失败：{error}"));
                }
            }
        }
        UiAction::SaveSnippet { block_index } => {
            app.set_toast(&format!(
                "保存片段将在 artifact store 接入后启用：#{block_index}。"
            ));
        }
        UiAction::ExpandBlock { block_index } => {
            app.set_toast(&format!(
                "展开详情将在富内容检查器接入后启用：#{block_index}。"
            ));
        }
        UiAction::PreviewMedia { block_index } => {
            app.set_toast(&format!(
                "媒体预览将在 artifact preview 接入后启用：#{block_index}。"
            ));
        }
        UiAction::OpenMedia { block_index } => {
            app.set_toast(&format!(
                "外部打开需经过 approval / sandbox：#{block_index}。"
            ));
        }
    }
    Ok(())
}

struct TerminalSession;

impl TerminalSession {
    fn enable_mouse() -> io::Result<Self> {
        if let Err(error) = execute!(io::stdout(), EnableMouseCapture) {
            let _ = ratatui::try_restore();
            return Err(error);
        }
        Ok(Self)
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let _ = execute!(io::stdout(), DisableMouseCapture);
        let _ = ratatui::try_restore();
    }
}
