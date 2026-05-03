mod app;
mod cli;
mod fpk;
mod http;
mod status;

use std::process::ExitCode;

fn main() -> ExitCode {
    match app::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("aicore-web 启动失败：{error}");
            ExitCode::from(1)
        }
    }
}
