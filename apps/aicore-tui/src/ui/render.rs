use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::widgets::{Block, Clear};

use crate::ui::app_state::AicoreTuiApp;
use crate::ui::conversation::render_conversation;
use crate::ui::layout::UiAreas;
use crate::ui::theme::surface_style;
use crate::ui::widgets::{
    render_action_bar, render_composer, render_left_pane, render_right_pane, render_top_bar,
};

pub fn draw(frame: &mut Frame, app: &mut AicoreTuiApp) {
    app.clear_hit_rects();
    let areas = UiAreas::new(frame.area());
    render_background(frame, frame.area());
    render_top_bar(frame, app, areas.top);
    render_left_pane(frame, app, areas.left);
    render_conversation(frame, app, areas.center);
    render_right_pane(frame, app, areas.right);
    render_composer(frame, app, areas.composer);
    render_action_bar(frame, app, areas.action);
}

fn render_background(frame: &mut Frame, area: Rect) {
    frame.render_widget(Clear, area);
    frame.render_widget(Block::default().style(surface_style()), area);
}
