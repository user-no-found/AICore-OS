mod app;
mod interaction;
mod render;
mod state;

pub use app::run;
pub use render::{append_local_echo, render_snapshot, render_transcript};
pub use state::{TuiBlock, TuiBlockKind, TuiModel, build_tui_model};
