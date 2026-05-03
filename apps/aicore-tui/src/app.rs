use std::io::{self, IsTerminal};
use std::path::PathBuf;

use crate::render::render_snapshot;
use crate::state::build_tui_model;
use crate::terminal::run_terminal;

pub fn run() -> i32 {
    let cwd = match std::env::current_dir() {
        Ok(cwd) => cwd,
        Err(error) => {
            eprintln!("无法读取当前目录：{error}");
            return 1;
        }
    };
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| cwd.clone());
    let model = match build_tui_model(&cwd, &home) {
        Ok(model) => model,
        Err(error) => {
            eprintln!("无法绑定当前实例：{error}");
            return 1;
        }
    };

    if io::stdin().is_terminal() && io::stdout().is_terminal() {
        return run_terminal(model);
    }

    print!("{}", render_snapshot(&model));
    0
}
