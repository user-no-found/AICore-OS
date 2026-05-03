use std::io::{self, Write};

use crate::render::{append_local_echo, render_snapshot};
use crate::state::TuiModel;

pub fn run_interactive(mut model: TuiModel) -> i32 {
    redraw(&model);
    loop {
        print!("aicore > ");
        if io::stdout().flush().is_err() {
            eprintln!("无法刷新终端输出。");
            return 1;
        }

        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(0) => return 0,
            Ok(_) => {
                let input = input.trim();
                if input.eq_ignore_ascii_case("q") || input.eq_ignore_ascii_case("quit") {
                    println!("已退出 AICore TUI。");
                    return 0;
                }
                if input.is_empty() {
                    continue;
                }
                append_local_echo(&mut model, input);
                redraw(&model);
            }
            Err(error) => {
                eprintln!("无法读取输入：{error}");
                return 1;
            }
        }
    }
}

fn redraw(model: &TuiModel) {
    print!("\x1b[2J\x1b[H{}", render_snapshot(model));
}
