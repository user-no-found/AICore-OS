use ratatui::layout::{Position, Rect};

use crate::state::TuiModel;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiAction {
    CopyBlock { block_index: usize },
    SaveSnippet { block_index: usize },
    ExpandBlock { block_index: usize },
    PreviewMedia { block_index: usize },
    OpenMedia { block_index: usize },
}

#[derive(Debug, Clone)]
pub struct HitRect {
    pub area: Rect,
    pub action: UiAction,
}

pub struct AicoreTuiApp {
    pub model: TuiModel,
    pub composer: String,
    pub should_quit: bool,
    pub toast: String,
    pub scroll: u16,
    pub copied_block: Option<usize>,
    pub hit_rects: Vec<HitRect>,
    pub focus_index: usize,
}

impl AicoreTuiApp {
    pub fn new(model: TuiModel) -> Self {
        Self {
            model,
            composer: String::new(),
            should_quit: false,
            toast: "就绪：TUI 仅显示和输入，不启动智能体运行时。".to_string(),
            scroll: 0,
            copied_block: None,
            hit_rects: Vec::new(),
            focus_index: 0,
        }
    }

    pub fn action_at(&self, position: Position) -> Option<&UiAction> {
        self.hit_rects
            .iter()
            .find(|hit| hit.area.contains(position))
            .map(|hit| &hit.action)
    }

    pub fn clear_hit_rects(&mut self) {
        self.hit_rects.clear();
    }

    pub fn set_toast(&mut self, message: &str) {
        self.toast = message.to_string();
    }

    pub fn mark_copied(&mut self, block_index: usize) {
        self.copied_block = Some(block_index);
    }

    pub fn next_focus(&mut self) {
        self.focus_index = (self.focus_index + 1) % 4;
        self.toast = format!("焦点已切换到 {}。", self.focus_label());
    }

    pub fn scroll_up(&mut self) {
        self.scroll = self.scroll.saturating_sub(3);
    }

    pub fn scroll_down(&mut self) {
        self.scroll = self.scroll.saturating_add(3);
    }

    pub fn push_hit(&mut self, area: Rect, action: UiAction) {
        if !area.is_empty() {
            self.hit_rects.push(HitRect { area, action });
        }
    }

    fn focus_label(&self) -> &'static str {
        match self.focus_index {
            0 => "消息流",
            1 => "实例面板",
            2 => "运行面板",
            _ => "输入栏",
        }
    }
}
