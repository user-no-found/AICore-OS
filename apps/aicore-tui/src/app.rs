use std::path::PathBuf;

use crate::state::build_launch_context;
use crate::warp_launcher::run_warp_tui;

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
    let context = match build_launch_context(&cwd, &home) {
        Ok(context) => context,
        Err(error) => {
            eprintln!("无法绑定当前实例：{error}");
            return 1;
        }
    };

    run_warp_tui(&context)
}
