mod app;
mod render;
mod state;
mod terminal;
mod ui;

pub use app::run;
pub use render::{append_local_echo, render_snapshot, render_transcript};
pub use state::{TuiBlock, TuiBlockKind, TuiModel, build_tui_model};
